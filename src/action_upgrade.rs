use crate::pacman;
use crate::{action_install, terminal_util};
use directories::ProjectDirs;
use version_compare::CompOp;
use version_compare::VersionCompare;

pub fn upgrade(dirs: &ProjectDirs) {
	let alpm = pacman::create_alpm();
	let pkg_cache = alpm
		.localdb()
		.pkgs()
		.expect("Could not get alpm.localdb().pkgs() packages");
	let aur_pkgs = pkg_cache
		.filter(|pkg| !pacman::is_package_installable(&alpm, pkg.name()))
		.map(|pkg| (pkg.name(), pkg.version().to_string()))
		.collect::<Vec<_>>();
	let aur_pkgs_string = aur_pkgs
		.iter()
		.map(|(pkg, _ver)| *pkg)
		.collect::<Vec<_>>()
		.join(" ");
	eprintln!("You have the following packages outside of main repos installed:");
	eprintln!("{}", aur_pkgs_string);
	eprintln!();
	let mut up_to_date = Vec::new();
	let mut outdated = Vec::new();
	let mut unexistent = Vec::new();
	for (pkg, local_ver) in aur_pkgs {
		let raur_ver = action_install::raur_info(pkg).map(|p| p.version);
		if let Some(raur_ver) = raur_ver {
			let is_outdated = VersionCompare::compare_to(&local_ver, &raur_ver, &CompOp::Lt)
				.unwrap_or_else(|_| {
					panic!(
						"Could not compare local->upstream versions: {}->{}",
						local_ver, raur_ver
					)
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
