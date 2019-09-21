// Commands that are run inside "bubblewrap" jail

use crate::rua_files;
use crate::rua_files::RuaDirs;
use crate::srcinfo_to_pkgbuild;
use crate::terminal_util;
use log::debug;
use log::info;
use log::trace;
use srcinfo::Srcinfo;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str;
use std::str::FromStr;
use std::sync::Once;

static BUBBLEWRAP_IS_RUNNABLE: Once = Once::new();
pub fn check_bubblewrap_runnable() {
	BUBBLEWRAP_IS_RUNNABLE.call_once(|| {
		if !Command::new("bwrap")
			.args(&["--ro-bind", "/", "/", "true"])
			.status()
			.expect("bwrap binary not found. RUA uses bubblewrap for security isolation.")
			.success()
		{
			eprintln!("Failed to run bwrap.");
			eprintln!("A possible cause for this is if RUA itself is run in jail (docker, bwrap, firejail,..).");
			eprintln!("If so, see https://github.com/vn971/rua/issues/8");
			std::process::exit(4)
		}
	});
}

fn wrap_yes_internet(dirs: &RuaDirs) -> Command {
	Command::new(&dirs.wrapper_bwrap_script)
}

fn download_srcinfo_sources(dir: &str, dirs: &RuaDirs) {
	let dir_path = PathBuf::from(dir).join("PKGBUILD.static");
	let mut file = File::create(&dir_path)
		.unwrap_or_else(|err| panic!("Cannot create {}/PKGBUILD.static, {}", dir, err));
	let srcinfo_path = PathBuf::from(dir)
		.join(".SRCINFO")
		.canonicalize()
		.unwrap_or_else(|e| panic!("Cannot resolve .SRCINFO path in {}, {}", dir, e));
	file.write_all(srcinfo_to_pkgbuild::static_pkgbuild(&srcinfo_path).as_bytes())
		.expect("cannot write to PKGBUILD.static");
	info!("Downloading sources using .SRCINFO...");
	let command = wrap_yes_internet(dirs)
		.args(&["--bind", dir, dir])
		.args(&["makepkg", "-f", "--verifysource"])
		.args(&["-p", "PKGBUILD.static"])
		.current_dir(dir)
		.status()
		.unwrap_or_else(|e| panic!("Failed to fetch dependencies in directory {}, {}", dir, e));
	assert!(command.success(), "Failed to download PKGBUILD sources");
	fs::remove_file(PathBuf::from(dir).join("PKGBUILD.static"))
		.expect("Failed to clean up PKGBUILD.static");
}

pub fn generate_srcinfo(dir: &str, dirs: &RuaDirs) -> Result<Srcinfo, String> {
	debug!("Getting srcinfo in directory {}", dir);
	let mut command = wrap_yes_internet(dirs);
	command.arg("--unshare-net");
	command.args(&["--ro-bind", dir, dir]);
	command.env("PKGDEST", "/tmp");
	command.env("SRCDEST", "/tmp");
	command.env("BUILDDIR", "/tmp");
	command.current_dir(dir);
	command
		.arg("makepkg")
		.arg("--holdver")
		.arg("--printsrcinfo");

	let output = command
		.output()
		.map_err(|err| format!("cannot execute makepkg --holdver --printsrcinfo, {}", err))?;
	if !output.status.success() {
		Err(format!(
			"makepkg failed to execute, Stdout:\n{}\n\nStderr:\n{}\n",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr),
		))?
	}
	let output = String::from_utf8(output.stdout).map_err(|err| {
		format!(
			"Non-UTF8 in output of makepkg --holdver --printsrcinfo, {}",
			err
		)
	})?;
	trace!("generated SRCINFO content:\n{}", output);
	let srcinfo = Srcinfo::from_str(&output).map_err(|e| {
		format!(
			"{}:{} Failed to parse SRCINFO:\n{:?}\nError is: {}",
			file!(),
			line!(),
			output,
			e
		)
	})?;
	Ok(srcinfo)
}

fn build_local(dir: &str, dirs: &RuaDirs, offline: bool, force: bool) {
	debug!("{}:{} Building directory {}", file!(), line!(), dir);
	let mut command = wrap_yes_internet(dirs);
	command.current_dir(dir);
	command.env("PKGDEST", dir);
	command.env("SRCDEST", dir);
	command.env("SRCPKGDEST", dir);
	command.env("LOGDEST", dir);
	if offline {
		command.arg("--unshare-net");
	}
	command.args(&["--bind", dir, dir]).arg("makepkg");
	if force {
		command.arg("--force");
	}
	let command = command
		.status()
		.unwrap_or_else(|e| panic!("Failed to execute ~/.config/rua/.system/wrap.sh, {}", e));
	if !command.success() {
		eprintln!(
			"Build failed with exit code {} in {}",
			command
				.code()
				.map_or_else(|| "???".to_owned(), |c| c.to_string()),
			dir,
		);
		std::process::exit(command.code().unwrap_or(1));
	}
}

pub fn build_directory(dir: &str, project_dirs: &RuaDirs, offline: bool, force: bool) {
	if offline {
		download_srcinfo_sources(dir, project_dirs);
	}
	build_local(dir, project_dirs, offline, force);
}

pub fn shellcheck(target: &Path) -> Result<(), String> {
	let target = if target.is_dir() {
		target.join("PKGBUILD")
	} else {
		target.to_path_buf()
	};
	if !target.exists() {
		return Err("Could not find target for shellcheck, aborting".to_string());
	} else if !target.is_file() {
		return Err("Shellcheck target is not a file, aborting".to_string());
	};
	check_bubblewrap_runnable();
	let mut command = Command::new("bwrap");
	command.args(&["--ro-bind", "/", "/"]);
	command.args(&["--proc", "/proc", "--dev", "/dev"]);
	command.args(&["--unshare-all"]);
	command.args(&[
		"shellcheck",
		"--check-sourced",
		"--norc",
		"--external-sources",
		// "--exclude", "SC2128"  // this would avoid warning for split packages, where $pkgname looks like an array to shellcheck, but it isn't an array later with `makepkg`
		"/dev/stdin",
	]);
	command.stdin(Stdio::piped());
	let mut child = command.spawn().map_err(|_| {
		"Failed to spawn shellcheck process. Do you have shellcheck installed?\
		 sudo pacman -S --needed shellcheck"
	})?;
	let stdin: &mut std::process::ChildStdin = child
		.stdin
		.as_mut()
		.map_or(Err("Failed to open stdin for shellcheck"), Ok)?;
	let target = target.to_str().expect("Failed to parse shellcheck target");
	let target = terminal_util::escape_bash_arg(target);
	let target = format!("source {}", target);
	let bytes = rua_files::SHELLCHECK_WRAPPER_BYTES.replace("source PKGBUILD", &target);
	stdin.write_all(bytes.as_bytes()).map_err(|err| {
		format!(
			"Failed to write shellcheck wrapper script to shellcheck-s stdin, {}",
			err
		)
	})?;
	let child = child
		.wait_with_output()
		.map_err(|e| format!("Failed waiting for shellcheck to exit: {}", e))?;
	if child.status.success() {
		eprintln!("Good job, shellcheck didn't find problems in the PKGBUILD.");
		Ok(())
	} else {
		Err("".to_string())
	}
}
