use crate::rua_dirs::PREFETCH_DIR;
use crate::rua_dirs::REVIEWED_BUILD_DIR;
use crate::terminal_util;

use std::{env, fs};

use directories::ProjectDirs;
use rm_rf;

pub fn download(name: &str, dirs: &ProjectDirs) {
	let path = dirs.config_dir().join(PREFETCH_DIR);
	let fetch = aur_fetch::Handle::with_combined_cache_dir(path);
	fetch.download(&[name]).unwrap();

	assert!(
		fetch.clone_dir.join(name).join(".SRCINFO").exists(),
		"Repository {} does not have an SRCINFO file. Does this package exist in AUR?",
		name
	);
}

pub fn review_repo(name: &str, dirs: &ProjectDirs) {
	let path = dirs.config_dir().join(PREFETCH_DIR);
	let reviewed_path = dirs.cache_dir().join(REVIEWED_BUILD_DIR);
	let fetch = aur_fetch::Handle::with_combined_cache_dir(path);
	let pkgs = &[name];

	let fetched = fetch.download(pkgs).unwrap();
	let to_merge = fetch.needs_merge(&fetched).unwrap();
	let pkgbuild = fetch.clone_dir.join(name).join("PKGBUILD");

	env::set_current_dir(fetch.clone_dir.join(&name))
		.unwrap_or_else(|err| panic!("Failed to cd into file view for {}, {}", name, err));

	loop {
		eprint!(
			"Verifying package {}. [V]=view PKGBUILD, [D]=view diff, \
			 [E]=edit PKGBUILD, [I]=run shell to inspect, [O]=ok: ",
			name
		);
		let string = terminal_util::console_get_line();

		if string == "v" {
			terminal_util::run_env_command("PAGER", "less", &[&pkgbuild.to_string_lossy()]);
		} else if string == "d" {
			fetch.print_diff(name).unwrap();
		} else if string == "e" {
			terminal_util::run_env_command("EDITOR", "nano", &[&pkgbuild.to_string_lossy()]);
		} else if string == "i" {
			eprintln!("Exit the shell with `logout` or Ctrl-D...");
			terminal_util::run_env_command("SHELL", "bash", &[]);
		} else if string == "o" {
			break;
		}
	}
	rm_rf::force_remove_all(&reviewed_path, true).unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to clean build dir {:?}, {}",
			file!(),
			line!(),
			REVIEWED_BUILD_DIR,
			err,
		)
	});
	fetch.merge(&to_merge).unwrap();
	fs::create_dir_all(&reviewed_path).unwrap();
	fs::rename(fetch.clone_dir.join(name), &reviewed_path.join(name)).unwrap_or_else(|err| {
		panic!(
			"Failed to move temporary directory '{}' to 'build', {}",
			PREFETCH_DIR, err,
		)
	});
}
