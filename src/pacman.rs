use std::collections::HashSet;
use std::io;
use std::process::Command;
use std::process::Stdio;
use std::collections::HashMap;
use std::path::PathBuf;
use std::path::Path;


pub fn is_package_installed(package: &str) -> bool {
	Command::new("pacman").arg("-T").arg(&package)
		.stdout(Stdio::null()).stderr(Stdio::null()).status()
		.expect(&format!("Failed to determine if package {} is installed", package)).success()
}

pub fn is_package_installable(package: &str) -> bool {
	Command::new("pacman").arg("-Si").arg(&package)
		.stdout(Stdio::null()).stderr(Stdio::null()).status()
		.expect(&format!("Failed to determine if package {} is installed", package)).success()
}


fn ensure_packages_installed(mut packages: HashMap<String, PathBuf>, base_args: &[&str]) {
	let mut first_run = true;
	while !packages.is_empty() {
		{
			let mut list = packages.iter().map(|(_name, path)| path.to_str().unwrap()).collect::<Vec<_>>();
			list.sort_unstable();
			eprintln!("Packages need to be installed:");
			eprintln!("\n    pacman {} --needed {}\n", base_args.join(" "), list.join(" "));
			eprint!("Enter S to `sudo` install it, or install manually and press M when done. ");
			if !first_run {
				eprint!("Press Z to skip pacman verification. ");
			}
			let mut string = String::new();
			io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
			let string = string.trim().to_lowercase();
			if string == "s" {
				Command::new("sudo").arg("pacman").args(base_args).arg("--needed")
					.args(&list).status().ok();
			} else if string == "z" && !first_run {
				break;
			}
		}
		packages.retain(|name, _path| !is_package_installed(name));
		first_run = false;
	}
}

pub fn ensure_aur_packages_installed(packages: HashMap<String, PathBuf>, is_dependency: bool) {
	if is_dependency {
		ensure_packages_installed(packages, &["-U", "--asdeps"]);
	} else {
		ensure_packages_installed(packages, &["-U"]);
	}
}

pub fn ensure_pacman_packages_installed(packages: HashSet<String>) {
	let mut map: HashMap<String, PathBuf> = HashMap::new();
	for package in packages {
		let path = Path::new(&package).to_path_buf();
		map.insert(package, path);
	}
	ensure_packages_installed(map, &["-S"]);
}
