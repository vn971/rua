use directories::ProjectDirs;
use regex::Regex;
use std::env;
use std::fs;
use std::io;
use std::process::Command;
use std::process::Output;
use util;

pub const PREFETCH_DIR: &str = "aur.tmp";


fn assert_command_success(command: &Output) {
	assert!(command.status.success(),
		"Command failed with exit code {:?}\nStderr: {}\nStdout: {}",
		command.status.code(),
		String::from_utf8_lossy(&command.stderr),
		String::from_utf8_lossy(&command.stdout),
	);
}


pub fn fresh_download(name: &str, dirs: &ProjectDirs) {
	lazy_static! {
		static ref name_regexp: Regex = Regex::new(r"[a-zA-Z][a-zA-Z._-]*").unwrap();
	}
	assert!(name_regexp.is_match(name), "unexpected package name {}", name);
	let path = dirs.cache_dir().join(name);
	if path.exists() {
		fs::remove_dir_all(&path).expect(&format!("Failed to clean cache dir {:?}", path));
	}
	fs::create_dir_all(dirs.cache_dir().join(name)).expect(&format!("Failed to create cache dir for {}", name));
	let git_http_ref = format!("https://aur.archlinux.org/{}.git", name);
	let command = Command::new("git").args(&["clone", &git_http_ref, PREFETCH_DIR])
		.output().expect(&format!("Failed to git-clone repository {}", name));
	assert_command_success(&command);
}


pub fn review_repo(name: &str, dirs: &ProjectDirs) {
	env::set_current_dir(dirs.cache_dir().join(name).join(PREFETCH_DIR)).expect(&format!("Faild to cd into build dir for {}", name));
	loop {
		eprint!("Verifying package {}. V=view PKGBUILD, E=edit PKGBUILD, \
		I=run shell to inspect, O=ok, use package: ", name);
		let mut string = String::new();
		io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
		let string = string.trim().to_lowercase();

		if string == "v" {
			util::run_env_command("PAGER", "less", &["PKGBUILD"]);
		} else if string == "e" {
			util::run_env_command("EDITOR", "nano", &["PKGBUILD"]);
		} else if string == "i" {
			eprintln!("Exit the shell with `logout` or Ctrl-D...");
			util::run_env_command("SHELL", "bash", &[]);
		} else if string == "o" {
			break;
		}
	}
	env::set_current_dir("..").unwrap();
	fs::rename(PREFETCH_DIR, "build")
		.expect(&format!("Failed to move temporary directory '{}' to 'build'", PREFETCH_DIR));
}
