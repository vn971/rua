use crate::action_install;
use crate::alpm_wrapper::new_alpm_wrapper;
use crate::alpm_wrapper::AlpmWrapper;
use crate::aur_rpc_utils;
use crate::pacman;
use crate::rua_paths::RuaPaths;
use crate::terminal_util;
use anyhow::Result;
use colored::*;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::debug;
use log::warn;
use prettytable::format::*;
use prettytable::*;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::HashSet;

fn pkg_is_devel(name: &str) -> bool {
	lazy_static! {
		// make sure that the --devel help string in cli_args.rs matches if you change this
		static ref RE: Regex = Regex::new(r"-(git|hg|bzr|svn|cvs|darcs)(-.+)*$").unwrap();
	}
	RE.is_match(name)
}

pub fn upgrade_printonly(
	devel: bool,
	ignored: &HashSet<&str>,
	only_packages: &HashSet<&str>,
) -> Result<()> {
	let alpm = new_alpm_wrapper();
	let (outdated, nonexistent) = calculate_upgrade(&*alpm, devel, ignored, only_packages)?;

	if outdated.is_empty() && nonexistent.is_empty() {
		eprintln!(
			"{}",
			"Good job! All AUR packages are up-to-date.".bright_green()
		);
	} else {
		for (pkg, _, _) in outdated {
			println!("{}", pkg);
		}
		for (pkg, _) in nonexistent {
			println!("{}", pkg);
		}
	}
	Ok(())
}

pub fn upgrade_real(
	devel: bool,
	rua_paths: &RuaPaths,
	ignored: &HashSet<&str>,
	only_packages: &HashSet<&str>,
) -> Result<()> {
	let alpm = new_alpm_wrapper();
	let (outdated, nonexistent) = calculate_upgrade(&*alpm, devel, ignored, only_packages)?;

	if outdated.is_empty() && nonexistent.is_empty() {
		eprintln!(
			"{}",
			"Good job! All AUR packages are up-to-date.".bright_green()
		);
		std::process::exit(7);
	} else if outdated.is_empty() {
		eprintln!("All AUR packages are up-to-date, but there are some packages installed locally that do not exist in neither pacman nor AUR.");
		eprintln!("These might be old dependencies from changed packages in pacman, or an AUR package that was removed from AUR.");
		eprintln!("Consider removing these packages, or ignoring if it's a personal package:");
		eprintln!();
		print_outdated(&outdated, &nonexistent);
	} else {
		print_outdated(&outdated, &nonexistent);
		eprintln!();
		loop {
			eprint!("Do you wish to upgrade them? [O]=ok, [X]=exit. ");
			let user_input = terminal_util::read_line_lowercase();
			if &user_input == "o" {
				let outdated: Vec<String> = outdated.iter().map(|o| o.0.to_string()).collect();
				action_install::install(&outdated, rua_paths, false, true);
				break;
			} else if &user_input == "x" {
				break;
			}
		}
	}
	Ok(())
}

type OutdatedPkgs = Vec<(String, String, String)>;
type ForeignPkgs = Vec<(String, String)>;

fn calculate_upgrade(
	alpm: &dyn AlpmWrapper,
	devel: bool,
	locally_ignored_packages: &HashSet<&str>,
	only_packages: &HashSet<&str>,
) -> Result<(OutdatedPkgs, ForeignPkgs)> {
	let system_ignored_packages = pacman::get_ignored_packages().unwrap_or_else(|err| {
		warn!("Could not get ignored packages, {}", err);
		HashSet::new()
	});

	let aur_pkgs = alpm.get_non_pacman_packages()?;

	if !only_packages.is_empty() {
		let installed: HashSet<&str> = aur_pkgs.iter().map(|(n, _)| n.as_str()).collect();
		for pkg in only_packages {
			if !installed.contains(pkg) {
				anyhow::bail!("Package {} is not installed", pkg);
			}
		}
	}

	let aur_pkgs_string = aur_pkgs
		.iter()
		.map(|(name, _version)| name.to_string())
		.collect::<Vec<_>>()
		.join(" ");
	debug!("You have the following packages outside of main repos installed:");
	debug!("{}", aur_pkgs_string);
	debug!("");

	let mut outdated = Vec::new();
	let mut nonexistent = Vec::new();
	let mut ignored = Vec::new();

	let info_map = aur_rpc_utils::info_map(&aur_pkgs.iter().map(|(p, _)| p).collect_vec());
	let info_map = info_map.unwrap_or_else(|err| panic!("Failed to get AUR information: {}", err));

	for (pkg, local_ver) in aur_pkgs {
		if !only_packages.is_empty() && !only_packages.contains(pkg.as_str()) {
			continue;
		}
		let raur_ver: Option<String> = info_map.get(&pkg).map(|p| p.version.to_string());

		if let Some(raur_ver) = raur_ver {
			if alpm.version_compare(&local_ver, &raur_ver)? == Ordering::Less
				|| (devel && pkg_is_devel(&pkg))
			{
				if locally_ignored_packages.contains(pkg.as_str())
					|| system_ignored_packages.contains(&pkg)
				{
					ignored.push(pkg.to_string());
				} else {
					outdated.push((pkg, local_ver.to_string(), raur_ver));
				}
			}
		} else if locally_ignored_packages.contains(pkg.as_str())
			|| system_ignored_packages.contains(&pkg)
		{
			ignored.push(pkg.to_string());
		} else {
			nonexistent.push((pkg, local_ver.to_string()));
		}
	}
	if !ignored.is_empty() {
		let ignored_string = ignored.join(" ");
		warn!(
			"The following packages have changed in AUR but are ignored in the system: {}",
			ignored_string
		);
	};

	Ok((outdated, nonexistent))
}

fn print_outdated(outdated: &[(String, String, String)], nonexistent: &[(String, String)]) {
	let mut table = Table::new();
	table.set_titles(row![
		"Package".underline(),
		"Current".underline(),
		"Latest".underline()
	]);

	for (pkg, local, remote) in outdated {
		table.add_row(row![pkg.yellow(), local, remote.green(),]);
	}
	for (pkg, local) in nonexistent {
		table.add_row(row![
			pkg.yellow(),
			local,
			"not found in neither pacman nor AUR, ignoring".dimmed(),
		]);
	}
	let fmt: TableFormat = FormatBuilder::new().padding(0, 1).build();
	table.set_format(fmt);
	table.printstd();
}
