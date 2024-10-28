use crate::git_utils;
use crate::rua_paths::RuaPaths;
use crate::terminal_util;
use crate::wrapped;
use colored::Colorize;
use log::debug;
use std::path::Path;

pub fn review_repo(dir: &Path, pkgbase: &str, rua_paths: &RuaPaths, autobuild: bool) {
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
		git_utils::init_repo(pkgbase, dir, rua_paths);
	} else {
		debug!("Directory {:?} is not empty, fetching new version", &dir);
		git_utils::fetch(dir, rua_paths);
	}

	let build_dir = rua_paths.build_dir(pkgbase);
	if build_dir.exists() && git_utils::is_upstream_merged(dir, rua_paths) {
		eprintln!("WARNING: your AUR repo is up-to-date.");
		eprintln!(
			"If you continue, the build directory will be removed and the build will be re-run."
		);
		eprintln!("If you don't want that, consider resolving the situation manually,");
		let build_dir = terminal_util::escape_bash_arg(
			build_dir
				.to_str()
				.unwrap_or_else(|| panic!("Failed to stringify build directory {:?}", build_dir)),
		);
		eprintln!("for example:    rua builddir {}", build_dir);
		eprintln!();
	}

	loop {
		eprintln!("\nReviewing {:?}. ", dir);
		let is_upstream_merged = git_utils::is_upstream_merged(dir, rua_paths);
		let identical_to_upstream =
			is_upstream_merged && git_utils::identical_to_upstream(dir, rua_paths);
		if is_upstream_merged {
			eprint!(
				"{}{}, ",
				"[S]".bold().green(),
				"=run shellcheck on PKGBUILD".green()
			);
			if identical_to_upstream {
				eprint!("{}, ", "[D]=(identical to upstream, empty diff)".dimmed());
			} else {
				eprint!("{}{}, ", "[D]".bold().green(), "=view your changes".green());
			};
		} else {
			eprint!(
				"{}{}, ",
				"[D]".bold().green(),
				"=view upstream changes since your last review".green()
			);
			eprint!(
				"{}{}, ",
				"[M]".bold().yellow(),
				"=accept/merge upstream changes".yellow()
			);
			eprint!(
				"{}, ",
				"[S]=(shellcheck not available until you merge)".dimmed()
			);
		}
		eprint!(
			"{}{}, ",
			"[T]".bold().cyan(),
			"=run shell to edit/inspect".cyan()
		);
		if is_upstream_merged {
			eprint!("{}{}. ", "[O]".bold().red(), "=ok, use package".red());
		} else {
			eprint!(
				"{}",
				"[O]=(cannot use the package until you merge) ".dimmed()
			);
		}

		let user_input = if autobuild {
			if is_upstream_merged {
				eprintln!("\n{} {}", "Autobuild:".italic(), "[O]".italic().red());
				"o".to_string()
			} else {
				eprintln!("\n{} {}", "Autobuild:".italic(), "[M]".italic().yellow());
				"m".to_string()
			}
		} else {
			terminal_util::read_line_lowercase()
		};
		if &user_input == "t" {
			eprintln!("Changes that you make will be merged with upstream updates in future.");
			eprintln!("Exit the shell with `logout` or Ctrl-D...");
			terminal_util::run_env_command(dir, "SHELL", "bash", &[]);
		} else if &user_input == "s" && is_upstream_merged {
			if let Err(err) = wrapped::shellcheck(&Some(dir.join("PKGBUILD"))) {
				eprintln!("{}", err);
			};
		} else if &user_input == "d" && is_upstream_merged {
			git_utils::show_upstream_diff(dir, false, rua_paths);
		} else if &user_input == "d" && !is_upstream_merged {
			git_utils::show_upstream_diff(dir, true, rua_paths);
		} else if &user_input == "m" && !is_upstream_merged {
			git_utils::merge_upstream(dir, rua_paths);
		} else if &user_input == "o" && is_upstream_merged {
			break;
		}
	}
}
