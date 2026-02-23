use crate::rua_paths::RuaPaths;
use colored::*;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

/// Note that we're using `git init` instead of `git clone`-like command
/// to let the user review the initial diff.
/// Also, the local branch does NOT track the remote one --
/// instead it's being merged upon each review.
pub fn init_repo(pkg: &str, dir: &Path, rua_paths: &RuaPaths) {
	silently_run_panic_if_error(&["init", "-q"], dir, rua_paths);
	let http_ref = format!("https://aur.archlinux.org/{}.git", pkg);
	silently_run_panic_if_error(&["remote", "add", "upstream", &http_ref], dir, rua_paths);
	fetch(dir, rua_paths);
}

pub fn fetch(dir: &Path, rua_paths: &RuaPaths) {
	silently_run_panic_if_error(&["fetch", "-q", "upstream"], dir, rua_paths);
}

pub fn is_upstream_merged(dir: &Path, rua_paths: &RuaPaths) -> bool {
	git(dir, rua_paths)
		.args(["merge-base", "--is-ancestor", "upstream/master", "HEAD"])
		.stderr(Stdio::null())
		.status()
		.expect("failed to run git")
		.success()
}

pub fn show_upstream_diff(dir: &Path, reverse: bool, rua_paths: &RuaPaths) {
	let mut command = git(dir, rua_paths);
	command.arg("diff");
	if reverse {
		command.arg("-R");
	};
	command.arg("upstream/master").status().ok();
}

pub fn identical_to_upstream(dir: &Path, rua_paths: &RuaPaths) -> bool {
	git(dir, rua_paths)
		.args(["diff", "--quiet", "upstream/master"])
		.status()
		.map(|t| t.success())
		.unwrap_or(false)
}

pub fn rev_parse_head(dir: &Path, rua_paths: &RuaPaths) -> Option<String> {
	let output = git(dir, rua_paths)
		.args(["rev-parse", "HEAD"])
		.output()
		.ok()?;
	if !output.status.success() {
		return None;
	}
	let rev = String::from_utf8(output.stdout).ok()?;
	Some(rev.trim().to_string())
}

pub fn merge_upstream(dir: &Path, rua_paths: &RuaPaths) {
	let email = "rua@local";
	let name = "RUA";
	git(dir, rua_paths)
		.args(["merge", "upstream/master"])
		.args(["-m", "Merge branch 'upstream/master' (automated by RUA)"])
		.arg("--no-edit")
		.env("GIT_AUTHOR_NAME", name)
		.env("GIT_AUTHOR_EMAIL", email)
		.env("GIT_COMMITTER_NAME", name)
		.env("GIT_COMMITTER_EMAIL", email)
		.status()
		.ok();
}

fn silently_run_panic_if_error(args: &[&str], dir: &Path, rua_paths: &RuaPaths) {
	let command = git(dir, rua_paths)
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

fn git(dir: &Path, rua_paths: &RuaPaths) -> Command {
	let mut command = Command::new(&rua_paths.wrapper_bwrap_script);
	command.arg("--bind");
	command.arg(dir);
	command.arg(dir);
	command.arg("git");
	command.env("GIT_CONFIG", "/dev/null"); // see `man git-config`
	command.env("GIT_CONFIG_NOSYSTEM", "1"); // see `man git`
	command.current_dir(dir);
	command
}
