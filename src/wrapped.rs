// Commands that are run inside "bubblewrap" jail

use aur::PREFETCH_DIR;
use aur;
use directories::ProjectDirs;
use itertools::Itertools;
use libalpm_fork as libalpm;
use pacman;
use srcinfo;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::fs;
use std::io::Write;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tar_check;


const CHECKED_TARS: &str = "checked_tars";
pub const WRAP_SCRIPT_PATH: &str = ".system/wrap.sh";

fn wrap_yes_internet(dirs: &ProjectDirs) -> Command {
	Command::new(dirs.config_dir().join(WRAP_SCRIPT_PATH))
}


fn download_srcinfo_sources(dirs: &ProjectDirs) {
	let dir = env::current_dir().unwrap().canonicalize().unwrap();
	let dir = dir.to_str().unwrap();
	let mut file = File::create("PKGBUILD.static").expect("cannot create temporary PKGBUILD.static file");
	file.write_all(srcinfo::static_pkgbuild(".SRCINFO").as_bytes()).expect("cannot write to PKGBUILD.static");
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
	let dir = env::current_dir().unwrap();
	let dir = dir.to_str().unwrap();
	let mut command = wrap_yes_internet(dirs);
	if is_offline { command.arg("--unshare-net"); }
	command.args(&["--bind", dir, dir]);
	let command = command.args(&["makepkg"]).status()
		.expect(&format!("Failed to build package (jailed makepkg) in directory {}", dir));
	assert!(command.success(), "Failed to build package");
}

pub fn build_directory(dir: &str, project_dirs: &ProjectDirs, is_offline: bool) {
	env::set_current_dir(dir).expect(format!("cannot build in directory {}", dir).as_str());
	if Path::new(dir).join("target").exists() {
		eprintln!("Skipping build for {} as 'target' directory is already present.", dir);
	} else {
		env::set_var("PKGDEST", Path::new(".").canonicalize()
			.expect(&format!("Failed to canonize target directory {}", dir)).join("target"));
		if is_offline {
			download_srcinfo_sources(project_dirs);
		}
		build_local(project_dirs, is_offline);
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
		tar_check::tar_check(file.expect(&format!("Failed to open file for tar_check analysis")).path());
	}
	fs::rename(
		dirs.cache_dir().join(name).join("build/target"),
		dirs.cache_dir().join(name).join(CHECKED_TARS),
	).expect(&format!("Failed to move 'build/target' (build artefacts) \
		to 'checked_tars' directory for package {}", name));
}


fn prefetch_aur(name: &str, dirs: &ProjectDirs,
	pacman_deps: &mut HashSet<String>,
	aur_deps: &mut HashMap<String, i32>,
	depth: i32,
) {
	if aur_deps.contains_key(name) {
		eprintln!("Skipping already fetched package {}", name);
		return;
	}
	aur_deps.insert(name.to_owned(), depth);
	aur::fresh_download(&name, &dirs);
	let info = srcinfo::FlatSrcinfo::new(dirs.cache_dir().join(name).join(PREFETCH_DIR).join(".SRCINFO"));
	let deps: Vec<&String> = info.get("depends").iter()
		.merge(info.get("makedepends"))
		.merge(info.get(&format!("depends_{}", libalpm::util::uname().machine())))
		.merge(info.get(&format!("makedepends_{}", libalpm::util::uname().machine())))
		.collect();
	debug!("package {} has dependencies: {:?}", name, &deps);
	for dep in deps.into_iter() {
		if pacman::is_package_installed(&dep) {
		} else if pacman::is_package_installable(&dep) {
			pacman_deps.insert(dep.to_owned());
		} else {
			eprintln!("{} depends on AUR package {}. Trying to fetch it...", name, &dep);
			prefetch_aur(&dep, dirs, pacman_deps, aur_deps, depth + 1);
		}
	}
}


fn install_all(dirs: &ProjectDirs, packages: HashMap<String, i32>, is_offline: bool) {
	let mut packages = packages.iter().collect::<Vec<_>>();
	packages.sort_unstable_by_key(|pair| -*pair.1);
	for (depth, packages) in &packages.iter().group_by(|pair| *pair.1) {
		let packages: Vec<_> = packages.into_iter().map(|pair| pair.0).collect();
		for name in &packages {
			build_directory(dirs.cache_dir().join(&name).join("build").to_str().unwrap(), dirs, is_offline);
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
					file.expect(&format!("Failed to open file for tar_check analysis")).path(),
				);
			}
		}
		pacman::ensure_aur_packages_installed(packages_to_install, depth > 0);
	}
}

fn show_install_summary(name: &str, pacman_deps: &HashSet<String>, aur_deps: &HashMap<String, i32>) {
	eprintln!("\nIn order to install {}, the following pacman packages will need to be installed:", name);
	eprint!("{}", pacman_deps.iter().map(|s| format!("  {}", s)).join("\n"));
	eprintln!("And the following AUR packages will need to be built and installed:");
	eprintln!("{}\n", aur_deps.keys().map(|s| format!("  {}", s)).join("\n"));
	loop {
		eprint!("Proceed? [O]=ok, Ctrl-C=abort. ");
		let mut string = String::new();
		io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
		let string = string.trim().to_lowercase();
		if string == "o" {
			break
		}
	}
}

pub fn install(name: &str, dirs: &ProjectDirs, is_offline: bool) {
	let mut pacman_deps = HashSet::new();
	let mut aur_deps = HashMap::new();
	prefetch_aur(name, dirs, &mut pacman_deps, &mut aur_deps, 0);
	show_install_summary(name, &pacman_deps, &aur_deps);
	for (name, _) in &aur_deps {
		aur::review_repo(name, dirs);
	}
	pacman::ensure_pacman_packages_installed(pacman_deps);
	install_all(dirs, aur_deps, is_offline);
}
