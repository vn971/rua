use crate::pacman;
use crate::reviewing;
use crate::rua_files;
use crate::tar_check;
use crate::terminal_util;
use crate::wrapped;

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

pub fn install(targets: &[String], dirs: &ProjectDirs, is_offline: bool, asdeps: bool) {
	let mut pacman_deps = HashSet::new();
	let mut split_to_depth = HashMap::new();
	let mut split_to_pkgbase = HashMap::new();
	let mut split_to_version = HashMap::new();
	let alpm = pacman::create_alpm();
	for install_target in targets {
		resolve_dependencies(
			&install_target,
			&mut pacman_deps,
			&mut split_to_depth,
			&mut split_to_pkgbase,
			&mut split_to_version,
			0,
			&alpm,
		);
	}
	pacman_deps.retain(|name| !pacman::is_package_installed(&alpm, name));
	show_install_summary(&pacman_deps, &split_to_depth);
	for pkgbase in split_to_pkgbase.values().collect::<HashSet<_>>() {
		let dir = rua_files::review_dir(dirs, pkgbase);
		fs::create_dir_all(&dir).unwrap_or_else(|err| {
			panic!("Failed to create repository dir for {}, {}", pkgbase, err)
		});
		reviewing::review_repo(&dir, pkgbase, dirs);
	}
	pacman::ensure_pacman_packages_installed(pacman_deps);
	install_all(
		dirs,
		split_to_depth,
		split_to_pkgbase,
		split_to_version,
		is_offline,
		asdeps,
	);
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

fn install_all(
	dirs: &ProjectDirs,
	split_to_depth: HashMap<String, i32>,
	split_to_pkgbase: HashMap<String, String>,
	split_to_version: HashMap<String, String>,
	offline: bool,
	asdeps: bool,
) {
	let archive_whitelist = split_to_version
		.into_iter()
		.map(|pair| format!("{}-{}", pair.0, pair.1))
		.collect::<Vec<_>>();
	trace!("All expected archive files: {:?}", archive_whitelist);
	// get a list of (pkgbase, depth)
	let packages = split_to_pkgbase.iter().map(|(split, pkgbase)| {
		let depth = split_to_depth
			.get(split)
			.expect("Internal error: split package doesn't have recursive depth");
		(pkgbase.to_string(), *depth, split.to_string())
	});
	// sort pairs in descending depth order
	let packages = packages.sorted_by_key(|(_pkgbase, depth, _split)| -depth);
	// Note that a pkgbase can appear at multiple depths because
	// multiple split pkgnames can be at multiple depths.
	// In this case, we only take the first occurrence of pkgbase,
	// which would be the maximum depth because of sort order.
	// We only take one occurrence because we want the package to only be built once.
	let packages: Vec<(String, i32, String)> = packages
		.unique_by(|(pkgbase, _depth, _split)| pkgbase.to_string())
		.collect::<Vec<_>>();
	// once we have a collection of pkgname-s and their depth, proceed straightforwardly.
	for (depth, packages) in &packages.iter().group_by(|(_pkgbase, depth, _split)| *depth) {
		let packages = packages.collect::<Vec<&(String, i32, String)>>();
		for (pkgbase, _depth, _split) in &packages {
			let review_dir = rua_files::review_dir(dirs, pkgbase);
			let build_dir = rua_files::build_dir(dirs, pkgbase);
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
		for (pkgbase, _depth, _split) in &packages {
			check_tars_and_move(pkgbase, dirs, &archive_whitelist);
		}
		// This relation between split_name and the archive file is not actually correct here.
		// Instead, all archive files of some group will be bound to one split name only here.
		// This is probably still good enough for install verification though --
		// and we only use this relation for this purpose. Feel free to improve, if you want...
		let mut files_to_install: Vec<(String, PathBuf)> = Vec::new();
		for (pkgbase, _depth, split) in &packages {
			let checked_tars = rua_files::checked_tars_dir(dirs, &pkgbase);
			let read_dir_iterator = fs::read_dir(checked_tars).unwrap_or_else(|e| {
				panic!(
					"Failed to read 'checked_tars' directory for {}, {}",
					pkgbase, e
				)
			});

			for file in read_dir_iterator {
				files_to_install.push((
					split.to_string(),
					file.expect("Failed to access checked_tars dir").path(),
				));
			}
		}
		pacman::ensure_aur_packages_installed(files_to_install, asdeps || depth > 0);
	}
}

pub fn check_tars_and_move(name: &str, dirs: &ProjectDirs, archive_whitelist: &[String]) {
	debug!("{}:{} checking tars for package {}", file!(), line!(), name);
	let build_dir = rua_files::build_dir(dirs, name);
	let dir_items: ReadDir = build_dir.read_dir().unwrap_or_else(|err| {
		panic!(
			"Failed to read directory contents for {:?}, {}",
			&build_dir, err
		)
	});
	let dir_items = dir_items.map(|f| f.expect("Failed to open file for tar_check analysis"));
	let dir_items = dir_items
		.filter(|file| {
			let file_name = file.file_name();
			let file_name = file_name
				.to_str()
				.expect("Non-UTF8 characters in tar file name");
			archive_whitelist
				.iter()
				.any(|prefix| file_name.starts_with(prefix))
		})
		.collect::<Vec<_>>();
	trace!("Files filtered for tar checking: {:?}", &dir_items);
	for file in dir_items.iter() {
		tar_check::tar_check(&file.path())
	}
	debug!("all package (tar) files checked, moving them");
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

	for file in dir_items {
		let file_name = file.file_name();
		let file_name = file_name
			.to_str()
			.expect("Non-UTF8 characters in tar file name");
		fs::rename(&file.path(), checked_tars_dir.join(file_name)).unwrap_or_else(|e| {
			panic!(
				"Failed to move {:?} (build artifact) to {:?}, {}",
				&file, &checked_tars_dir, e,
			)
		});
	}
}

fn clean_and_check_package_name(name: &str) -> String {
	match clean_package_name(name) {
		Some(name) => name,
		None => {
			eprintln!("Unexpected package name {}", name);
			std::process::exit(1)
		}
	}
}

fn clean_package_name(name: &str) -> Option<String> {
	lazy_static! {
		static ref CLEANUP: Regex = Regex::new(r"(=.*|>.*|<.*)").unwrap_or_else(|err| panic!(
			"{}:{} Failed to parse regexp, {}",
			file!(),
			line!(),
			err
		));
	}
	let name: String = CLEANUP.replace_all(name, "").to_lowercase();
	lazy_static! {
		// From PKGBUILD manual page:
		// Valid characters are alphanumerics, and any of the following characters: “@ . _ + -”.
		// Additionally, names are not allowed to start with hyphens or dots.
		static ref NAME_REGEX: Regex = Regex::new(r"^[a-z0-9@_+][a-z0-9@_+.-]*$").unwrap_or_else(
			|err| panic!("{}:{} Failed to parse regexp, {}", file!(), line!(), err)
		);
	}
	if NAME_REGEX.is_match(&name) {
		Some(name)
	} else {
		None
	}
}

/// Resolve dependencies recursively.
/// "split_name" is the `pkgname` in PKGBUILD terminology. It's called "split" to avoid
/// ambiguity of "package name" meaning.
fn resolve_dependencies(
	split_name: &str,
	pacman_deps: &mut HashSet<String>,
	split_to_depth: &mut HashMap<String, i32>,
	split_to_pkgbase: &mut HashMap<String, String>,
	split_to_version: &mut HashMap<String, String>,
	depth: i32,
	alpm: &Alpm,
) {
	let split_name = clean_and_check_package_name(&split_name);
	if let Some(old_depth) = split_to_depth.get(&split_name) {
		let old_depth = *old_depth;
		split_to_depth.insert(split_name.to_owned(), cmp::max(depth + 1, old_depth));
		info!("Skipping already resolved package {}", split_name);
	} else {
		split_to_depth.insert(split_name.to_owned(), depth);
		let info = raur_info_assert_one(&split_name);
		split_to_pkgbase.insert(split_name.to_string(), info.package_base);
		split_to_version.insert(split_name.to_string(), info.version);
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
					split_name, &dep
				);
				resolve_dependencies(
					&dep,
					pacman_deps,
					split_to_depth,
					split_to_pkgbase,
					split_to_version,
					depth + 1,
					alpm,
				);
			} else {
				pacman_deps.insert(dep.to_owned());
			}
		}
	}
}

fn raur_info_assert_one(pkg: &str) -> Package {
	match raur_info(pkg) {
		Some(pkg) => pkg,
		None => {
			eprintln!("Package {} not found in AUR", pkg);
			std::process::exit(1)
		}
	}
}

pub fn raur_info(pkg: &str) -> Option<Package> {
	trace!(
		"{}:{} Fetching AUR information for package {}",
		file!(),
		line!(),
		pkg
	);
	let info = raur::info(&[pkg]);
	let info = info.unwrap_or_else(|e| panic!("Failed to fetch info for package {}, {}", &pkg, e));
	info.into_iter().next()
}

#[cfg(test)]
mod tests {
	use crate::action_install::*;

	#[test]
	fn test_starting_hyphen() {
		assert_eq!(clean_package_name("test"), Some("test".to_string()));
		assert_eq!(
			clean_package_name("abcdefghijklmnopqrstuvwxyz0123456789@_+.-"),
			Some("abcdefghijklmnopqrstuvwxyz0123456789@_+.-".to_string())
		);

		assert_eq!(clean_package_name(""), None);
		assert_eq!(clean_package_name("-test"), None);
		assert_eq!(clean_package_name(".test"), None);
		assert_eq!(clean_package_name("!"), None);
		assert_eq!(clean_package_name("german_ö"), None);

		assert_eq!(clean_package_name("@"), Some("@".to_string()));
		assert_eq!(clean_package_name("_"), Some("_".to_string()));
		assert_eq!(clean_package_name("+"), Some("+".to_string()));

		assert_eq!(clean_package_name("test>=0"), Some("test".to_string()));
		assert_eq!(clean_package_name("test>0"), Some("test".to_string()));
		assert_eq!(clean_package_name("test<0"), Some("test".to_string()));
		assert_eq!(clean_package_name("test<=0"), Some("test".to_string()));
		assert_eq!(clean_package_name("test=0"), Some("test".to_string()));
	}
}
