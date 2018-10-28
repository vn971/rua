use std::collections::HashSet;
use std::io;
use std::process::Command;
use std::process::Stdio;


pub fn is_package_installed(package: &str) -> bool {
	Command::new("pacman").arg("-Qi").arg(&package)
		.stdout(Stdio::null()).stderr(Stdio::null()).status().unwrap().success()
}

pub fn is_package_installable(package: &str) -> bool {
	Command::new("pacman").arg("-Si").arg(&package)
		.stdout(Stdio::null()).stderr(Stdio::null()).status().unwrap().success()
}


pub fn ensure_pacman_packages_installed(pacman_deps: &mut HashSet<String>) {
	while !pacman_deps.is_empty() {
		let mut deps_list = pacman_deps.iter().map(|s| s.to_string()).collect::<Vec<_>>();
		deps_list.sort_unstable();
		eprintln!("Pacman dependencies need to be installed:");
		eprintln!("\n    pacman -S --needed --asdeps {}\n", deps_list.join(" "));
		eprint!("Enter S to `sudo` install it, or install manually and press M when done: ");
		let mut string = String::new();
		io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
		let string = string.trim().to_lowercase();
		if string == "s" {
			Command::new("sudo").arg("pacman").arg("-S").arg("--needed").arg("--asdeps")
				.args(&deps_list).status().ok();
		}
		for dep in &deps_list {
			if is_package_installed(&dep) {
				pacman_deps.remove(dep);
			}
		}
	}
}

