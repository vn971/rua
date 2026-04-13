mod action_builddir;
mod action_install;
mod action_search;
mod action_upgrade;
mod alpm_wrapper;
mod aur_rpc_utils;
mod cli_args;
mod git_utils;
mod pacman;
mod print_format;
mod print_package_info;
mod print_package_table;
mod reviewing;
mod rua_environment;
mod rua_paths;
mod srcinfo_to_pkgbuild;
mod tar_check;
mod terminal_util;
mod wrapped;

use crate::print_package_info::info;
use crate::wrapped::shellcheck;
use cli_args::Action;
use cli_args::CliArgs;
use std::collections::HashSet;
use std::process::exit;
use structopt::StructOpt;

fn main() {
	let cli_args: CliArgs = CliArgs::from_args();
	rua_environment::prepare_environment(&cli_args);
	match &cli_args.action {
		Action::Info { ref target } => {
			info(target, false).unwrap();
		}
		Action::Install {
			asdeps,
			offline,
			target,
		} => {
			let paths = rua_paths::RuaPaths::initialize_paths();
			action_install::install(target, &paths, *offline, *asdeps);
		}
		Action::Builddir {
			offline,
			force,
			target,
		} => {
			let paths = rua_paths::RuaPaths::initialize_paths();
			action_builddir::action_builddir(target, &paths, *offline, *force);
		}
		Action::Search { target } => action_search::action_search(target),
		Action::Shellcheck { target } => {
			let result = shellcheck(target);
			result
				.map_err(|err| {
					eprintln!("{}", err);
					exit(1);
				})
				.ok();
		}
		Action::Tarcheck { target } => {
			tar_check::tar_check_unwrap(
				target,
				target.to_str().expect("target is not valid UTF-8"),
			);
			eprintln!("Finished checking package: {:?}", target);
		}
		Action::Upgrade {
			devel,
			printonly,
			ignored,
			packages,
		} => {
			let ignored_set = ignored
				.iter()
				.flat_map(|i| i.split(','))
				.collect::<HashSet<&str>>();
			let only_packages: HashSet<&str> = packages.iter().map(String::as_str).collect();
			let result = if *printonly {
				action_upgrade::upgrade_printonly(*devel, &ignored_set, &only_packages)
			} else {
				let paths = rua_paths::RuaPaths::initialize_paths();
				action_upgrade::upgrade_real(*devel, &paths, &ignored_set, &only_packages)
			};
			if let Err(e) = result {
				eprintln!("{}", e);
				exit(1);
			}
		}
	};
}
