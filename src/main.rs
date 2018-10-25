#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

extern crate chrono;
extern crate config;
extern crate directories;
extern crate env_logger;
extern crate fs2;
extern crate regex;
#[macro_use] extern crate log;
extern crate clap;

mod parse_opts;

use chrono::Utc;
use directories::ProjectDirs;
use regex::Regex;
use std::env;
use std::fs::OpenOptions;
use std::fs;
use std::io::Write;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;
use std::fs::Permissions;

fn ensure_env(key: &str, value: &str) {
	if env::var_os(key).is_none() {
		env::set_var(key, value);
	}
}

fn ensure_file(path: &PathBuf, content: &[u8]) {
	let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(path).unwrap();
	file.write_all(content).unwrap();
}

fn ensure_script(path: &PathBuf, content: &[u8]) {
	ensure_file(path, content);
	fs::set_permissions(path, Permissions::from_mode(0o755)).unwrap();
}


fn assert_command_success(command: &Output) {
	assert!(command.status.success(),
		"Command failed with exit code {:?}\nStderr: {}\nStdout: {}",
		command.status.code(),
		String::from_utf8_lossy(&command.stderr),
		String::from_utf8_lossy(&command.stdout),
	);
}


fn wrap_yes_internet(dirs: &ProjectDirs) -> Command {
	Command::new(dirs.config_dir().join("wrap_yes_internet.sh"))
}
fn wrap_no_internet(dirs: &ProjectDirs) -> Command {
	Command::new(dirs.config_dir().join("wrap_no_internet.sh"))
}

fn download(name: &str, dirs: &ProjectDirs) {
	let valid_name_regexp = Regex::new(r"[a-zA-Z][a-zA-Z._-]*").unwrap();
	assert!(valid_name_regexp.is_match(name), "unexpected package name {}", name);
	// TODO: else download new version, with some caching
	if !Path::new(&dirs.cache_dir().join("build").join(name)).exists() {
		env::set_current_dir(dirs.cache_dir().join("build")).unwrap();
		let dir = format!("{}.tmp", name);
		fs::remove_dir_all(&dir).ok();
		let git_http_ref = format!("https://aur.archlinux.org/{}.git", name);
		let command = Command::new("git").args(&["clone", &git_http_ref, &dir]).output().unwrap();
		assert_command_success(&command);
		env::set_current_dir(&dir).unwrap();
		assert!(Path::new("PKGBUILD").exists(), "PKGBUILD not found for package {}. \
			Does this package really exist in AUR?", name);
		loop {
			let mut string = String::new();
			eprint!("Downloaded {}. Show PKGBUILD? Y=yes, I=run shell to inspect, O=ok, use the file: ", name);
			io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
			let string = string.trim().to_lowercase();

			if string == "y" {
				Command::new("less").arg("PKGBUILD").status().ok();
			} else if string == "i" {
				Command::new(env::var("SHELL").unwrap_or("bash".to_string())).status().ok();
			} else if string == "o" {
				break;
			}
		}
		env::set_current_dir("..").unwrap();
		fs::rename(dir, name).unwrap();
	}
}

fn get_deps(name: &str, dirs: &ProjectDirs) -> Vec<String> {
	let dir = dirs.cache_dir().join("build").join(name);
	let dir = dir.to_str().unwrap();
	env::set_current_dir(dir).unwrap();
	let mut command = wrap_no_internet(dirs);
	command.args(&["--ro-bind", dir, dir]);
	command.args(&["bash", "--restricted", dirs.config_dir().join("get_deps.sh").to_str().unwrap()]);
	let command = command.stderr(Stdio::inherit()).output().unwrap();
	String::from_utf8_lossy(&command.stdout).trim().split(' ').map(|s| s.to_string()).collect()
}

fn download_sources(dirs: &ProjectDirs) {
	let current_dir = env::current_dir().unwrap();
	let current_dir = current_dir.to_str().unwrap();
	let mut command = wrap_yes_internet(dirs);
	command.args(&["--bind", current_dir, current_dir]);
	command.args(&["makepkg", "--noprepare", "--nobuild"]);
	let command = command.status().unwrap();
	assert!(command.success(), "Failed to download PKGBUILD sources");
}

fn do_build(dirs: &ProjectDirs) {
	let dir = env::current_dir().unwrap();
	let mut command = wrap_no_internet(dirs);
	command.args(&["--bind", dir.to_str().unwrap(), dir.to_str().unwrap()]);
	command.args(&["makepkg", "--force"]);
	let command = command.status().unwrap();
	assert!(command.success(), "Failed to download PKGBUILD sources");
}

fn jail_build(dirs: &ProjectDirs) {
	env::set_var("PKGDEST", Path::new(".").canonicalize().unwrap().join("target"));
	download_sources(dirs);
	do_build(dirs);
}


fn main() {
	ensure_env("RUST_LOG", "info");
	ensure_env("RUST_BACKTRACE", "1");
	env_logger::Builder::from_default_env()
		.format(|buf, record| writeln!(buf,
			"{} [{}] - {}",
			Utc::now().format("%Y-%m-%d %H:%M:%S"),
			record.level(),
			record.args()
		))
		.init();
	info!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

	let dirs = ProjectDirs::from("com.gitlab", "vn971", "rua").unwrap();
	std::fs::create_dir_all(dirs.cache_dir().join("build")).unwrap();
	std::fs::create_dir_all(dirs.config_dir()).unwrap();
	ensure_env("RUA_CONFIG_DIR", dirs.config_dir().to_str().unwrap());
	ensure_file(&dirs.config_dir().join("seccomp.bpf"), include_bytes!("../res/seccomp.bpf"));
	ensure_script(&dirs.config_dir().join("get_deps.sh"), include_bytes!("../res/get_deps.sh"));
	ensure_script(&dirs.config_dir().join("wrap_no_internet.sh"), include_bytes!("../res/wrap_no_internet.sh"));
	ensure_script(&dirs.config_dir().join("wrap_yes_internet.sh"), include_bytes!("../res/wrap_yes_internet.sh"));

	let opts = parse_opts::parse_opts();
	if let Some(matches) = opts.subcommand_matches("install") {
		let target = matches.value_of("TARGET").unwrap();
		download(&target, &dirs);
		let deps = get_deps(&target, &dirs);
		debug!("deps: {:?}", deps);
		env::set_current_dir(dirs.cache_dir().join("build").join(target)).unwrap();
		jail_build(&dirs);
	} else if let Some(matches) = opts.subcommand_matches("jailbuild") {
		let target_dir = matches.value_of("DIR").unwrap_or(".");
		env::set_current_dir(target_dir).expect(format!("directory {} not accessible", target_dir).as_str());
		jail_build(&dirs);
	}
}
