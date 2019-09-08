use crate::pacman;
use crate::reviewing;
use crate::rua_files;
use crate::tar_check;
use crate::terminal_util;
use crate::wrapped;

use aur_depends::{Actions, Missing, Resolver};
use directories::ProjectDirs;
use fs_extra::dir::CopyOptions;
use itertools::Itertools;
use log::debug;
use log::trace;
use std::fs;
use std::fs::ReadDir;
use std::path::PathBuf;

pub fn install(
	resolver: Resolver,
	targets: &[String],
	dirs: &ProjectDirs,
	is_offline: bool,
	as_deps: bool,
) {
	let actions = resolver
		.resolve_targets(targets)
		.unwrap_or_else(|e| panic!("failed to resolver deps {}", e));

	if !actions.missing.is_empty() {
		eprintln!("Need to install packages: but they are not found:");

		for Missing { stack, dep } in actions.missing {
			eprint!("{}", dep);
			if !stack.is_empty() {
				eprint!(" (Wanted by: {})", stack.join(" -> "));
			}
			eprintln!();
		}
		std::process::exit(1)
	}

	show_install_summary(&actions);

	for base in &actions.build {
		let dir = rua_files::review_dir(dirs, &base.pkgbase);
		fs::create_dir_all(&dir).unwrap_or_else(|err| {
			panic!(
				"Failed to create repository dir for {}, {}",
				base.pkgbase, err
			)
		});
		reviewing::review_repo(&dir, &base.pkgbase, dirs);
	}

	pacman::ensure_pacman_packages_installed(&actions);
	install_all(dirs, &actions, as_deps, is_offline)
}

fn show_install_summary(actions: &Actions) {
	if !actions.install.is_empty() {
		println!("Repo packages to install:");

		for install in &actions.install {
			let pkg = &install.pkg;
			println!("    {}", pkg.name())
		}
		println!();
	}

	if !actions.build.is_empty() {
		println!("Aur packages to install:");

		for install in actions.iter_build_pkgs() {
			let pkg = &install.pkg;
			println!("    {}", pkg.name)
		}
		println!();
	}

	loop {
		eprint!("Proceed? [O]=ok, Ctrl-C=abort. ");
		let string = terminal_util::read_line_lowercase();
		if string == "o" {
			break;
		}
	}
}

fn install_all(dirs: &ProjectDirs, actions: &Actions, as_deps: bool, offline: bool) {
	let archive_whitelist = actions
		.iter_build_pkgs()
		.map(|pkg| pkg.pkg.name.as_str())
		.collect::<Vec<_>>();
	trace!("All expected split packages: {:?}", archive_whitelist);

	for base in &actions.build {
		let pkgbase = &base.pkgbase;
		let review_dir = rua_files::review_dir(dirs, pkgbase);
		let build_dir = rua_files::build_dir(dirs, pkgbase);
		rm_rf::force_remove_all(&build_dir).expect("Failed to remove old build dir");
		std::fs::create_dir_all(&build_dir).expect("Failed to create build dir");
		fs_extra::copy_items(
			&vec![review_dir],
			rua_files::global_build_dir(dirs),
			&CopyOptions::new(),
		)
		.expect("failed to copy reviewed dir to build dir");
		rm_rf::force_remove_all(build_dir.join(".git")).expect("Failed to remove .git");
		wrapped::build_directory(
			&build_dir.to_str().expect("Non-UTF8 directory name"),
			dirs,
			offline,
		);
	}
	for base in &actions.build {
		check_tars_and_move(&base.pkgbase, dirs, &archive_whitelist);
	}
	// This relation between split_name and the archive file is not actually correct here.
	// Instead, all archive files of some group will be bound to one split name only here.
	// This is probably still good enough for install verification though --
	// and we only use this relation for this purpose. Feel free to improve, if you want...
	let mut files_to_install: Vec<(String, PathBuf)> = Vec::new();
	for base in &actions.build {
		let pkgbase = &base.pkgbase;
		let checked_tars = rua_files::checked_tars_dir(dirs, &pkgbase);
		let read_dir_iterator = fs::read_dir(checked_tars).unwrap_or_else(|e| {
			panic!(
				"Failed to read 'checked_tars' directory for {}, {}",
				pkgbase, e
			)
		});

		for file in read_dir_iterator {
			files_to_install.push((
				pkgbase.to_string(),
				file.expect("Failed to access checked_tars dir").path(),
			));
		}
	}

	pacman::ensure_aur_packages_installed(files_to_install, true || as_deps); //TODO hande as_deps properly
}

pub fn check_tars_and_move(name: &str, dirs: &ProjectDirs, archive_whitelist: &[&str]) {
	debug!("checking tars and moving for package {}", name);
	let build_dir = rua_files::build_dir(dirs, name);
	let dir_items: ReadDir = build_dir.read_dir().unwrap_or_else(|err| {
		panic!(
			"Failed to read directory contents for {:?}, {}",
			&build_dir, err
		)
	});
	let dir_items = dir_items.map(|f| f.expect("Failed to open file for tar_check analysis"));
	let mut dir_items = dir_items
		.map(|file| {
			let file_name = file.file_name();
			let file_name = file_name
				.into_string()
				.expect("Non-UTF8 characters in tar file name");
			(file, file_name)
		})
		.filter(|(_, name)| (name.ends_with(".pkg.tar") || name.ends_with(".pkg.tar.xz")))
		.collect::<Vec<_>>();
	let dir_items_names = dir_items
		.iter()
		.map(|(_, name)| name.as_str())
		.collect_vec();
	let common_suffix_length =
		tar_check::common_suffix_length(&dir_items_names, &archive_whitelist);
	dir_items.retain(|(_, name)| {
		archive_whitelist.contains(&&name[..name.len() - common_suffix_length])
	});
	trace!("Files filtered for tar checking: {:?}", &dir_items);
	for (file, file_name) in dir_items.iter() {
		tar_check::tar_check_unwrap(&file.path(), file_name);
	}
	debug!("all package (tar) files checked, moving them");
	let checked_tars_dir = rua_files::checked_tars_dir(dirs, name);
	rm_rf::force_remove_all(&checked_tars_dir).unwrap_or_else(|err| {
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

	for (file, file_name) in dir_items {
		fs::rename(&file.path(), checked_tars_dir.join(file_name)).unwrap_or_else(|e| {
			panic!(
				"Failed to move {:?} (build artifact) to {:?}, {}",
				&file, &checked_tars_dir, e,
			)
		});
	}
}
