use crate::rua_dirs::CHECKED_TARS;
use crate::rua_dirs::REVIEWED_BUILD_DIR;
use crate::rua_dirs::TARGET_SUBDIR;
use crate::tar_check;
use crate::{aur_download, wrapped};
use crate::{pacman, terminal_util};

use directories::ProjectDirs;
use itertools::Itertools;
use std::fs;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub fn install(targets: Vec<String>, dirs: &ProjectDirs, is_offline: bool, asdeps: bool) {
	let mut pacman_deps = HashSet::new();
	let mut aur_packages = HashMap::new();
	let alpm = pacman::create_alpm();
	for install_target in targets {
		wrapped::prefetch_aur(
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
			wrapped::build_directory(
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
		let mut packages_to_install: Vec<(String, PathBuf)> = Vec::new();
		for name in packages {
			let checked_tars = dirs.cache_dir().join(name).join(CHECKED_TARS);
			let read_dir_iterator = fs::read_dir(checked_tars).unwrap_or_else(|e| {
				panic!(
					"Failed to read 'checked_tars' directory for {}, {}",
					name, e
				)
			});
			for file in read_dir_iterator {
				packages_to_install.push((
					name.to_owned(),
					file.expect("Failed to open file for tar_check analysis")
						.path(),
				));
			}
		}
		pacman::ensure_aur_packages_installed(packages_to_install, asdeps || depth > 0);
	}
}

pub fn check_tars_and_move(name: &str, dirs: &ProjectDirs) {
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
