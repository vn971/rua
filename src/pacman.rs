use crate::alpm_wrapper::AlpmWrapper;
use crate::terminal_util;
use indexmap::IndexSet;
use itertools::Itertools;
use lazy_static::lazy_static;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::str;

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
				"\n    pacman {} --needed {}\n",
				base_args
					.iter()
					.map(|f| terminal_util::escape_bash_arg(f))
					.collect_vec()
					.join(" "),
				list.join(" ")
			);
			if attempt == 0 {
				eprint!(
					"Enter S to `sudo` install it, or install manually and press M when done. "
				);
			} else {
				eprint!("Enter S to `sudo` install it, X to skip installation, ");
				eprint!("or install manually and enter M when done. ");
			}
			attempt += 1;
			let string = terminal_util::read_line_lowercase();
			if string == "s" {
				let exit_status = Command::new("sudo")
					.arg("pacman")
					.args(base_args)
					.arg("--needed")
					.args(&list)
					.status();
				if exit_status.map(|c| c.success()).unwrap_or(false) {
					break;
				}
			} else if string == "m" {
			} else if string == "x" {
				break;
			}
		}
		let alpm = crate::alpm_impl::new();
		packages.retain(|(name, _)| !alpm.is_package_installed(name));
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
	ensure_packages_installed(map, &["-S", "--asdeps"]);
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
