use crate::pacman;
use crate::{action_install, terminal_util};
use colored::*;
use directories::ProjectDirs;
use log::debug;
use std::collections::HashSet;
use version_compare::CompOp;
use version_compare::VersionCompare;

pub fn upgrade(dirs: &ProjectDirs) {
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
		.map(|pkg| (pkg.name(), pkg.version().to_string()))
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
	for (pkg, local_ver) in aur_pkgs {
		let raur_ver = action_install::raur_info(pkg).map(|p| p.version);
		if let Some(raur_ver) = raur_ver {
			let is_outdated = VersionCompare::compare_to(&local_ver, &raur_ver, &CompOp::Lt)
				.unwrap_or_else(|_| {
					eprintln!(
						"Could not compare local->upstream versions: {}->{}  Assuming outdated...",
						local_ver.red(),
						raur_ver.green()
					);
					true
				});
			if is_outdated {
				outdated.push((pkg, local_ver, raur_ver));
			} else {
				up_to_date.push(pkg);
			}
		} else {
			unexistent.push(pkg);
		}
	}
	if !unexistent.is_empty() {
		eprintln!("The following packages do not seem to exist in neither AUR nor main repos:");
		eprintln!("{}", unexistent.join(" "));
		eprintln!("Consider deleting them or verifying if they are really in use.");
		eprintln!();
	}
	if outdated.is_empty() {
		eprintln!("All AUR packages are up-to-date. Congratulations!");
	} else {
		eprintln!("The following AUR packages have upstream upgrades:");
		for (pkg, local, remote) in &outdated {
			eprintln!("{}  {} -> {}", pkg, local, remote);
		}
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
