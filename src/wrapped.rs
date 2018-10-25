// Commands that are run inside "bubblewrap" jail

use directories::ProjectDirs;
use regex::Regex;
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;


fn wrap_yes_internet(dirs: &ProjectDirs) -> Command {
	Command::new(dirs.config_dir().join("wrap.sh"))
}
fn wrap_no_internet(dirs: &ProjectDirs) -> Command {
	let mut command = Command::new(dirs.config_dir().join("wrap.sh"));
	command.arg("--unshare-net");
	command
}

pub fn get_deps(name: &str, dirs: &ProjectDirs) -> Vec<String> {
	let dir = dirs.cache_dir().join("build").join(name);
	let dir = dir.to_str().unwrap();
	env::set_current_dir(dir).unwrap();
	let mut command = wrap_no_internet(dirs);
	command.args(&["--ro-bind", dir, dir]);
	command.args(&["bash", "--restricted", dirs.config_dir().join("get_deps.sh").to_str().unwrap()]);
	let command = command.stderr(Stdio::inherit()).output().unwrap();
	String::from_utf8_lossy(&command.stdout).trim().split(' ').map(|s| s.to_string()).collect()
}

fn download_sources(dirs: &ProjectDirs) {
	let current_dir = env::current_dir().unwrap();
	let current_dir = current_dir.to_str().unwrap();
	let mut command = wrap_yes_internet(dirs);
	command.args(&["--bind", current_dir, current_dir]);
	command.args(&["makepkg", "--noprepare", "--nobuild"]);
	let command = command.status().unwrap();
	assert!(command.success(), "Failed to download PKGBUILD sources");
}

fn do_build(dirs: &ProjectDirs) {
	let dir = env::current_dir().unwrap();
	let mut command = wrap_no_internet(dirs);
	command.args(&["--bind", dir.to_str().unwrap(), dir.to_str().unwrap()]);
	command.args(&["makepkg", "--force"]);
	let command = command.status().unwrap();
	assert!(command.success(), "Failed to download PKGBUILD sources");
}

pub fn jail_build(dir: &str, project_dirs: &ProjectDirs) {
	env::set_current_dir(dir).expect(format!("cannot build in directory {}", dir).as_str());
	env::set_var("PKGDEST", Path::new(".").canonicalize().unwrap().join("target"));
	download_sources(project_dirs);
	do_build(project_dirs);
}


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
	if !Path::new(&dirs.cache_dir().join("build").join(name)).exists() {
		env::set_current_dir(dirs.cache_dir().join("build")).unwrap();
		let dir = format!("{}.tmp", name);
		fs::remove_dir_all(&dir).ok();
		let git_http_ref = format!("https://aur.archlinux.org/{}.git", name);
		let command = Command::new("git").args(&["clone", &git_http_ref, &dir]).output().unwrap();
		assert_command_success(&command);
		env::set_current_dir(&dir).unwrap();
		assert!(Path::new("PKGBUILD").exists(), "PKGBUILD not found for package {}. \
			Does this package really exist in AUR?", name);
		loop {
			let mut string = String::new();
			eprint!("Downloaded {}. Show PKGBUILD? Y=yes, I=run shell to inspect, O=ok, use the file: ", name);
			io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
			let string = string.trim().to_lowercase();

			if string == "y" {
				Command::new("less").arg("PKGBUILD").status().ok();
			} else if string == "i" {
				Command::new(env::var("SHELL").unwrap_or("bash".to_string())).status().ok();
			} else if string == "o" {
				break;
			}
		}
		env::set_current_dir("..").unwrap();
		fs::rename(dir, name).unwrap();
	}
}
