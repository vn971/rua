// Commands that are run inside "bubblewrap" jail

use crate::aur_download::PREFETCH_DIR;
use crate::aur_download;
use directories::ProjectDirs;
use itertools::Itertools;
use crate::libalpm::Alpm;
use crate::libalpm::SigLevel;
use crate::pacman;
use crate::srcinfo;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use crate::tar_check;
use uname;
use crate::util;


const CHECKED_TARS: &str = "checked_tars";
pub const WRAP_SCRIPT_PATH: &str = ".system/wrap.sh";

fn wrap_yes_internet(dirs: &ProjectDirs) -> Command {
	Command::new(dirs.config_dir().join(WRAP_SCRIPT_PATH))
}


fn download_srcinfo_sources(dirs: &ProjectDirs) {
	let dir = env::current_dir().unwrap().canonicalize().unwrap();
	let dir = dir.to_str().unwrap();
	let mut file = File::create("PKGBUILD.static")
		.expect("cannot create temporary PKGBUILD.static file");
	let srcinfo_path = Path::new(".SRCINFO").canonicalize()
		.expect(&format!("Cannot resolve .SRCINFO path in {}", dir));
	file.write_all(srcinfo::static_pkgbuild(srcinfo_path).as_bytes())
		.expect("cannot write to PKGBUILD.static");
	eprintln!("Downloading sources using .SRCINFO... (integrity tests will be done when building)");
	let command = wrap_yes_internet(dirs)
		.args(&["--bind", dir, dir])
		.args(&["makepkg", "--verifysource", "--skipinteg"])
		.args(&["-p", "PKGBUILD.static"])
		.status().expect(&format!("Failed to fetch dependencies in directory {}", dir));
	assert!(command.success(), "Failed to download PKGBUILD sources");
	fs::remove_file("PKGBUILD.static").expect("Failed to clean up PKGBUILD.static");
}


fn build_local(dirs: &ProjectDirs, is_offline: bool) {
	let dir = env::current_dir()
		.expect(&format!("{}:{} Failed to get current dir", file!(), line!()));
	let dir = dir.to_str().unwrap();
	let mut command = wrap_yes_internet(dirs);
	if is_offline { command.arg("--unshare-net"); }
	command.args(&["--bind", dir, dir]);
	let command = command.args(&["makepkg"]).status()
		.expect(&format!("Failed to build package (jailed makepkg) in directory {}", dir));
	assert!(command.success(), "Failed to build package");
}

pub fn build_directory(dir: &str, project_dirs: &ProjectDirs, offline: bool, lazy: bool) {
	env::set_current_dir(dir).expect(format!("cannot build in directory {}", dir).as_str());
	if Path::new(dir).join("target").exists() && lazy {
		eprintln!("Skipping build for {} as 'target' directory is already present.", dir);
	} else {
		env::set_var("PKGDEST", Path::new(".").canonicalize()
			.expect(&format!("Failed to canonize target directory {}", dir)).join("target"));
		if offline {
			download_srcinfo_sources(project_dirs);
		}
		build_local(project_dirs, offline);
	}
}

fn package_tar_review(name: &str, dirs: &ProjectDirs) {
	if dirs.cache_dir().join(name).join(CHECKED_TARS).exists() {
		eprintln!("Skipping *.tar verification for package {} as it already has been verified before.", name);
		return;
	}
	let expect = format!("target directory not found for package {}: {:?}", name,
		dirs.cache_dir().join(name).join("build/target"));
	for file in fs::read_dir(dirs.cache_dir().join(name).join("build/target")).expect(&expect) {
		tar_check::tar_check(file.expect("Failed to open file for tar_check analysis").path());
	}
	fs::rename(
		dirs.cache_dir().join(name).join("build/target"),
		dirs.cache_dir().join(name).join(CHECKED_TARS),
	).expect(&format!("Failed to move 'build/target' (build artefacts) \
		to 'checked_tars' directory for package {}", name));
}

lazy_static! {
	static ref uname_arch: String = uname::uname()
		.expect("Failed to get system architecture via uname").machine;
}

fn prefetch_aur(name: &str, dirs: &ProjectDirs,
	pacman_deps: &mut HashSet<String>,
	aur_packages: &mut HashMap<String, i32>,
	depth: i32,
	alpm: &Alpm,
) {
	if aur_packages.contains_key(name) {
		eprintln!("Skipping already fetched package {}", name);
		return;
	}
	aur_packages.insert(name.to_owned(), depth);
	aur_download::fresh_download(&name, &dirs);
	let info = dirs.cache_dir().join(name).join(PREFETCH_DIR).join(".SRCINFO");
	let info = srcinfo::FlatSrcinfo::new(info);
	let deps: Vec<&String> = info.get("depends").iter()
		.merge(info.get("makedepends"))
		.merge(info.get(&format!("depends_{}", uname_arch.as_str())))
		.merge(info.get(&format!("makedepends_{}", uname_arch.as_str())))
		.collect();
	debug!("package {} has dependencies: {:?}", name, &deps);
	for dep in deps.into_iter() {
		if pacman::is_package_installed(alpm, &dep) {
		} else if !pacman::is_package_installable(alpm, &dep) {
			eprintln!("{} depends on AUR package {}. Trying to fetch it...", name, &dep);
			prefetch_aur(&dep, dirs, pacman_deps, aur_packages, depth + 1, alpm);
		} else {
			pacman_deps.insert(dep.to_owned());
		}
	}
}


fn show_install_summary(name: &str, pacman_deps: &HashSet<String>, aur_packages: &HashMap<String, i32>) {
	if pacman_deps.len() + aur_packages.len() == 1 { return; }
	eprintln!("\nIn order to install {}, the following pacman packages will need to be installed:", name);
	eprintln!("{}", pacman_deps.iter().map(|s| format!("  {}", s)).join("\n"));
	eprintln!("And the following AUR packages will need to be built and installed:");
	eprintln!("{}\n", aur_packages.keys().map(|s| format!("  {}", s)).join("\n"));
	loop {
		eprint!("Proceed? [O]=ok, Ctrl-C=abort. ");
		let string = util::console_get_line();
		if string == "o" {
			break;
		}
	}
}

fn install_all(dirs: &ProjectDirs, packages: HashMap<String, i32>, offline: bool, alpm: &Alpm) {
	let mut packages = packages.iter().collect::<Vec<_>>();
	packages.sort_unstable_by_key(|pair| -*pair.1);
	for (depth, packages) in &packages.iter().group_by(|pair| *pair.1) {
		let packages: Vec<_> = packages.into_iter().map(|pair| pair.0).collect();
		for name in &packages {
			build_directory(dirs.cache_dir().join(&name).join("build").to_str()
				.expect(&format!("{}:{} Failed to resolve build path for {}", file!(), line!(), name)),
				dirs, offline, true);
		}
		for name in &packages {
			package_tar_review(name, dirs);
		}
		let mut packages_to_install: HashMap<String, PathBuf> = HashMap::new();
		for name in packages {
			let checked_tars = dirs.cache_dir().join(name).join(CHECKED_TARS);
			let read_dir_iterator = fs::read_dir(checked_tars)
				.expect(&format!("Failed to read 'checked_tars' directory for {}", name));
			for file in read_dir_iterator {
				packages_to_install.insert(
					name.to_owned(),
					file.expect("Failed to open file for tar_check analysis").path()
				);
			}
		}
		pacman::ensure_aur_packages_installed(packages_to_install, depth > 0, alpm);
	}
}

pub fn install(name: &str, dirs: &ProjectDirs, is_offline: bool) {
	let mut pacman_deps = HashSet::new();
	let mut aur_packages = HashMap::new();
	let alpm = Alpm::new("/", "/var/lib/pacman"); // default locations on arch linux
	let alpm = alpm.expect("Failed to initialize alpm library");
	for repo in pacman::get_repository_list() {
		alpm.register_sync_db(&repo, &SigLevel::default()).expect(&format!("Failed to register {} in libalpm", &repo));
	}
	prefetch_aur(name, dirs, &mut pacman_deps, &mut aur_packages, 0, &alpm);
	pacman_deps.retain(|name| !pacman::is_package_installed(&alpm, name));
	show_install_summary(name, &pacman_deps, &aur_packages);
	for (name, _) in &aur_packages {
		aur_download::review_repo(name, dirs);
	}
	pacman::ensure_pacman_packages_installed(pacman_deps, &alpm);
	install_all(dirs, aur_packages, is_offline, &alpm);
}
