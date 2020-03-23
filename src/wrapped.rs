// Commands that are run inside "bubblewrap" jail

use crate::rua_files::{self, RuaPaths};
use crate::srcinfo_to_pkgbuild;
use log::debug;
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
use std::str::FromStr;
use std::sync::Once;

static BUBBLEWRAP_IS_RUNNABLE: Once = Once::new();
pub fn assert_bubblewrap_runnable() {
	BUBBLEWRAP_IS_RUNNABLE.call_once(|| {
		if !Command::new("bwrap")
			.args(&["--ro-bind", "/", "/", "true"])
			.status()
			.expect("bwrap binary not found. RUA uses bubblewrap for security isolation.")
			.success()
		{
			eprintln!("Failed to run bwrap.");
			eprintln!(
				"A possible cause is if RUA itself is run in jail (docker, bwrap, firejail,..)."
			);
			eprintln!("If so, see https://github.com/vn971/rua/issues/8");
			std::process::exit(4)
		}
	});
}

impl<'cmd> Sandbox<'cmd> {
	pub fn new(rua_paths: &'cmd RuaPaths) -> Self {
		Self {
			jail_cmd: &rua_paths.wrapper_bwrap_script,
		}
	}

	fn makepkg(&self, pkgbuild_dir: &Path, work_dir: Option<&Path>, opts: SandboxOpts) -> Command {
		let mut command = Command::new(self.jail_cmd);

		let pkgbuild_dir = pkgbuild_dir.canonicalize().unwrap();
		command.current_dir(&pkgbuild_dir);

		let work_dir = work_dir.unwrap_or(&pkgbuild_dir);
		command.env("PKGDEST", work_dir);
		command.env("SRCDEST", work_dir);
		command.env("LOGDEST", work_dir);
		command.env("SRCPKGDEST", work_dir);
		command.env("BUILDDIR", work_dir);

		if !opts.network_access {
			command.arg("--unshare-net");
		}

		if !opts.writable_pkgbuild_dir {
			command.arg("--ro-bind");
		} else {
			command.arg("--bind");
		};
		command.args(&[&pkgbuild_dir, &pkgbuild_dir]);

		command.arg("makepkg");
		command
	}
}

fn download_srcinfo_sources(sandbox: Sandbox, dir: &Path) {
	let static_pkgbuild_path = dir.join("PKGBUILD.static");
	let srcinfo_path = dir
		.join(".SRCINFO")
		.canonicalize()
		.unwrap_or_else(|e| panic!("Cannot resolve .SRCINFO path in {}, {}", dir.display(), e));

	File::create(&static_pkgbuild_path)
		.unwrap_or_else(|e| panic!("Cannot create {}/PKGBUILD.static, {}", dir.display(), e))
		.write_all(srcinfo_to_pkgbuild::static_pkgbuild(&srcinfo_path).as_bytes())
		.expect("cannot write to PKGBUILD.static");

	info!("Downloading sources using .SRCINFO...");

	let makepkg_result = sandbox
		.makepkg(dir, None, SandboxOpts {
			writable_pkgbuild_dir: true,
			..SandboxOpts::default()
		})
		.args(&["--force", "--verifysource", "-p", "PKGBUILD.static"])
		.status()
		.unwrap_or_else(|e| panic!("Failed to fetch sources in {}, {}", dir.display(), e));

	assert!(makepkg_result.success(), "Failed to fetch PKGBUILD sources");

	fs::remove_file(static_pkgbuild_path).expect("Failed to clean up PKGBUILD.static");
}

pub fn generate_srcinfo(sandbox: Sandbox, dir: &Path) -> Result<Srcinfo, String> {
	debug!("Getting srcinfo in directory {}", dir.display());

	let makepkg = sandbox
		.makepkg(&dir, Some("/tmp".as_ref()), SandboxOpts::default())
		.args(&["--holdver", "--printsrcinfo"])
		.output()
		.map_err(|err| format!("cannot execute makepkg --holdver --printsrcinfo, {}", err))?;

	if !makepkg.status.success() {
		return Err(format!(
			"makepkg failed to execute, Stdout:\n{}\n\nStderr:\n{}\n",
			String::from_utf8_lossy(&makepkg.stdout),
			String::from_utf8_lossy(&makepkg.stderr),
		));
	}

	let srcinfo = String::from_utf8(makepkg.stdout).map_err(|err| {
		format!(
			"Non-UTF8 in output of makepkg --holdver --printsrcinfo, {}",
			err
		)
	})?;

	trace!("generated SRCINFO content:\n{}", srcinfo);
	let srcinfo = Srcinfo::from_str(&srcinfo).map_err(|e| {
		format!(
			"{}:{} Failed to parse SRCINFO:\n{:?}\nError is: {}",
			file!(),
			line!(),
			srcinfo,
			e
		)
	})?;

	Ok(srcinfo)
}

pub fn build_directory(sandbox: Sandbox, dir: &Path, offline: bool, force: bool) {
	if offline {
		download_srcinfo_sources(sandbox, dir);
	}

	debug!("Building directory {}", dir.display());

	let mut makepkg = sandbox.makepkg(dir, None, SandboxOpts {
		network_access: !offline,
		writable_pkgbuild_dir: true,
		..SandboxOpts::default()
	});

	if force {
		makepkg.arg("--force");
	}

	let makepkg = makepkg
		.status()
		.unwrap_or_else(|e| panic!("Failed to execute {}: {}", sandbox.jail_cmd.display(), e));

	if !makepkg.success() {
		eprintln!(
			"Build failed with exit code {} in {}",
			makepkg
				.code()
				.map_or_else(|| "???".to_owned(), |c| c.to_string()),
			dir.display(),
		);
		std::process::exit(makepkg.code().unwrap_or(1));
	}
}

#[derive(Clone, Copy)]
pub struct Sandbox<'cmd> {
	jail_cmd: &'cmd Path,
}

#[derive(Default)]
struct SandboxOpts {
	network_access: bool,
	writable_pkgbuild_dir: bool,
	_non_exhaustive: (),
}

pub fn shellcheck(target: Option<PathBuf>) -> Result<(), String> {
	let target = match target {
		None => Path::new("/dev/stdin").to_path_buf(),
		Some(path) if path.is_dir() => path.join("PKGBUILD"),
		Some(path) => path,
	};

	let target_contents = match std::fs::read_to_string(&target) {
		Err(err) => return Err(format!("Failed to open {:?} for reading: {}", target, err)),
		Ok(ok) => ok,
	};

	assert_bubblewrap_runnable();

	let mut command = Command::new("bwrap");
	command.args(&["--ro-bind", "/", "/"]);
	command.args(&["--proc", "/proc", "--dev", "/dev"]);
	command.args(&["--unshare-all"]);
	command.args(&[
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
		.map_or(Err("Failed to open stdin for shellcheck"), Ok)?;

	let bytes = rua_files::SHELLCHECK_WRAPPER.replace("%PKGBUILD%", &target_contents);
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
