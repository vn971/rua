use libalpm::Db;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;


fn ensure_packages_installed(
	mut packages: HashMap<String, PathBuf>,
	base_args: &[&str],
	alpm_db: &Db
) {
	while !packages.is_empty() {
		{
			let mut list = packages.iter().map(|(_name, path)| path.to_str().unwrap()).collect::<Vec<_>>();
			list.sort_unstable();
			eprintln!("Packages need to be installed:");
			eprintln!("\n    pacman {} --needed {}\n", base_args.join(" "), list.join(" "));
			eprint!("Enter S to `sudo` install it, or install manually and press M when done. ");
			let mut string = String::new();
			io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
			let string = string.trim().to_lowercase();
			if string == "s" {
				Command::new("sudo").arg("pacman").args(base_args).arg("--needed")
					.args(&list).status().ok();
			} else if string == "m" {
				break;
			}
		}
		packages.retain(|name, _| alpm_db.find_satisfier(name)
			.expect("Failed to access libalpm.find_satisfier")
			.expect(&format!("satisfier for {} no longer exists", name))
			.install_date().is_none()
		);
	}
}

pub fn ensure_aur_packages_installed(packages: Vec<PathBuf>, is_dependency: bool, alpm_db: &Db) {
	let mut map: HashMap<String, PathBuf> = HashMap::new();
	for package in packages {
		let path = Path::new(&package).to_path_buf();
		map.insert(package.to_str().unwrap().to_owned(), path);
	}
	if is_dependency {
		ensure_packages_installed(map, &["-U", "--asdeps"], alpm_db);
	} else {
		ensure_packages_installed(map, &["-U"], alpm_db);
	}
}

pub fn ensure_pacman_packages_installed(packages: HashSet<String>, alpm_db: &Db) {
	let mut map: HashMap<String, PathBuf> = HashMap::new();
	for package in packages {
		let path = Path::new(&package).to_path_buf();
		map.insert(package, path);
	}
	ensure_packages_installed(map, &["-S", "--asdeps"], alpm_db);
}
