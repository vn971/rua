use crate::action_install;
use crate::pacman;
use crate::print_package_table;
use crate::terminal_util;
use aur_depends::{AurUpdate, AurUpdates, Flags, Resolver};
use colored::*;
use directories::ProjectDirs;
use prettytable::format::*;
use prettytable::*;
use std::collections::HashSet;

pub fn upgrade(dirs: &ProjectDirs) {
	let alpm = pacman::create_alpm();
	let raur = raur::Handle::default();
	let mut cache = HashSet::new();
	let mut resolver = Resolver::new(&alpm, &mut cache, &raur, Flags::new() | Flags::AUR_ONLY);

	let updates = resolver
		.aur_updates()
		.unwrap_or_else(|e| panic!("failed to get aur updates {}", e));

	if updates.updates.is_empty() {
		eprintln!("All AUR packages are up-to-date. Congratulations!");
	} else {
		print_updates(&updates);
		eprintln!();
		let outdated: Vec<String> = updates
			.updates
			.iter()
			.map(|u| u.remote.name.to_string())
			.collect();
		loop {
			eprint!("Do you wish to upgrade them? [O]=ok, [X]=exit. ");
			let string = terminal_util::read_line_lowercase();
			if string == "o" {
				action_install::install(resolver, &outdated, dirs, false, true);
				break;
			} else if string == "x" {
				break;
			}
		}
	}
}

fn print_updates(updates: &AurUpdates) {
	let mut table = Table::new();
	table.set_titles(row![
		"Package".underline(),
		"Current".underline(),
		"Latest".underline()
	]);

	let AurUpdates { updates, missing } = updates;

	for AurUpdate { local, remote } in updates {
		table.add_row(row![
			print_package_table::trunc(&remote.name, 39).yellow(),
			print_package_table::trunc(local.version(), 19),
			print_package_table::trunc(&remote.version, 19).green(),
		]);
	}
	for pkg in missing {
		table.add_row(row![
			print_package_table::trunc(pkg.name(), 39).yellow(),
			print_package_table::trunc(pkg.version(), 19),
			"NOT FOUND, ignored".red(),
		]);
	}

	let fmt: TableFormat = FormatBuilder::new().padding(0, 1).build();
	table.set_format(fmt);
	table.printstd();
}
