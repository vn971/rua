use crate::git_utils;
use crate::rua_files::RuaDirs;
use crate::terminal_util;
use crate::wrapped;
use log::debug;
use std::path::Path;

pub fn review_repo(dir: &Path, pkgbase: &str, dirs: &RuaDirs) {
	let mut dir_contents = dir.read_dir().unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to read directory for reviewing, {}",
			file!(),
			line!(),
			err
		)
	});
	if dir_contents.next().is_none() {
		debug!("Directory {:?} is empty, using git clone", &dir);
		git_utils::init_repo(pkgbase, &dir);
	} else {
		debug!("Directory {:?} is not empty, fetching new version", &dir);
		git_utils::fetch(&dir);
	}

	let build_dir = dirs.build_dir(pkgbase);
	if build_dir.exists() && git_utils::is_upstream_merged(&dir) {
		eprintln!("WARNING: your AUR repo is up-to-date.");
		eprintln!(
			"If you continue, the build directory will be removed and the build will be re-run."
		);
		eprintln!("If you don't want that, consider resolving the situation manually,");
		eprintln!("for example:    rua builddir {:?}", build_dir);
		eprintln!();
	}

	loop {
		eprintln!("\nReviewing {:?}. ", dir);
		let is_upstream_merged = git_utils::is_upstream_merged(&dir);
		let identical_to_upstream = is_upstream_merged && git_utils::identical_to_upstream(dir);
		if is_upstream_merged {
			eprint!("[S]=run shellcheck on PKGBUILD, ");
			if identical_to_upstream {
				eprint!("[D]=(identical to upstream, empty diff), ");
			} else {
				eprint!("[D]=view your changes, ");
			};
		} else {
			eprint!("[D]=view upstream changes since your last review, ");
			eprint!("[M]=accept/merge upstream changes, ");
			eprint!("[S]=(shellcheck not available until you merge), ");
		}
		eprint!("[T]=run shell to edit/inspect, ");
		if is_upstream_merged {
			eprint!("[O]=ok, use package ");
		} else {
			eprint!("[O]=(cannot use the package until you merge) ");
		}
		let string = terminal_util::read_line_lowercase();

		if string == "t" {
			eprintln!("Exit the shell with `logout` or Ctrl-D...");
			terminal_util::run_env_command(&dir, "SHELL", "bash", &[]);
		} else if string == "s" && is_upstream_merged {
			match wrapped::shellcheck(&dir.join("PKGBUILD")) {
				Err(err) => eprintln!("{}", err),
				Ok(_) => eprintln!("shellcheck found no problems"),
			};
		} else if string == "d" && is_upstream_merged {
			git_utils::show_upstream_diff(dir, false);
		} else if string == "d" && !is_upstream_merged {
			git_utils::show_upstream_diff(dir, true);
		} else if string == "m" && !is_upstream_merged {
			git_utils::merge_upstream(dir);
		} else if string == "o" && is_upstream_merged {
			break;
		}
	}
}
