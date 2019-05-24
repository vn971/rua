// Commands that are run inside "bubblewrap" jail

use crate::aur_download;
use crate::rua_dirs::CHECKED_TARS;
use crate::rua_dirs::PREFETCH_DIR;
use crate::rua_dirs::REVIEWED_BUILD_DIR;
use crate::rua_dirs::TARGET_SUBDIR;
use crate::{pacman, tar_check, terminal_util};

use directories::ProjectDirs;
use itertools::Itertools;
use lazy_static::lazy_static;
use libalpm::{Alpm, SigLevel};
use log::debug;
use srcinfo::Srcinfo;

use std::cmp;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;
use std::{env, fs};

pub const WRAP_SCRIPT_PATH: &str = ".system/wrap.sh";

fn wrap_yes_internet(dirs: &ProjectDirs) -> Command {
	Command::new(dirs.config_dir().join(WRAP_SCRIPT_PATH))
}

fn download_srcinfo_sources(dirs: &ProjectDirs) {
	let dir = env::current_dir().unwrap().canonicalize().unwrap();
	let dir = dir.to_str().unwrap();
	let mut file = File::create("PKGBUILD.static")
		.unwrap_or_else(|err| panic!("Cannot create {}/PKGBUILD.static, {}", dir, err));
	let srcinfo_path = Path::new(".SRCINFO")
		.canonicalize()
		.unwrap_or_else(|e| panic!("Cannot resolve .SRCINFO path in {}, {}", dir, e));
	file.write_all(crate::srcinfo_to_pkgbuild::static_pkgbuild(&srcinfo_path).as_bytes())
		.expect("cannot write to PKGBUILD.static");
	eprintln!("Downloading sources using .SRCINFO...");
	let command = wrap_yes_internet(dirs)
		.args(&["--bind", dir, dir])
		.args(&["makepkg", "-f", "--verifysource"])
		.args(&["-p", "PKGBUILD.static"])
		.status()
		.unwrap_or_else(|e| panic!("Failed to fetch dependencies in directory {}, {}", dir, e));
	assert!(command.success(), "Failed to download PKGBUILD sources");
	fs::remove_file("PKGBUILD.static").expect("Failed to clean up PKGBUILD.static");
}

fn build_local(dirs: &ProjectDirs, is_offline: bool) {
	let dir = env::current_dir()
		.unwrap_or_else(|e| panic!("{}:{} Failed to get current dir, {}", file!(), line!(), e));
	let dir = dir.to_str().unwrap();
	let mut command = wrap_yes_internet(dirs);
	if is_offline {
		command.arg("--unshare-net");
	}
	command.args(&["--bind", dir, dir]);
	let command = command.args(&["makepkg"]).status().unwrap_or_else(|e| {
		panic!(
			"Failed to build package (jailed makepkg) in directory {}, {}",
			dir, e,
		)
	});
	if !command.success() {
		eprintln!(
			"Build failed with exit code {} in {}",
			command
				.code()
				.map_or_else(|| "???".to_owned(), |c| c.to_string()),
			dir,
		);
		std::process::exit(command.code().unwrap_or(1));
	}
}

pub fn build_directory(dir: &str, project_dirs: &ProjectDirs, offline: bool) {
	env::set_current_dir(dir)
		.unwrap_or_else(|e| panic!("cannot change the current directory to {}, {}", dir, e));
	env::set_var(
		"PKGDEST",
		Path::new(".")
			.canonicalize()
			.unwrap_or_else(|e| panic!("Failed to canonize target directory {}, {}", dir, e))
			.join(TARGET_SUBDIR),
	);
	if offline {
		download_srcinfo_sources(project_dirs);
	}
	build_local(project_dirs, offline);
}

fn check_tars_and_move(name: &str, dirs: &ProjectDirs) {
	let build_target_dir = dirs
		.cache_dir()
		.join(name)
		.join(REVIEWED_BUILD_DIR)
		.join(TARGET_SUBDIR);
	let checked_tars_dir = dirs.cache_dir().join(name).join(CHECKED_TARS);
	rm_rf::force_remove_all(&checked_tars_dir, true).unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to clean checked tar files dir {:?}, {}",
			file!(),
			line!(),
			CHECKED_TARS,
			err,
		)
	});
	let target_dir = fs::read_dir(&build_target_dir);
	let target_dir = target_dir.unwrap_or_else(|err| {
		panic!(
			"target directory not found for package {}: {:?}. \
			 \nDoes the PKGBUILD respect the environment variable PKGDEST ?\
			 \n{}",
			name, &build_target_dir, err,
		)
	});
	for file in target_dir {
		tar_check::tar_check(
			&file
				.expect("Failed to open file for tar_check analysis")
				.path(),
		);
	}
	fs::rename(&build_target_dir, &checked_tars_dir).unwrap_or_else(|e| {
		panic!(
			"Failed to move {:?} (build artifacts) to {:?} for package {}, {}",
			&build_target_dir, &checked_tars_dir, name, e,
		)
	});
}

lazy_static! {
	static ref PACMAN_ARCH: String = libalpm::util::uname().machine().to_owned();
}

fn prefetch_aur(
	name: &str,
	dirs: &ProjectDirs,
	pacman_deps: &mut HashSet<String>,
	aur_packages: &mut HashMap<String, i32>,
	depth: i32,
	alpm: &Alpm,
) {
	if let Some(old_depth) = aur_packages.get(name) {
		let old_depth = *old_depth;
		aur_packages.insert(name.to_owned(), cmp::max(depth + 1, old_depth));
		eprintln!("Skipping already fetched package {}", name);
		return;
	}
	aur_packages.insert(name.to_owned(), depth);
	aur_download::fresh_download(&name, &dirs);
	let srcinfo_path = dirs
		.cache_dir()
		.join(name)
		.join(PREFETCH_DIR)
		.join(".SRCINFO");
	let info = Srcinfo::parse_file(&srcinfo_path).unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to parse {:?}, {}",
			file!(),
			line!(),
			srcinfo_path,
			err,
		)
	});
	let deps = info
		.pkg(name)
		.unwrap_or_else(|| {
			panic!(
				"{}:{} pkgname {} not found in {:?}",
				file!(),
				line!(),
				name,
				&srcinfo_path
			)
		})
		.depends
		.iter()
		.chain(&info.base.makedepends)
		.chain(&info.base.checkdepends)
		.filter(|deps_vector| deps_vector.supports(PACMAN_ARCH.as_str()))
		.flat_map(|deps_vector| &deps_vector.vec)
		.collect::<Vec<_>>();
	debug!("package {} has dependencies: {:?}", name, &deps);
	for dep in deps.into_iter() {
		if pacman::is_package_installed(alpm, &dep) {
		} else if !pacman::is_package_installable(alpm, &dep) {
			eprintln!(
				"{} depends on AUR package {}. Trying to fetch it...",
				name, &dep
			);
			prefetch_aur(&dep, dirs, pacman_deps, aur_packages, depth + 1, alpm);
		} else {
			pacman_deps.insert(dep.to_owned());
		}
	}
}

fn show_install_summary(pacman_deps: &HashSet<String>, aur_packages: &HashMap<String, i32>) {
	if pacman_deps.len() + aur_packages.len() == 1 {
		return;
	}
	eprintln!("\nIn order to install all targets, the following pacman packages will need to be installed:");
	eprintln!(
		"{}",
		pacman_deps.iter().map(|s| format!("  {}", s)).join("\n")
	);
	eprintln!("And the following AUR packages will need to be built and installed:");
	let mut aur_packages = aur_packages.iter().collect::<Vec<_>>();
	aur_packages.sort_by_key(|pair| -*pair.1);
	eprintln!(
		"{}\n",
		aur_packages.iter().map(|s| format!("  {}", s.0)).join("\n")
	);
	loop {
		eprint!("Proceed? [O]=ok, Ctrl-C=abort. ");
		let string = terminal_util::console_get_line();
		if string == "o" {
			break;
		}
	}
}

fn install_all(dirs: &ProjectDirs, packages: HashMap<String, i32>, offline: bool, asdeps: bool) {
	let mut packages = packages.iter().collect::<Vec<_>>();
	packages.sort_by_key(|pair| -*pair.1);
	for (depth, packages) in &packages.iter().group_by(|pair| *pair.1) {
		let packages: Vec<_> = packages.map(|pair| pair.0).collect();
		for name in &packages {
			build_directory(
				dirs.cache_dir()
					.join(&name)
					.join(REVIEWED_BUILD_DIR)
					.to_str()
					.unwrap_or_else(|| {
						panic!(
							"{}:{} Failed to resolve build path for {}",
							file!(),
							line!(),
							name
						)
					}),
				dirs,
				offline,
			);
		}
		for name in &packages {
			check_tars_and_move(name, dirs);
		}
		let mut packages_to_install: HashMap<String, PathBuf> = HashMap::new();
		for name in packages {
			let checked_tars = dirs.cache_dir().join(name).join(CHECKED_TARS);
			let read_dir_iterator = fs::read_dir(checked_tars).unwrap_or_else(|e| {
				panic!(
					"Failed to read 'checked_tars' directory for {}, {}",
					name, e
				)
			});
			for file in read_dir_iterator {
				packages_to_install.insert(
					name.to_owned(),
					file.expect("Failed to open file for tar_check analysis")
						.path(),
				);
			}
		}
		pacman::ensure_aur_packages_installed(packages_to_install, asdeps || depth > 0);
	}
}

pub fn install(targets: Vec<String>, dirs: &ProjectDirs, is_offline: bool, asdeps: bool) {
	let mut pacman_deps = HashSet::new();
	let mut aur_packages = HashMap::new();
	let alpm = Alpm::new("/", "/var/lib/pacman"); // default locations on arch linux
	let alpm = alpm.unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to initialize alpm library, {}",
			file!(),
			line!(),
			err
		)
	});
	for repo in pacman::get_repository_list() {
		alpm.register_sync_db(&repo, &SigLevel::default())
			.unwrap_or_else(|e| panic!("Failed to register {} in libalpm, {}", &repo, e));
	}
	for install_target in targets {
		prefetch_aur(
			&install_target,
			dirs,
			&mut pacman_deps,
			&mut aur_packages,
			0,
			&alpm,
		);
	}
	pacman_deps.retain(|name| !pacman::is_package_installed(&alpm, name));
	show_install_summary(&pacman_deps, &aur_packages);
	for name in aur_packages.keys() {
		aur_download::review_repo(name, dirs);
	}
	pacman::ensure_pacman_packages_installed(pacman_deps);
	install_all(dirs, aur_packages, is_offline, asdeps);
}
