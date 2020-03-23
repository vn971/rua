use crate::pacman;
use crate::rua_environment::RuaEnv;
use crate::tar_check;
use crate::wrapped;
use std::path::Path;
use std::path::PathBuf;

pub fn action_builddir(dir: Option<PathBuf>, rua_env: &RuaEnv, offline: bool, force: bool) {
	// Set `.` as default dir in case no build directory is provided.
	let dir = dir.as_deref().unwrap_or_else(|| Path::new("."));

	let sandbox = wrapped::Sandbox::new(&rua_env.paths);

	wrapped::build_directory(sandbox, &dir, offline, force);

	let srcinfo = wrapped::generate_srcinfo(sandbox, &dir).unwrap();
	let ver = srcinfo.version();
	let archive_names = srcinfo.pkgs.iter().map(|package| {
		let arch = if package.arch.contains(&*pacman::PACMAN_ARCH) {
			pacman::PACMAN_ARCH.to_string()
		} else {
			"any".to_string()
		};
		format!("{}-{}-{}{}", package.pkgname, ver, arch, rua_env.pkgext)
	});

	for archive_name in archive_names {
		tar_check::tar_check_unwrap(&dir.join(archive_name));
	}

	eprintln!("Package built and checked in: {}", dir.display());
	eprintln!("If you want to install the built artifacts, do it manually.");
}
