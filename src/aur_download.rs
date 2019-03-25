use crate::util;

use std::path::Path;
use std::process::{Command, Output};
use std::{env, fs};

use crate::rua_dirs::PREFETCH_DIR;
use crate::rua_dirs::REVIEWED_BUILD_DIR;
use directories::ProjectDirs;
use lazy_static::lazy_static;
use regex::Regex;
use rm_rf;

fn assert_command_success(command: &Output) {
	assert!(
		command.status.success(),
		"Command failed with exit code {:?}\nStderr: {}\nStdout: {}",
		command.status.code(),
		String::from_utf8_lossy(&command.stderr),
		String::from_utf8_lossy(&command.stdout),
	);
}

pub fn fresh_download(name: &str, dirs: &ProjectDirs) {
	lazy_static! {
		static ref NAME_REGEX: Regex = Regex::new(r"[a-zA-Z][a-zA-Z._-]*")
			.unwrap_or_else(|_| panic!("{}:{} Failed to parse regexp", file!(), line!()));
	}
	assert!(
		NAME_REGEX.is_match(name),
		"{}:{} unexpected package name {}",
		file!(),
		line!(),
		name
	);
	let path = dirs.cache_dir().join(name).join(PREFETCH_DIR);
	rm_rf::force_remove_all(&path, true).unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to clean cache dir {:?}, {}",
			file!(),
			line!(),
			path,
			err,
		)
	});
	fs::create_dir_all(dirs.cache_dir().join(name))
		.unwrap_or_else(|err| panic!("Failed to create cache dir for {}, {}", name, err));
	env::set_current_dir(dirs.cache_dir().join(name))
		.unwrap_or_else(|err| panic!("Failed to cd into {}, {}", name, err));
	let git_http_ref = format!("https://aur.archlinux.org/{}.git", name);
	let command = Command::new("git")
		.args(&["clone", &git_http_ref, PREFETCH_DIR])
		.output()
		.unwrap_or_else(|err| panic!("Failed to git-clone repository {}, {}", name, err));
	assert_command_success(&command);
	assert!(
		Path::new(PREFETCH_DIR).join(".SRCINFO").exists(),
		"Repository {} does not have an SRCINFO file. Does this package exist in AUR?",
		name
	);
}

pub fn review_repo(name: &str, dirs: &ProjectDirs) {
	env::set_current_dir(dirs.cache_dir().join(name).join(PREFETCH_DIR))
		.unwrap_or_else(|err| panic!("Failed to cd into build dir for {}, {}", name, err));
	loop {
		eprint!(
			"Verifying package {}. [V]=view PKGBUILD, [E]=edit PKGBUILD, \
			 [I]=run shell to inspect, [O]=ok, use package: ",
			name
		);
		let string = util::console_get_line();

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
	env::set_current_dir("..").unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to move to parent repo after review, {}",
			file!(),
			line!(),
			err,
		)
	});
	rm_rf::force_remove_all(REVIEWED_BUILD_DIR, true).unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to clean build dir {:?}, {}",
			file!(),
			line!(),
			REVIEWED_BUILD_DIR,
			err,
		)
	});
	fs::rename(PREFETCH_DIR, REVIEWED_BUILD_DIR).unwrap_or_else(|err| {
		panic!(
			"Failed to move temporary directory '{}' to 'build', {}",
			PREFETCH_DIR, err,
		)
	});
}
