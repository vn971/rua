use crate::pacman;
use crate::rua_paths::RuaPaths;
use crate::tar_check;
use crate::wrapped;
use std::path::Path;
use std::path::PathBuf;

/// Build and install a package, see `crate::cli_args::Action::Builddir` for details
pub fn action_builddir(dir: &Option<PathBuf>, rua_paths: &RuaPaths, offline: bool, force: bool) {
	// Set `.` as default dir in case no build directory is provided.
	let dir = match dir {
		Some(path) => path,
		None => Path::new("."),
	};
	let dir = dir
		.canonicalize()
		.unwrap_or_else(|err| panic!("Cannot canonicalize path {:?}, {}", dir, err));
	let dir_str = dir
		.to_str()
		.unwrap_or_else(|| panic!("{}:{} Cannot parse CLI target directory", file!(), line!()));
	wrapped::build_directory(dir_str, rua_paths, offline, force);

	let srcinfo = wrapped::generate_srcinfo(dir_str, rua_paths).expect("Failed to obtain SRCINFO");
	let ver = srcinfo.version();
	let packages = srcinfo.pkgs().iter().map(|package| {
		let arch = if package.arch().contains(&*pacman::PACMAN_ARCH) {
			pacman::PACMAN_ARCH.to_string()
		} else {
			"any".to_string()
		};
		let file = format!(
			"{}-{}-{}{}",
			package.pkgname, ver, arch, rua_paths.makepkg_pkgext
		);
		let file = dir.join(file);
		(package.pkgname.clone(), file)
	});
	let packages: Vec<(String, PathBuf)> = packages.collect();

	for (_, file) in &packages {
		let file_str = file.to_str().expect("Builddir target has unvalid UTF-8");
		tar_check::tar_check(file, file_str).ok();
	}
	eprintln!("Package built and checked.");

	pacman::ensure_aur_packages_installed(packages, false);
}
