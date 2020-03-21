use crate::aur_rpc_utils;
use crate::pacman;
use crate::reviewing;
use crate::rua_files::RuaDirs;
use crate::tar_check;
use crate::terminal_util;
use crate::wrapped;
use fs_extra::dir::CopyOptions;
use indexmap::IndexMap;
use indexmap::IndexSet;
use itertools::Itertools;
use log::*;
use raur::Package;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::fs::ReadDir;
use std::path::PathBuf;

pub fn install(targets: &[String], dirs: &RuaDirs, is_offline: bool, asdeps: bool) {
	let alpm = pacman::create_alpm();
	let (split_to_raur, pacman_deps, split_to_depth) =
		aur_rpc_utils::recursive_info(targets, &alpm).unwrap_or_else(|err| {
			panic!("Failed to fetch info from AUR, {}", err);
		});
	let split_to_pkgbase: IndexMap<String, String> = split_to_raur
		.iter()
		.map(|(split, raur)| (split.to_string(), raur.package_base.to_string()))
		.collect();
	let not_found = split_to_depth
		.keys()
		.filter(|pkg| !split_to_raur.contains_key(*pkg))
		.collect_vec();
	if !not_found.is_empty() {
		eprintln!(
			"Need to install packages: {:?}, but they are not found on AUR.",
			not_found
		);
		std::process::exit(1)
	}
	show_install_summary(targets, split_to_raur, &alpm);
	for pkgbase in split_to_pkgbase.values().collect::<HashSet<_>>() {
		let dir = dirs.review_dir(pkgbase);
		fs::create_dir_all(&dir).unwrap_or_else(|err| {
			panic!("Failed to create repository dir for {}, {}", pkgbase, err)
		});
		reviewing::review_repo(&dir, pkgbase, dirs);
	}
	pacman::ensure_pacman_packages_installed(pacman_deps);
	install_all(dirs, split_to_depth, split_to_pkgbase, is_offline, asdeps);
}

// Prints the dependency tree
fn show_install_summary(
	targets: &[String],
	split_to_raur: IndexMap<String, Package>,
	alpm: &alpm::Alpm,
) {
	for target in targets {
		// Create dep_map of depth 1 and 2
		let mut deps_1_map: HashMap<String, Vec<(String, String)>> = HashMap::default();
		let mut deps_2_map: HashMap<String, Vec<(String, String)>> = HashMap::default();
		// Add deps-dependencies into the map
		gen_deps_depth_1_and_2(
			&mut deps_1_map,
			&mut deps_2_map,
			&split_to_raur,
			target,
			alpm,
		);
		// If there are no dependencies to install, return
		if deps_1_map.is_empty() {
			return;
		}
		// Print the dependency tree from the dependency map data
		print_dep_tree(&target, &mut deps_1_map, &deps_2_map);
	}
	loop {
		eprint!("Proceed? [O]=ok, Ctrl-C=abort. ");
		let string = terminal_util::read_line_lowercase();
		if &string == "o" {
			break;
		}
	}
}

fn install_all(
	dirs: &RuaDirs,
	split_to_depth: IndexMap<String, i32>,
	split_to_pkgbase: IndexMap<String, String>,
	offline: bool,
	asdeps: bool,
) {
	let archive_whitelist = split_to_depth
		.iter()
		.map(|(split, _depth)| split.as_str())
		.collect::<IndexSet<_>>();
	trace!("All expected split packages: {:?}", archive_whitelist);
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
			let review_dir = dirs.review_dir(pkgbase);
			let build_dir = dirs.build_dir(pkgbase);
			rm_rf::ensure_removed(&build_dir).unwrap_or_else(|err| {
				panic!("Failed to remove old build dir {:?}, {}", &build_dir, err)
			});
			std::fs::create_dir_all(&build_dir).unwrap_or_else(|err| {
				panic!("Failed to create build dir {:?}, {}", &build_dir, err)
			});
			fs_extra::copy_items(
				&vec![&review_dir],
				&dirs.global_build_dir,
				&CopyOptions::new(),
			)
			.unwrap_or_else(|err| {
				panic!(
					"failed to copy reviewed dir {:?} to build dir {:?}, error is {}",
					&review_dir, dirs.global_build_dir, err
				)
			});
			{
				let dir_to_remove = build_dir.join(".git");
				rm_rf::ensure_removed(build_dir.join(".git"))
					.unwrap_or_else(|err| panic!("Failed to remove {:?}, {}", dir_to_remove, err));
			}
			wrapped::build_directory(
				&build_dir.to_str().expect("Non-UTF8 directory name"),
				dirs,
				offline,
				false,
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
			let checked_tars = dirs.checked_tars_dir(&pkgbase);
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

pub fn check_tars_and_move(name: &str, dirs: &RuaDirs, archive_whitelist: &IndexSet<&str>) {
	debug!("checking tars and moving for package {}", name);
	let build_dir = dirs.build_dir(name);
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
		.filter(|(_, name)| {
			name.ends_with(".pkg.tar")
				|| name.ends_with(".pkg.tar.xz")
				|| name.ends_with(".pkg.tar.lzma")
				|| name.ends_with(".pkg.tar.gz")
				|| name.ends_with(".pkg.tar.gzip")
				|| name.ends_with(".pkg.tar.zst")
				|| name.ends_with(".pkg.tar.zstd")
		})
		.collect::<Vec<_>>();
	let dir_items_names = dir_items
		.iter()
		.map(|(_, name)| name.as_str())
		.collect_vec();
	let common_suffix_length =
		tar_check::common_suffix_length(&dir_items_names, &archive_whitelist);
	dir_items
		.retain(|(_, name)| archive_whitelist.contains(&name[..name.len() - common_suffix_length]));
	trace!("Files filtered for tar checking: {:?}", &dir_items);
	for (file, file_name) in dir_items.iter() {
		tar_check::tar_check_unwrap(&file.path(), file_name);
	}
	debug!("all package (tar) files checked, moving them");
	let checked_tars_dir = dirs.checked_tars_dir(name);
	rm_rf::ensure_removed(&checked_tars_dir).unwrap_or_else(|err| {
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

/// Generates the dependencies map of 1st and 2nd depth lvl of the
/// package storing also if the dependencies are from aur or not.
fn gen_package_deps_map(
	map: &mut HashMap<String, Vec<(String, String)>>,
	split_to_raur: &IndexMap<String, Package>,
	dep_name: &str,
	alpm: &alpm::Alpm,
) {
	// info!("INPUT. dep_name: {}, map: {:?}", dep_name, map);
	let raur_package: &Package = match split_to_raur.get(dep_name) {
		Some(pkg) => pkg,
		None => {
			warn!("package not found in the map: {}", dep_name);
			return
		}
	};
	let all_deps = aur_rpc_utils::all_dependencies_of(raur_package);
	let all_deps = all_deps.into_iter().filter(|pkg| !pacman::is_installed(alpm, pkg));

	let mut pacman_deps: IndexSet<String> = IndexSet::new();
	let mut aur_deps: IndexSet<String> = IndexSet::new();
	if pacman::is_installable(alpm, dep_name) {
		// pacman_deps.insert(dep_name.to_string());
	} else {
		// info!("Adding dependency {} because it is not installable", dep_name);
		aur_deps.insert(dep_name.to_string());
	};

	for dep in all_deps {
		if pacman::is_installable(alpm, &dep) {
			pacman_deps.insert(dep);
		} else {
			aur_deps.insert(dep);
		}
	}
	// info!("dep_name: {}, pacman deps: {:?}, aur_deps: {:?}", dep_name, pacman_deps, aur_deps);

	if aur_deps.is_empty() && pacman_deps.is_empty() {
		return;
	}
	let mut dep_vec = Vec::new();
	// Push pacman deps to the dep_vec with the pacman option
	for dep in aur_deps {
		dep_vec.push((dep.to_string(), "AUR".to_string()));
	}
	// Push aur deps to the dep_vec with the AUR option
	for pacman_dep in pacman_deps.iter() {
		if !(dep_vec.contains(&(pacman_dep.to_string(), "pacman".to_string()))
			|| dep_vec.contains(&(pacman_dep.to_string(), "AUR".to_string())))
			&& pacman_dep.clone() != dep_name
		{
			dep_vec.push((pacman_dep.to_string(), "pacman".to_string()))
		}
	}
	map.insert(dep_name.to_string(), dep_vec);
}

/// Generates the dependencies map with a depth = 1 and 2 and adds them
/// into the deps_map provided.
fn gen_deps_depth_1_and_2(
	deps_map_1: &mut HashMap<String, Vec<(String, String)>>,
	deps_map_2: &mut HashMap<String, Vec<(String, String)>>,
	split_to_raur: &IndexMap<String, Package>,
	dep_name: &str,
	alpm: &alpm::Alpm,
) {
	// Gen dep-map of depth 1
	gen_package_deps_map(deps_map_1, split_to_raur, dep_name, alpm);
	// Get 1st depth deps to gen dep-map of depth 2
	for val in deps_map_1.values().into_iter() {
		val.iter()
			.map(|(name, _)| gen_package_deps_map(deps_map_2, split_to_raur, name, alpm))
			.for_each(drop);
	}
}

fn print_dep_tree(
	pack_name: &str,
	deps_1: &mut HashMap<String, Vec<(String, String)>>,
	deps_2: &HashMap<String, Vec<(String, String)>>,
) {
	// Print first dep
	println!(
		"{} ({})",
		deps_1.get(pack_name).unwrap()[0].0,
		deps_1.get(pack_name).unwrap()[0].1
	);

	// Get last item and remove it from the map
	let mut deps_1 = deps_1.get(pack_name).unwrap().clone();
	let last = deps_1.pop().unwrap();
	// Print the tree except the last item
	for (name, repo) in deps_1.iter().skip(1) {
		if deps_2.get(name).is_some() {
			print_deps_with_depth(name, deps_2.get(name).unwrap(), true);
		} else {
			println!("├── {} ({})", name, repo.clone());
		}
	}
	// Print last elem
	println!("└── {} ({})", last.0, last.1);
}

fn print_deps_with_depth(parent_dep: &str, dep_names: &[(String, String)], depth: bool) {
	let dep_names = dep_names.to_vec();
	// Save last elem
	let (last_name, last_repo) = dep_names.clone().pop().unwrap();
	// Print first elem (OG package) without indent
	println!("├── {} ({})", dep_names[0].0, dep_names[0].1);
	// Print the rest of deps
	dep_names
		.iter()
		.filter(|(dep_name, _)| dep_name != parent_dep)
		.map(|(dep_name, repo)| {
			if !depth {
				println!("├── {} ({})", dep_name, repo.clone());
			} else {
				println!("│   ├── {} ({})", dep_name, repo.clone());
			}
		})
		.for_each(drop);
	// Print last dep
	if !depth {
		println!("└── {} ({})", last_name, last_repo);
	} else {
		println!("│   └── {} ({})", last_name, last_repo);
	}
}
