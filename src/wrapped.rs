// Commands that are run inside "bubblewrap" jail

use crate::aur_download;
use crate::pacman;
use crate::pacman::PACMAN_ARCH;
use crate::rua_dirs::PREFETCH_DIR;
use crate::rua_dirs::TARGET_SUBDIR;

use directories::ProjectDirs;
use libalpm::Alpm;
use log::debug;
use srcinfo::Srcinfo;

use std::cmp;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;
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

pub fn prefetch_aur(
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
