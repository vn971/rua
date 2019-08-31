use crate::alpm_impl::AlpmImpl;
use crate::alpm_wrapper::AlpmWrapper;
use crate::{action_install, terminal_util};
use directories::ProjectDirs;

pub fn upgrade(dirs: &ProjectDirs) {
	let alpm: AlpmImpl = crate::alpm_impl::new();
	let aur_pkgs = alpm.list_foreign_packages();
	let aur_pkgs_string = aur_pkgs.join(" ");
	eprintln!("You have the following packages outside of main repos installed:");
	eprintln!("{}", aur_pkgs_string);
	eprintln!();
	let mut up_to_date = Vec::new();
	let mut outdated = Vec::new();
	let mut unexistent = Vec::new();
	for pkg in aur_pkgs {
		let raur_ver = action_install::raur_info(&pkg).map(|p| p.version);
		if let Some(raur_ver) = raur_ver {
			let is_outdated = alpm.is_package_older_than(&pkg, &raur_ver);
			if is_outdated {
				outdated.push(pkg);
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
		for pkg in &outdated {
			eprintln!("{}", pkg);
		}
		eprintln!();
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
