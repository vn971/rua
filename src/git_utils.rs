use colored::*;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Note that we're using `git init` instead of `git clone`-like command
/// to let the user review the initial diff.
/// Also, the local branch does NOT track the remote one --
/// instead it's being merged upon each review.
pub fn init_repo(pkg: &str, dir: &PathBuf) {
	silently_run_panic_if_error("git", &["init", "-q"], dir);
	let http_ref = format!("https://aur.archlinux.org/{}.git", pkg);
	silently_run_panic_if_error("git", &["remote", "add", "upstream", &http_ref], dir);
	fetch(dir);
}

pub fn fetch(dir: &PathBuf) {
	silently_run_panic_if_error("git", &["fetch", "-q", "upstream"], dir);
}

pub fn is_upstream_merged(dir: &PathBuf) -> bool {
	let rev_parse_head = git(dir)
		.args(&["rev-parse", "HEAD"])
		.stderr(Stdio::null())
		.stdout(Stdio::null())
		.status()
		.expect("failed to run git");
	if !rev_parse_head.success() {
		false
	} else {
		git(dir)
			.args(&["merge-base", "--is-ancestor", "upstream/master", "HEAD"])
			.status()
			.expect("failed to run git")
			.success()
	}
}

pub fn show_upstream_diff(dir: &PathBuf) {
	git(dir)
		.args(&["diff", "-R", "upstream/master"])
		.status()
		.ok();
}

pub fn merge_upstream(dir: &PathBuf) {
	git(dir).args(&["merge", "upstream/master"]).status().ok();
}

fn silently_run_panic_if_error(first_arg: &str, other_args: &[&str], directory: &PathBuf) {
	let command = Command::new(first_arg)
		.args(other_args)
		.current_dir(directory)
		.output()
		.unwrap_or_else(|err| panic!("Failed to execute process {}, {}", first_arg, err));
	assert!(
		command.status.success(),
		"Command {} {} failed with exit code {:?}\nStderr: {}\nStdout: {}",
		first_arg,
		other_args.join(" "),
		command.status.code(),
		String::from_utf8_lossy(&command.stderr).red(),
		String::from_utf8_lossy(&command.stdout),
	);
}

fn git(dir: &PathBuf) -> Command {
	let mut command = Command::new("git");
	command.current_dir(dir);
	command
}
