#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

mod action_builddir;
mod action_install;
mod action_search;
mod action_upgrade;
mod aur_rpc_utils;
mod cli_args;
mod git_utils;
mod pacman;
mod print_format;
mod print_package_info;
mod print_package_table;
mod reviewing;
mod rua_environment;
mod rua_files;
mod srcinfo_to_pkgbuild;
mod tar_check;
mod terminal_util;
mod wrapped;

use crate::cli_args::CLIColorType;
use crate::print_package_info::info;
use crate::wrapped::shellcheck;
use cli_args::{Action, CliArgs};
use directories::ProjectDirs;
use fs2::FileExt;
use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;

fn main() {
	let dirs = ProjectDirs::from("com.gitlab", "vn971", "rua")
		.expect("Failed to determine XDG directories");
	let config: CliArgs = CliArgs::from_args();
	match config.color {
		// see "colored" crate and referenced specs
		CLIColorType::auto => {
			env::remove_var("NOCOLOR");
			env::remove_var("CLICOLOR_FORCE");
			env::remove_var("CLICOLOR");
		}
		CLIColorType::never => {
			env::set_var("NOCOLOR", "1");
			env::remove_var("CLICOLOR_FORCE");
			env::set_var("CLICOLOR", "0");
		}
		CLIColorType::always => {
			env::remove_var("NOCOLOR");
			env::set_var("CLICOLOR_FORCE", "1");
			env::remove_var("CLICOLOR");
		}
	}
	rua_environment::prepare_environment(&dirs);
	let locked_file = File::open(dirs.config_dir()).unwrap_or_else(|err| {
		panic!(
			"Failed to open config dir {:?} for locking, {}",
			dirs.config_dir(),
			err
		);
	});
	locked_file.try_lock_exclusive().unwrap_or_else(|_| {
		eprintln!("Another RUA instance already running.");
		exit(2)
	});
	match config.action {
		Action::Info { ref target } => {
			info(target, false).unwrap();
		}
		Action::Install {
			asdeps,
			offline,
			target,
		} => {
			action_install::install(&target, &dirs, offline, asdeps);
		}
		Action::Builddir { offline, target } => {
			action_builddir::action_builddir(offline, target, &dirs)
		}
		Action::Search { target } => action_search::action_search(target),
		Action::Shellcheck { target } => {
			let result = shellcheck(&target.unwrap_or_else(|| PathBuf::from("./PKGBUILD")));
			result
				.map_err(|err| {
					eprintln!("{}", err);
					exit(1);
				})
				.ok();
		}
		Action::Tarcheck { target } => {
			tar_check::tar_check_unwrap(
				&target,
				target.to_str().expect("target is not valid UTF-8"),
			);
			eprintln!("Finished checking pachage: {:?}", target);
		}
		Action::Upgrade { devel } => {
			action_upgrade::upgrade(&dirs, devel);
		}
	};
}
