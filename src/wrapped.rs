// Commands that are run inside "bubblewrap" jail

use crate::rua_paths;
use crate::rua_paths::RuaPaths;
use crate::srcinfo_to_pkgbuild;
use log::debug;
use log::error;
use log::info;
use log::trace;
use srcinfo::Srcinfo;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::str;
use std::str::FromStr;
use std::sync::Once;

static BUBBLEWRAP_IS_RUNNABLE: Once = Once::new();
/// Check if bubblewrap binary is runnable
pub fn check_bubblewrap_runnable() {
	BUBBLEWRAP_IS_RUNNABLE.call_once(|| {
		let command = Command::new("bwrap")
			.args(["--ro-bind", "/", "/", "true"])
			.status();
		let command = command.unwrap_or_else(|err| {
			error!(
				"bwrap binary not found. RUA uses bubblewrap for security isolation. \
				Please install via `pacman -S bubblewrap-suid` (for hardened kernel) \
				or `pacman -S bubblewrap` (otherwise). Underlying error: {}",
				err
			);
			std::process::exit(4)
		});
		if !command.success() {
			error!("Failed to run bwrap.");
			error!(
				"A possible cause is if RUA itself is run in jail (docker, bwrap, firejail,..)."
			);
			error!("If so, see https://github.com/vn971/rua/issues/8");
			std::process::exit(4)
		}
	});
}

/// Creates a new command jail in bubblewrap,
/// suitable for makepkg build and with no internet restrictions.
fn jail_for_makepkg(rua_paths: &RuaPaths, cur_dir: &str, makepkg_dir: &str) -> Command {
	let mut command = Command::new(&rua_paths.wrapper_bwrap_script);
	command.current_dir(cur_dir);
	command.env("PKGDEST", makepkg_dir);
	command.env("SRCDEST", makepkg_dir);
	command.env("SRCPKGDEST", makepkg_dir);
	command.env("LOGDEST", makepkg_dir);
	command.env("BUILDDIR", makepkg_dir);
	command
}

fn download_srcinfo_sources(dir: &str, rua_paths: &RuaPaths) {
	let dir_path = PathBuf::from(dir).join("PKGBUILD.static");
	let mut file = File::create(dir_path)
		.unwrap_or_else(|err| panic!("Cannot create {}/PKGBUILD.static, {}", dir, err));
	let srcinfo_path = PathBuf::from(dir)
		.join(".SRCINFO")
		.canonicalize()
		.unwrap_or_else(|e| panic!("Cannot resolve .SRCINFO path in {}, {}", dir, e));
	file.write_all(srcinfo_to_pkgbuild::static_pkgbuild(&srcinfo_path).as_bytes())
		.expect("cannot write to PKGBUILD.static");
	info!("Downloading sources using .SRCINFO...");
	let command = jail_for_makepkg(rua_paths, dir, dir)
		.args(["--bind", dir, dir])
		.args(["makepkg", "-f", "--verifysource"])
		.args(["-p", "PKGBUILD.static"])
		.status()
		.unwrap_or_else(|e| panic!("Failed to fetch dependencies in directory {}, {}", dir, e));
	assert!(command.success(), "Failed to download PKGBUILD sources");
	fs::remove_file(PathBuf::from(dir).join("PKGBUILD.static"))
		.expect("Failed to clean up PKGBUILD.static");
}

pub fn generate_srcinfo(dir: &str, rua_paths: &RuaPaths) -> Result<Srcinfo, String> {
	debug!("Getting srcinfo in directory {}", dir);
	let mut command = jail_for_makepkg(rua_paths, dir, "/tmp");
	command.arg("--unshare-net");
	command.args(["--ro-bind", dir, dir]);
	command
		.arg("makepkg")
		.arg("--holdver")
		.arg("--printsrcinfo");

	let output = command
		.output()
		.map_err(|err| format!("cannot execute makepkg --holdver --printsrcinfo, {}", err))?;
	if !output.status.success() {
		return Err(format!(
			"makepkg failed to execute, Stdout:\n{}\n\nStderr:\n{}\n",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr),
		));
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

fn build_local(dir: &str, rua_paths: &RuaPaths, offline: bool, force: bool) {
	debug!("{}:{} Building directory {}", file!(), line!(), dir);
	let mut command = jail_for_makepkg(rua_paths, dir, dir);
	if offline {
		command.arg("--unshare-net");
	}
	command.args(["--bind", dir, dir]).arg("makepkg");
	command.env("FAKEROOTDONTTRYCHOWN", "true");
	if force {
		command.arg("--force");
	}
	let command = command.status().unwrap_or_else(|e| {
		panic!(
			"Failed to execute ~/.config/rua/.system/security-wrapper.sh, {}",
			e
		)
	});
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

pub fn build_directory(dir: &str, rua_paths: &RuaPaths, offline: bool, force: bool) {
	if offline {
		download_srcinfo_sources(dir, rua_paths);
	}
	build_local(dir, rua_paths, offline, force);
}

/// Perform a shellcheck check of a PKGBUILD, taking care of special variables
/// See https://github.com/koalaman/shellcheck
/// See ../res/shellcheck-wrapper
pub fn shellcheck(target: &Option<PathBuf>) -> Result<(), String> {
	let target = match target {
		None => Path::new("/dev/stdin").to_path_buf(),
		Some(path) if path.is_dir() => path.join("PKGBUILD"),
		Some(path) => path.to_path_buf(),
	};
	let target_contents = match std::fs::read_to_string(&target) {
		Err(err) => return Err(format!("Failed to open {:?} for reading: {}", target, err)),
		Ok(ok) => ok,
	};
	check_bubblewrap_runnable();
	let mut command = Command::new("bwrap");
	command.args(["--ro-bind", "/", "/"]);
	command.args(["--proc", "/proc", "--dev", "/dev"]);
	command.args(["--unshare-all"]);
	command.args([
		"shellcheck",
		"--norc",
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
		.ok_or("Failed to open stdin for shellcheck")?;
	let bytes = rua_paths::SHELLCHECK_WRAPPER.replace("%PKGBUILD%", &target_contents);
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
