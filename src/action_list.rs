use crate::alpm_wrapper::new_alpm_wrapper;
use crate::alpm_wrapper::AlpmWrapper;
use crate::aur_rpc_utils;
use crate::pacman;
use colored::*;
use itertools::Itertools;
use log::debug;
use log::warn;
use prettytable::format::*;
use prettytable::*;
use std::cmp::Ordering;
use std::collections::HashSet;

type RAURVersion = Option<String>;

pub fn list_aur_packages() {
	let system_ignored_packages = pacman::get_ignored_packages().unwrap_or_else(|err| {
		warn!("Could not get ignored packages, {}", err);
		HashSet::new()
	});

	let alpm = new_alpm_wrapper();
	let aur_pkgs = alpm
		.get_non_pacman_packages()
		.expect("failed to get non-pacman packages");

	let aur_pkgs_string = aur_pkgs
		.iter()
		.map(|(name, _version)| name.to_string())
		.collect::<Vec<_>>()
		.join(" ");
	debug!("You have the following packages outside of main repos installed:");
	debug!("{}", aur_pkgs_string);
	debug!("");

	let mut all_pkgs: Vec<(String, String, RAURVersion)> = Vec::new();
	let mut ignored = Vec::new();

	let info_map = aur_rpc_utils::info_map(&aur_pkgs.iter().map(|(p, _)| p).collect_vec());
	let info_map = info_map.unwrap_or_else(|err| panic!("Failed to get AUR information: {}", err));

	for (pkg, local_ver) in aur_pkgs {
		let raur_ver: Option<String> = info_map.get(&pkg).map(|p| p.version.to_string());

		if let Some(raur_ver) = raur_ver {
			if system_ignored_packages.contains(&pkg) {
				ignored.push(pkg.to_string());
			} else {
				all_pkgs.push((pkg, local_ver.to_string(), Some(raur_ver)));
			}
		} else if system_ignored_packages.contains(&pkg) {
			ignored.push(pkg.to_string());
		} else {
			all_pkgs.push((pkg, local_ver.to_string(), None));
		}
	}
	if !ignored.is_empty() {
		let ignored_string = ignored.join(" ");
		warn!(
			"The following packages have changed in AUR but are ignored in the system: {}",
			ignored_string
		);
	};
	print_packages(&all_pkgs, alpm);
}

fn print_packages(pkgs: &[(String, String, RAURVersion)], alpm: Box<dyn AlpmWrapper>) {
	let mut table = Table::new();
	table.set_titles(row![
		"Package".underline(),
		"Current".underline(),
		"Latest".underline()
	]);

	for (pkg, local, aur_version) in pkgs {
		let pkg_row = match aur_version {
			// if found in neither pacman nor AUR, just display this in dimmed as the AUR version
			None => row![
				pkg,
				local,
				"not found in neither pacman nor AUR, ignoring".dimmed(),
			],
			Some(aur_version) => {
				// if there is an AUR version and the local version is outdated, mark the package name in yellow and the AUR version in green
				if alpm
					.version_compare(local, aur_version)
					.expect("Failed to compare local version to AUR version")
					== Ordering::Less
				{
					row![pkg.yellow(), local, aur_version.green(),]
				} else {
					// if AUR version asnd local version are identical, just display the line in white.
					row![pkg, local, aur_version,]
				}
			}
		};
		table.add_row(pkg_row);
	}
	let fmt: TableFormat = FormatBuilder::new().padding(0, 1).build();
	table.set_format(fmt);
	table.printstd();
}
