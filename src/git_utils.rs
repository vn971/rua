use colored::*;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

/// Note that we're using `git init` instead of `git clone`-like command
/// to let the user review the initial diff.
/// Also, the local branch does NOT track the remote one --
/// instead it's being merged upon each review.
pub fn init_repo(pkg: &str, dir: &Path) {
	silently_run_panic_if_error(&["init", "-q"], dir);
	let http_ref = format!("https://aur.archlinux.org/{}.git", pkg);
	silently_run_panic_if_error(&["remote", "add", "upstream", &http_ref], dir);
	fetch(dir);
}

pub fn fetch(dir: &Path) {
	silently_run_panic_if_error(&["fetch", "-q", "upstream"], dir);
}

pub fn is_upstream_merged(dir: &Path) -> bool {
	git(dir)
		.args(&["merge-base", "--is-ancestor", "upstream/master", "HEAD"])
		.stderr(Stdio::null())
		.status()
		.expect("failed to run git")
		.success()
}

pub fn show_upstream_diff(dir: &Path, reverse: bool) {
	let mut command = git(dir);
	command.arg("diff");
	if reverse {
		command.arg("-R");
	};
	command.arg("upstream/master").status().ok();
}

pub fn identical_to_upstream(dir: &Path) -> bool {
	git(dir)
		.args(&["diff", "--quiet", "upstream/master"])
		.status()
		.map(|t| t.success())
		.unwrap_or(false)
}

pub fn merge_upstream(dir: &Path) {
	git(dir).args(&["merge", "upstream/master"]).status().ok();
}

fn silently_run_panic_if_error(args: &[&str], dir: &Path) {
	let command = git(dir)
		.args(args)
		.output()
		.unwrap_or_else(|err| panic!("Failed to execute process git {:?}, {}", args, err));
	assert!(
		command.status.success(),
		"Command git {} failed with exit code {:?}\nStderr: {}\nStdout: {}",
		args.join(" "),
		command.status.code(),
		String::from_utf8_lossy(&command.stderr).red(),
		String::from_utf8_lossy(&command.stdout),
	);
}

fn git(dir: &Path) -> Command {
	let mut command = Command::new("git");
	command.env("GIT_CONFIG", "/dev/null"); // see `man git-config`
	command.env("GIT_CONFIG_NOSYSTEM", "1"); // see `man git`
	command.env("XDG_CONFIG_HOME", "/dev/null"); // see `man git`
	command.env("HOME", "/dev/null"); // see `man git`
	command.current_dir(dir);
	command
}
