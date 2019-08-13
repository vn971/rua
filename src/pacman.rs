use crate::terminal_util;

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::str;

use lazy_static::lazy_static;
use libalpm::Alpm;
use libalpm::SigLevel;

pub fn is_package_installed(alpm: &Alpm, name: &str) -> bool {
	alpm.local_db()
		.find_satisfier(name)
		.expect("Failed to access libalpm.find_satisfier")
		.map_or(false, |sat| sat.install_date().is_some())
}

pub fn is_package_installable(alpm: &Alpm, name: &str) -> bool {
	alpm.find_satisfier(name)
		.expect("Failed to access libalpm.find_satisfier")
		.is_some()
}

fn get_repository_list() -> Vec<String> {
	let cmd = Command::new("pacman-conf")
		.arg("--repo-list")
		.output()
		.expect("cannot get repository list: pacman-conf --repo-list");
	let output = String::from_utf8(cmd.stdout)
		.expect("Failed to get repo list from `pacman-conf --repo-list`");
	output.lines().map(ToOwned::to_owned).collect()
}

/// Create `Alpm` instance with no registered databases except local
fn create_local_alpm() -> Alpm {
	let alpm = Alpm::new("/", "/var/lib/pacman"); // default locations on arch linux
	let alpm = alpm.unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to initialize alpm library, {}",
			file!(),
			line!(),
			err
		)
	});
	alpm
}

pub fn create_alpm() -> Alpm {
	let alpm = create_local_alpm();
	for repo in get_repository_list() {
		alpm.register_sync_db(&repo, &SigLevel::default())
			.unwrap_or_else(|e| panic!("Failed to register {} in libalpm, {}", &repo, e));
	}
	alpm
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
				"\n    pacman {} --needed {}\n",
				base_args.join(" "),
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
			let string = terminal_util::console_get_line();
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
		let alpm = create_local_alpm();
		packages.retain(|(name, _)| !is_package_installed(&alpm, name));
	}
}

pub fn ensure_aur_packages_installed(packages: Vec<(String, PathBuf)>, is_dependency: bool) {
	if is_dependency {
		ensure_packages_installed(packages, &["-U", "--asdeps"]);
	} else {
		ensure_packages_installed(packages, &["-U"]);
	}
}

pub fn ensure_pacman_packages_installed(packages: HashSet<String>) {
	let mut map: Vec<(String, PathBuf)> = Vec::new();
	for package in packages {
		let path = Path::new(&package).to_path_buf();
		map.push((package, path));
	}
	ensure_packages_installed(map, &["-S", "--asdeps"]);
}

// Some old functions that invoke shelling below.
// Currently, using "libalpm" crate is preferred instead.
// These functions might get back in use should RUA-s move away from using libalpm (I don't know that yet).

//pub fn is_package_installable(package: &str) -> bool {
//	Command::new("pacman").arg("-Sddp").arg(&package)
//		.stdout(Stdio::null()).stderr(Stdio::null()).status()
//		.expect(&format!("Failed to determine if package {} is installable", package))
//		.success()
//}

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
		arch.trim().into()
	};
}
