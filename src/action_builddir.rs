use crate::rua_files::RuaDirs;
use crate::tar_check;
use crate::wrapped;
use std::path::PathBuf;

pub fn action_builddir(offline: bool, dir: PathBuf, dirs: &RuaDirs) {
	let dir = dir
		.canonicalize()
		.unwrap_or_else(|err| panic!("Cannot canonicalize path {:?}, {}", dir, err));
	let dir_str = dir
		.to_str()
		.unwrap_or_else(|| panic!("{}:{} Cannot parse CLI target directory", file!(), line!()));
	wrapped::build_directory(dir_str, &dirs, offline);
	for file in dir
		.read_dir()
		.expect("cannot read directory with built package")
	{
		let file = file
			.expect("Failed to open file for tar_check analysis")
			.path();
		let file_str = file.to_str().expect("Builddir target has unvalid UTF-8");
		tar_check::tar_check(&file, file_str).ok();
	}
	eprintln!("Package built and checked in: {}", dir_str);
	eprintln!("If you want to install the built artifacts, do it manually.");
}
