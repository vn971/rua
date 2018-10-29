use directories::ProjectDirs;
use regex::Regex;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::process::Output;


fn assert_command_success(command: &Output) {
	assert!(command.status.success(),
		"Command failed with exit code {:?}\nStderr: {}\nStdout: {}",
		command.status.code(),
		String::from_utf8_lossy(&command.stderr),
		String::from_utf8_lossy(&command.stdout),
	);
}

pub fn download_if_absent(name: &str, dirs: &ProjectDirs) {
	let valid_name_regexp = Regex::new(r"[a-zA-Z][a-zA-Z._-]*").unwrap();
	assert!(valid_name_regexp.is_match(name), "unexpected package name {}", name);
	// TODO: download new version, with some caching
	fs::create_dir_all(dirs.cache_dir().join(name)).unwrap();
	env::set_current_dir(dirs.cache_dir().join(name)).unwrap();
	if !Path::new("build").exists() && !Path::new("target").exists() {
		let dir = "aur.tmp";
		fs::remove_dir_all(dir).ok();
		let git_http_ref = format!("https://aur.archlinux.org/{}.git", name);
		let command = Command::new("git").args(&["clone", &git_http_ref, dir]).output().unwrap();
		assert_command_success(&command);
		env::set_current_dir(&dir).unwrap();
		assert!(Path::new("PKGBUILD").exists(), "PKGBUILD not found for package {}. \
			Does this package really exist in AUR?", name);
		loop {
			eprint!("Downloaded {}. Show PKGBUILD? Y=yes, I=run shell to inspect, O=ok, use the file: ", name);
			let mut string = String::new();
			io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
			let string = string.trim().to_lowercase();

			if string == "y" {
				Command::new(env::var("PAGER").unwrap_or("less".to_string())).arg("PKGBUILD").status().ok();
			} else if string == "i" {
				eprintln!("Exit the shell with `logout` or Ctrl-D...");
				Command::new(env::var("SHELL").unwrap_or("bash".to_string())).status().ok();
			} else if string == "o" {
				break;
			}
		}
		env::set_current_dir("..").unwrap();
		fs::rename(dir, "build").unwrap();
	}
}
