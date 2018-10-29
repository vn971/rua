use std::collections::HashSet;
use std::io;
use std::process::Command;
use std::process::Stdio;
use std::collections::HashMap;
use std::path::PathBuf;


pub fn is_package_installed(package: &str) -> bool {
	Command::new("pacman").arg("-Qi").arg(&package)
		.stdout(Stdio::null()).stderr(Stdio::null()).status().unwrap().success()
}

pub fn is_package_installable(package: &str) -> bool {
	Command::new("pacman").arg("-Si").arg(&package)
		.stdout(Stdio::null()).stderr(Stdio::null()).status().unwrap().success()
}

// TODO: DRY

pub fn ensure_aur_packages_installed(mut packages: HashMap<String, PathBuf>) {
	while !packages.is_empty() {
		{
			let mut list = packages.iter().map(|(_name, path)| path.to_str().unwrap()).collect::<Vec<_>>();
			list.sort_unstable();
			eprintln!("Dependencies need to be installed:");
			eprintln!("\n    pacman -U --needed --asdeps {}\n", list.join(" "));
			eprint!("Enter S to `sudo` install it, or install manually and press M when done: ");
			let mut string = String::new();
			io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
			let string = string.trim().to_lowercase();
			if string == "s" {
				Command::new("sudo").arg("pacman").arg("-U").arg("--needed").arg("--asdeps")
					.args(&list).status().ok();
			}
		}
		packages.retain(|name, _path| !is_package_installed(name));
	}
}

pub fn ensure_pacman_packages_installed(mut packages: HashSet<String>) {
	while !packages.is_empty() {
		let mut list = packages.iter().map(|s| s.to_string()).collect::<Vec<_>>();
		list.sort_unstable();
		eprintln!("Pacman dependencies need to be installed:");
		eprintln!("\n    pacman -S --needed --asdeps {}\n", list.join(" "));
		eprint!("Enter S to `sudo` install it, or install manually and press M when done: ");
		let mut string = String::new();
		io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
		let string = string.trim().to_lowercase();
		if string == "s" {
			Command::new("sudo").arg("pacman").arg("-S").arg("--needed").arg("--asdeps")
				.args(&list).status().ok();
		}
		packages.retain(|name| !is_package_installed(name));
	}
}

