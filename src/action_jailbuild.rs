use crate::rua_dirs::TARGET_SUBDIR;
use crate::{tar_check, wrapped};
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

pub fn action_jailbuild(offline: bool, target: PathBuf, dirs: &ProjectDirs) {
	let target_str = target
		.to_str()
		.unwrap_or_else(|| panic!("{}:{} Cannot parse CLI target directory", file!(), line!()));
	wrapped::build_directory(target_str, &dirs, offline);
	for file in fs::read_dir(TARGET_SUBDIR).expect("'target' directory not found") {
		tar_check::tar_check(
			&file
				.expect("Failed to open file for tar_check analysis")
				.path(),
		);
	}
	eprintln!(
		"Package built and checked in: {:?}",
		target.join(TARGET_SUBDIR)
	);
}
