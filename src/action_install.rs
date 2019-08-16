use crate::{pacman, terminal_util};
use crate::{reviewing, wrapped};
use crate::{rua_files, tar_check};

use core::cmp;
use directories::ProjectDirs;
use fs_extra::dir::CopyOptions;
use itertools::Itertools;
use lazy_static::lazy_static;
use libalpm::Alpm;
use log::debug;
use log::info;
use log::trace;
use raur::Package;
use regex::Regex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::fs::ReadDir;
use std::path::PathBuf;

pub fn install(targets: Vec<String>, dirs: &ProjectDirs, is_offline: bool, asdeps: bool) {
	let mut pacman_deps = HashSet::new();
	let mut aur_packages = HashMap::new();
	let alpm = pacman::create_alpm();
	for install_target in targets {
		resolve_dependencies(
			&install_target,
			&mut pacman_deps,
			&mut aur_packages,
			0,
			&alpm,
		);
	}
	pacman_deps.retain(|name| !pacman::is_package_installed(&alpm, name));
	show_install_summary(&pacman_deps, &aur_packages);
	for name in aur_packages.keys() {
		let dir = rua_files::review_dir(dirs, name);
		fs::create_dir_all(&dir)
			.unwrap_or_else(|err| panic!("Failed to create repository dir for {}, {}", name, err));
		reviewing::review_repo(&dir, name, dirs);
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
		let string = terminal_util::read_line_lowercase();
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
			let review_dir = rua_files::review_dir(dirs, name);
			let build_dir = rua_files::build_dir(dirs, name);
			rm_rf::force_remove_all(&build_dir, true).expect("Failed to remove old build dir");
			std::fs::create_dir_all(&build_dir).expect("Failed to create build dir");
			fs_extra::copy_items(
				&vec![review_dir],
				rua_files::global_build_dir(dirs),
				&CopyOptions::new(),
			)
			.expect("failed to copy reviewed dir to build dir");
			rm_rf::force_remove_all(build_dir.join(".git"), true).expect("Failed to remove .git");
			wrapped::build_directory(
				&build_dir.to_str().expect("Non-UTF8 directory name"),
				dirs,
				offline,
			);
		}
		for name in &packages {
			check_tars_and_move(name, dirs);
		}
		let mut files_to_install: Vec<(String, PathBuf)> = Vec::new();
		for name in &packages {
			let checked_tars = rua_files::checked_tars_dir(dirs, &name);
			let read_dir_iterator = fs::read_dir(checked_tars).unwrap_or_else(|e| {
				panic!(
					"Failed to read 'checked_tars' directory for {}, {}",
					name, e
				)
			});
			for file in read_dir_iterator {
				files_to_install.push((
					name.to_string(),
					file.expect("Failed to open file for tar_check analysis")
						.path(),
				));
			}
		}
		pacman::ensure_aur_packages_installed(files_to_install, asdeps || depth > 0);
	}
}

pub fn check_tars_and_move(name: &str, dirs: &ProjectDirs) {
	debug!("{}:{} checking tars for package {}", file!(), line!(), name);
	let build_dir = rua_files::build_dir(dirs, name);
	let dir_items: ReadDir = build_dir.read_dir().unwrap_or_else(|err| {
		panic!(
			"Failed to read directory contents for {:?}, {}",
			&build_dir, err
		)
	});
	let checked_files = dir_items.flat_map(|file| {
		tar_check::tar_check(
			&file
				.expect("Failed to open file for tar_check analysis")
				.path(),
		)
	});
	debug!("all package (tar) files checked, moving them",);
	let checked_tars_dir = rua_files::checked_tars_dir(dirs, name);
	rm_rf::force_remove_all(&checked_tars_dir, true).unwrap_or_else(|err| {
		panic!(
			"Failed to clean checked tar files dir {:?}, {}",
			checked_tars_dir, err,
		)
	});
	fs::create_dir_all(&checked_tars_dir).unwrap_or_else(|err| {
		panic!(
			"Failed to create checked_tars dir {:?}, {}",
			&checked_tars_dir, err
		);
	});

	for file in checked_files {
		let file_name = file.file_name().expect("Failed to parse package tar name");
		let file_name = file_name
			.to_str()
			.expect("Non-UTF8 characters in tar file name");
		fs::rename(&file, checked_tars_dir.join(file_name)).unwrap_or_else(|e| {
			panic!(
				"Failed to move {:?} (build artifact) to {:?}, {}",
				&file, &checked_tars_dir, e,
			)
		});
	}
}

/// Check that the package name is easy to work with in shell
fn check_package_name(name: &str) {
	lazy_static! {
		static ref NAME_REGEX: Regex = Regex::new(r"[a-zA-Z][a-zA-Z._-]*")
			.unwrap_or_else(|_| panic!("{}:{} Failed to parse regexp", file!(), line!()));
	}
	if !NAME_REGEX.is_match(name) {
		eprintln!("Unexpected package name {}", name);
		std::process::exit(1)
	}
}

fn resolve_dependencies(
	name: &str,
	pacman_deps: &mut HashSet<String>,
	aur_packages: &mut HashMap<String, i32>,
	depth: i32,
	alpm: &Alpm,
) {
	check_package_name(&name);
	if let Some(old_depth) = aur_packages.get(name) {
		let old_depth = *old_depth;
		aur_packages.insert(name.to_owned(), cmp::max(depth + 1, old_depth));
		info!("Skipping already resolved package {}", name);
	} else {
		aur_packages.insert(name.to_owned(), depth);
		let info = raur_info(&name);
		let deps = info
			.depends
			.iter()
			.chain(info.make_depends.iter())
			.collect::<Vec<_>>();
		for dep in deps.into_iter() {
			if pacman::is_package_installed(alpm, &dep) {
				// skip if already installed
			} else if !pacman::is_package_installable(alpm, &dep) {
				info!(
					"{} depends on AUR package {}. Trying to resolve it...",
					name, &dep
				);
				resolve_dependencies(&dep, pacman_deps, aur_packages, depth + 1, alpm);
			} else {
				pacman_deps.insert(dep.to_owned());
			}
		}
	}
}

fn raur_info(pkg: &str) -> Package {
	trace!(
		"{}:{} Fetching AUR information for package {}",
		file!(),
		line!(),
		pkg
	);
	let info = raur::info(&[pkg]);
	let info = info.unwrap_or_else(|e| panic!("Failed to fetch info for package {}, {}", &pkg, e));
	match info.into_iter().next() {
		Some(pkg) => pkg,
		None => {
			eprintln!("Package {} not found in AUR", pkg);
			std::process::exit(1)
		}
	}
}
