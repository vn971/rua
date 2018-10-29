// Commands that are run inside "bubblewrap" jail

use aur;
use directories::ProjectDirs;
use itertools::Itertools;
use pacman;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use tar_check;


const CHECKED_TARS: &str = "checked_tars";

fn wrap_yes_internet(dirs: &ProjectDirs) -> Command {
	Command::new(dirs.cache_dir().join(".rua/wrap.sh"))
}

fn wrap_no_internet(dirs: &ProjectDirs) -> Command {
	let mut command = Command::new(dirs.cache_dir().join(".rua/wrap.sh"));
	command.arg("--unshare-net");
	command
}

pub fn get_deps(dir: &str, dirs: &ProjectDirs) -> Vec<String> {
	env::set_current_dir(dir).unwrap();
	let command = wrap_no_internet(dirs)
		.args(&["bash", "--restricted", dirs.cache_dir().join(".rua/get_deps.sh").to_str().unwrap()])
		.stderr(Stdio::inherit()).output().unwrap();
	String::from_utf8_lossy(&command.stdout).split(' ')
		.map(|s| s.trim().to_string())
		.filter(|s| !s.is_empty()).collect()
}

fn download_sources(dirs: &ProjectDirs) {
	let dir = env::current_dir().unwrap();
	let dir = dir.to_str().unwrap();
	let command = wrap_yes_internet(dirs)
		.args(&["--bind", dir, dir])
		.args(&["makepkg", "--noprepare", "--nobuild"])
		.status().unwrap();
	assert!(command.success(), "Failed to download PKGBUILD sources");
}


fn build_offline(dirs: &ProjectDirs) {
	let dir = env::current_dir().unwrap();
	let dir = dir.to_str().unwrap();
	let command = wrap_no_internet(dirs)
		.args(&["--bind", dir, dir])
		.args(&["makepkg"]).status().unwrap();
	assert!(command.success(), "Failed to build package");
}

pub fn build_directory(dir: &str, project_dirs: &ProjectDirs) {
	if Path::new(dir).join("target").exists() == false {
		env::set_current_dir(dir).expect(format!("cannot build in directory {}", dir).as_str());
		env::set_var("PKGDEST", Path::new(".").canonicalize().unwrap().join("target"));
		download_sources(project_dirs);
		build_offline(project_dirs);
	}
}

fn package_tar_review(name: &str, dirs: &ProjectDirs) {
	if dirs.cache_dir().join(name).join(CHECKED_TARS).exists() {
		return;
	}
	let expect = format!("target directory not found for package {}: {:?}", name,
		dirs.cache_dir().join(name).join("build/target"));
	for file in fs::read_dir(dirs.cache_dir().join(name).join("build/target")).expect(&expect) {
		tar_check::tar_check(file.unwrap().path());
	}
	fs::rename(
		dirs.cache_dir().join(name).join("build/target"),
		dirs.cache_dir().join(name).join(CHECKED_TARS),
	).unwrap();
}


fn prefetch_aur(name: &str, dirs: &ProjectDirs,
	pacman_deps: &mut HashSet<String>,
	aur_deps: &mut HashMap<String, i32>,
	depth: i32,
) {
	if aur_deps.contains_key(name) {
		return;
	}
	aur_deps.insert(name.to_string(), depth);
	aur::download_if_absent(&name, &dirs);
	let deps = get_deps(dirs.cache_dir().join(name).join("build").to_str().unwrap(), &dirs);
	debug!("package {} has dependencies: {:?}", name, &deps);
	for dep in &deps {
		if !pacman::is_package_installable(&dep) {
			eprintln!("{} depends on AUR package {}. Trying to fetch it...", name, &dep);
			prefetch_aur(&dep, dirs, pacman_deps, aur_deps, depth + 1);
		} else if !pacman::is_package_installed(&dep) {
			pacman_deps.insert(dep.to_string());
		}
	}
}


fn install_all(dirs: &ProjectDirs, packages: HashMap<String, i32>) {
	let mut packages = packages.iter().collect::<Vec<_>>();
	packages.sort_unstable_by_key(|pair| -*pair.1);
	for (_, packages) in &packages.iter().group_by(|pair| *pair.1) {
		let packages: Vec<_> = packages.into_iter().map(|pair| pair.0).collect();
		for name in &packages {
			build_directory(dirs.cache_dir().join(&name).join("build").to_str().unwrap(), dirs);
		}
		for name in &packages {
			package_tar_review(name, dirs);
		}
		let mut packages_to_install: HashMap<String, PathBuf> = HashMap::new();
		for name in packages {
			for file in fs::read_dir(dirs.cache_dir().join(name).join(CHECKED_TARS)).unwrap() {
				packages_to_install.insert(name.to_string(), file.unwrap().path());
			}
		}
		pacman::ensure_aur_packages_installed(packages_to_install);
	}
}

pub fn install(name: &str, dirs: &ProjectDirs) {
	let mut pacman_deps = HashSet::new();
	let mut aur_deps = HashMap::new();
	prefetch_aur(name, dirs, &mut pacman_deps, &mut aur_deps, 0);
	pacman::ensure_pacman_packages_installed(pacman_deps);
	install_all(dirs, aur_deps);
}
