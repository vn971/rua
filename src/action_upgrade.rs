use crate::action_install;
use crate::aur_rpc_utils;
use crate::pacman;
use crate::print_package_table;
use crate::terminal_util;
use alpm::Version;
use colored::*;
use directories::ProjectDirs;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::debug;
use prettytable::format::*;
use prettytable::*;
use regex::Regex;
use std::collections::HashSet;

fn pkg_is_devel(name: &str) -> bool {
	lazy_static! {
		// make sure that the --devel help string in cli_args.rs matches if you change this
		static ref RE: Regex = Regex::new(r"-(git|hg|bzr|svn|cvs|darcs)(-.+)*$").unwrap();
	}
	RE.is_match(name)
}

pub fn upgrade(dirs: &ProjectDirs, devel: bool) {
	let alpm = pacman::create_alpm();
	let pkg_cache = alpm
		.localdb()
		.pkgs()
		.expect("Could not get alpm.localdb().pkgs() packages");
	let ignored_packages = pacman::get_ignored_packages().unwrap_or_else(|err| {
		eprintln!("Warning: Could not get ignored packages, {}", err);
		HashSet::new()
	});
	let aur_pkgs = pkg_cache
		.filter(|pkg| !pacman::is_installable(&alpm, pkg.name()))
		.filter(|pkg| !ignored_packages.contains(pkg.name()))
		.map(|pkg| (pkg.name(), pkg.version()))
		.collect::<Vec<_>>();
	let aur_pkgs_string = aur_pkgs
		.iter()
		.map(|(pkg, _ver)| *pkg)
		.collect::<Vec<_>>()
		.join(" ");
	debug!("You have the following packages outside of main repos installed:");
	debug!("{}", aur_pkgs_string);
	debug!("");
	let mut up_to_date = Vec::new();
	let mut outdated = Vec::new();
	let mut unexistent = Vec::new();
	let info_map = aur_rpc_utils::info_map(&aur_pkgs.iter().map(|(p, _)| *p).collect_vec());
	let info_map = info_map.unwrap_or_else(|err| panic!("Failed to get AUR information: {}", err));
	for (pkg, local_ver) in aur_pkgs {
		let raur_ver = info_map.get(pkg).map(|p| p.version.to_string());
		if let Some(raur_ver) = raur_ver {
			if local_ver < Version::new(&raur_ver) || (devel && pkg_is_devel(pkg)) {
				outdated.push((pkg, local_ver.to_string(), raur_ver));
			} else {
				up_to_date.push(pkg);
			}
		} else {
			unexistent.push((pkg, local_ver.to_string()));
		}
	}
	if outdated.is_empty() {
		eprintln!("All AUR packages are up-to-date. Congratulations!");
	} else {
		print_outdated(&outdated, &unexistent);
		eprintln!();
		let outdated: Vec<String> = outdated.iter().map(|o| o.0.to_string()).collect();
		loop {
			eprint!("Do you wish to upgrade them? [O]=ok, [X]=exit. ");
			let string = terminal_util::read_line_lowercase();
			if string == "o" {
				action_install::install(&outdated, dirs, false, true);
				break;
			} else if string == "x" {
				break;
			}
		}
	}
}

fn print_outdated(outdated: &[(&str, String, String)], unexistent: &[(&str, String)]) {
	let mut table = Table::new();
	table.set_titles(row![
		"Package".underline(),
		"Current".underline(),
		"Latest".underline()
	]);

	for (pkg, local, remote) in outdated {
		table.add_row(row![
			print_package_table::trunc(pkg, 39).yellow(),
			print_package_table::trunc(local, 19),
			print_package_table::trunc(remote, 19).green(),
		]);
	}
	for (pkg, local) in unexistent {
		table.add_row(row![
			print_package_table::trunc(pkg, 39).yellow(),
			print_package_table::trunc(local, 19),
			"NOT FOUND, ignored".red(),
		]);
	}
	let fmt: TableFormat = FormatBuilder::new().padding(0, 1).build();
	table.set_format(fmt);
	table.printstd();
}
