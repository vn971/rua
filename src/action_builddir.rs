use crate::pacman;
use crate::rua_environment;
use crate::rua_paths::RuaPaths;
use crate::tar_check;
use crate::terminal_util;
use crate::wrapped;
use itertools::Itertools;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

pub fn action_builddir(dir: &Option<PathBuf>, rua_paths: &RuaPaths, offline: bool, force: bool) {
	// Set `.` as default dir in case no build directory is provided.
	let dir = match dir {
		Some(path) => &path,
		None => Path::new("."),
	};
	let dir = dir
		.canonicalize()
		.unwrap_or_else(|err| panic!("Cannot canonicalize path {:?}, {}", dir, err));
	let dir_str = dir
		.to_str()
		.unwrap_or_else(|| panic!("{}:{} Cannot parse CLI target directory", file!(), line!()));
	wrapped::build_directory(dir_str, &rua_paths, offline, force);

	let srcinfo = wrapped::generate_srcinfo(dir_str, &rua_paths).expect("Failed to obtain SRCINFO");
	let ver = srcinfo.version();
	let archive_names = srcinfo.pkgs.iter().map(|package| {
		let arch = if package.arch.contains(&*pacman::PACMAN_ARCH) {
			pacman::PACMAN_ARCH.to_string()
		} else {
			"any".to_string()
		};
		format!(
			"{}-{}-{}{}",
			package.pkgname, ver, arch, rua_paths.makepkg_pkgext
		)
	});
	let archive_names = archive_names.collect::<Vec<_>>();

	for archive_name in &archive_names {
		let file = dir.join(archive_name);
		let file_str = file.to_str().expect("Builddir target has unvalid UTF-8");
		tar_check::tar_check(&file, file_str).ok();
	}

	let archive_escaped = archive_names
		.iter()
		.map(|archive| {
			let canon = dir.join(archive).canonicalize();
			let canon = canon.unwrap_or_else(|err| {
				panic!(
					"Failed to canonicalize path for archive {} in directory {:?}, {}",
					archive, dir, err
				)
			});
			let canon = canon.to_str().expect("Builddir target has unvalid UTF-8");
			terminal_util::escape_bash_arg(canon) // this is only printing. rua does not use bash to install packages
		})
		.collect_vec()
		.join(" ");

	eprintln!("Package built and checked. Do you want to install?");
	eprintln!("    pacman -U -- {}", archive_escaped);
	loop {
		eprint!(
			"[S]={} install, [X]=skip installation. ",
			rua_environment::sudo_command()
		);
		let user_input = terminal_util::read_line_lowercase();
		if &user_input == "s" {
			let exit_status = Command::new(rua_environment::sudo_command())
				.args(&["pacman", "-U", "--"])
				.args(&archive_names)
				.status();
			if exit_status.map(|c| c.success()).unwrap_or(false) {
				break;
			} else {
				eprintln!("Pacman installation command failed")
			}
		} else if &user_input == "x" {
			break;
		}
	}
}
