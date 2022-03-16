use crate::alpm_wrapper::new_alpm_wrapper;
use crate::rua_environment;
use crate::terminal_util;
use indexmap::IndexSet;
use itertools::Itertools;
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::str;

pub fn get_ignored_packages() -> Result<HashSet<String>, String> {
	let command = Command::new("pacman-conf")
		.arg("IgnorePkg")
		.output()
		.map_err(|_| "cannot execute pacman-conf IgnorePkg")?;
	let output = String::from_utf8(command.stdout)
		.map_err(|err| format!("Failed to parse output of pacman-conf IgnorePkg, {}", err))?;
	Ok(output.lines().map(ToOwned::to_owned).collect())
}

fn ensure_packages_installed(mut packages: Vec<(String, PathBuf)>, base_args: &[&str]) {
	let mut attempt = 0;
	while !packages.is_empty() {
		{
			let mut list = packages
				.iter()
				.map(|(_name, path)| {
					path.to_str().unwrap_or_else(|| {
						panic!("{}:{} cannot parse package name", file!(), line!())
					})
				})
				.collect::<Vec<_>>();
			list.sort_unstable();
			eprintln!("Packages need to be installed:");
			eprintln!(
				"\n    pacman {} -- {}\n",
				base_args.join(" "),
				list.iter()
					.map(|p| terminal_util::escape_bash_arg(p)) // this is only printing. rua does not use bash to install packages
					.collect_vec()
					.join(" ")
			);
			if attempt == 0 {
				eprint!(
					"Enter S to `{}` install it, or install manually and press M when done. ",
					rua_environment::sudo_command()
				);
			} else {
				eprint!(
					"Enter S to `{}` install it, X to skip installation, ",
					rua_environment::sudo_command()
				);
				eprint!("or install manually and enter M when done. ");
			}
			attempt += 1;
			let string = terminal_util::read_line_lowercase();
			if string == "s" {
				let exit_status = Command::new(rua_environment::sudo_command())
					.arg("pacman")
					.args(base_args)
					.arg("--")
					.args(&list)
					.status();
				if exit_status.map(|c| c.success()).unwrap_or(false) {
					break;
				}
			} else if &string == "m" {
			} else if &string == "x" {
				break;
			} else {
				continue;
			}
		}
		let alpm = new_alpm_wrapper();
		packages.retain(|(name, _)| {
			!alpm
				.is_installed(name)
				.expect("Failed to check install status for a package")
		});
	}
}

pub fn ensure_aur_packages_installed(packages: Vec<(String, PathBuf)>, is_dependency: bool) {
	if is_dependency {
		ensure_packages_installed(packages, &["-U", "--asdeps"]);
	} else {
		ensure_packages_installed(packages, &["-U"]);
	}
}

pub fn ensure_pacman_packages_installed(packages: IndexSet<String>) {
	let mut map: Vec<(String, PathBuf)> = Vec::new();
	for package in packages {
		let path = Path::new(&package).to_path_buf();
		map.push((package, path));
	}
	ensure_packages_installed(map, &["-S", "--asdeps", "--needed"]);
}

// Architecture as defined in the local pacman configuration
lazy_static! {
	pub static ref PACMAN_ARCH: String = {
		let process_output = Command::new("pacman-conf").arg("architecture").output()
			.expect("Failed to get system architecture via pacman-conf");
		if !process_output.status.success() {
			panic!("pacman-conf call failed with an non-zero status");
		}
		let arch = str::from_utf8(&process_output.stdout).expect("Found non-utf8 in pacman-conf output");
		// Trim away the "/n" & convert into a String
		arch.trim().to_string()
	};
}
