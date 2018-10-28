// Commands that are run inside "bubblewrap" jail

use directories::ProjectDirs;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use tar_check;
use pacman;
use aur;


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
	let current_dir = env::current_dir().unwrap();
	let current_dir = current_dir.to_str().unwrap();
	let command = wrap_yes_internet(dirs)
		.args(&["--bind", current_dir, current_dir])
		.args(&["makepkg", "--noprepare", "--nobuild"])
		.status().unwrap();
	assert!(command.success(), "Failed to download PKGBUILD sources");
}


fn do_build(dirs: &ProjectDirs) {
	let dir = env::current_dir().unwrap();
	let command = wrap_no_internet(dirs)
		.args(&["--bind", dir.to_str().unwrap(), dir.to_str().unwrap()])
		.args(&["makepkg"]).status().unwrap();
	assert!(command.success(), "Failed to build package");

	for file in fs::read_dir("target").unwrap() {
		tar_check::tar_check(file.unwrap().path());
	}
}

pub fn jail_build(dir: &str, project_dirs: &ProjectDirs) {
	env::set_current_dir(dir).expect(format!("cannot build in directory {}", dir).as_str());
	env::set_var("PKGDEST", Path::new(".").canonicalize().unwrap().join("target"));
	download_sources(project_dirs);
	do_build(project_dirs);
}


fn prefetch_aur(target: &str, dirs: &ProjectDirs,
	package_ii: &mut HashMap<String, (bool, bool)>,
	pacman_deps: &mut HashSet<String>,
	aur_deps: &mut HashMap<String, bool>,
) {
	aur::download_if_absent(&target, &dirs);
	let deps = get_deps(dirs.cache_dir().join(target).join("build").to_str().unwrap(), &dirs);
	aur_deps.insert(target.to_string(), !deps.is_empty());
	debug!("package {} has dependencies: {:?}", target, &deps);
	for dep in deps {
		let ii = pacman::is_package_installed_installable(dep.as_str());
		package_ii.insert(dep.to_string(), ii);
		trace!("dependency {}, installed={}, pacman-installable: {}", &dep, ii.0, ii.1);
		if ii == (false, true) {
			pacman_deps.insert(dep.to_string());
		} else if ii == (false, false) {
			eprintln!("{} depends on AUR package {}. Trying to fetch it...", target, &dep);
			prefetch_aur(&dep, dirs, package_ii, pacman_deps, aur_deps);
		}
	}
}


pub fn install(target: &str, dirs: &ProjectDirs) {
	let mut package_ii: HashMap<String, (bool, bool)> = HashMap::new();
	let mut pacman_deps = HashSet::new();
	let mut aur_deps = HashMap::new();
	prefetch_aur(target, dirs, &mut package_ii, &mut pacman_deps, &mut aur_deps);
	pacman::ensure_pacman_packages_installed(&mut pacman_deps);
	for (target, _) in aur_deps {
		// TODO: group in independent branches
		jail_build(dirs.cache_dir().join(target).join("build").to_str().unwrap(), &dirs);
	}
}
