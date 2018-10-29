#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

extern crate chrono;
extern crate clap;
extern crate config;
extern crate directories;
extern crate env_logger;
extern crate fs2;
extern crate itertools;
extern crate regex;
extern crate tar;
#[macro_use] extern crate log;

mod parse_opts;
mod wrapped;
mod tar_check;
mod pacman;
mod aur;

use chrono::Utc;
use directories::ProjectDirs;
use env_logger::Env;
use std::env;
use std::fs::OpenOptions;
use std::fs::Permissions;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;

fn ensure_env(key: &str, value: &str) {
	if env::var_os(key).is_none() {
		env::set_var(key, value);
	}
}


fn overwrite_file(path: &PathBuf, content: &[u8]) {
	let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(path).unwrap();
	file.write_all(content).unwrap();
}

fn ensure_script(path: &PathBuf, content: &[u8]) {
	if path.exists() == false {
		let mut file = OpenOptions::new().create(true).write(true).open(path).unwrap();
		file.write_all(content).unwrap();
		fs::set_permissions(path, Permissions::from_mode(0o755)).unwrap();
	}
}

fn overwrite_script(path: &PathBuf, content: &[u8]) {
	overwrite_file(path, content);
	fs::set_permissions(path, Permissions::from_mode(0o755)).unwrap();
}


fn main() {
	ensure_env("RUST_BACKTRACE", "1");
	env_logger::Builder::from_env(Env::default().filter_or("LOG_LEVEL", "info"))
		.format(|buf, record| writeln!(buf,
			"{} [{}] - {}",
			Utc::now().format("%Y-%m-%d %H:%M:%S"),
			record.level(),
			record.args()
		))
		.init();
	info!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
	assert!(env::var("PKGDEST").is_err(), "PKGDEST environment is set, but RUA needs to modify it. Please run RUA without it");
	assert!(env::var("PKGEXT").is_err(), "PKGEXT environment is set, but RUA needs to modify it. Please run RUA without it");
	ensure_env("PKGEXT", ".pkg.tar");

	let dirs = ProjectDirs::from("com.gitlab", "vn971", "rua").unwrap();
	std::fs::create_dir_all(dirs.cache_dir()).unwrap();
	std::fs::create_dir_all(dirs.config_dir().join(".system")).unwrap();
	ensure_env("RUA_CONFIG_DIR", dirs.config_dir().to_str().unwrap());
	let seccomp_file = dirs.config_dir().join(".system/seccomp.bpf");
	if cfg!(target_arch = "i686") {
		overwrite_file(&seccomp_file, include_bytes!("../res/seccomp-i686.bpf"));
	} else if cfg!(target_arch = "x86_64") {
		overwrite_file(&seccomp_file, include_bytes!("../res/seccomp-x86_64.bpf"));
	} else if seccomp_file.exists() == false {
		panic!("Unable to find seccomp file for your architecture. Please create it and put it to {:?}", seccomp_file);
	}
	ensure_env("RUA_SECCOMP_FILE", seccomp_file.to_str().unwrap());
	overwrite_script(&dirs.config_dir().join(wrapped::GET_DEPS_SCRIPT_PATH), include_bytes!("../res/get_deps.sh"));
	overwrite_script(&dirs.config_dir().join(wrapped::WRAP_SCRIPT_PATH), include_bytes!("../res/wrap.sh"));
	ensure_script(&dirs.config_dir().join("wrap_args.sh"), include_bytes!("../res/wrap_args.sh"));

	let opts = parse_opts::parse_opts();
	if let Some(matches) = opts.subcommand_matches("install") {
		fs::remove_dir_all(dirs.cache_dir()).unwrap();
		std::fs::create_dir_all(dirs.cache_dir()).unwrap();
		let target = matches.value_of("TARGET").unwrap();
		let is_offline = matches.is_present("offline");
		wrapped::install(target, &dirs, is_offline);
	} else if let Some(matches) = opts.subcommand_matches("jailbuild") {
		let target_dir = matches.value_of("DIR").unwrap_or(".");
		let is_offline = matches.is_present("offline");
		wrapped::build_directory(target_dir, &dirs, is_offline);
		for file in fs::read_dir("target").unwrap() {
			tar_check::tar_check(file.unwrap().path());
		}
		eprintln!("Package built and checked in: {:?}", Path::new(target_dir).join("target"));
	} else if let Some(matches) = opts.subcommand_matches("tarcheck") {
		let target_dir = matches.value_of("TARGET").unwrap();
		tar_check::tar_check(Path::new(target_dir).to_path_buf());
	}
}
