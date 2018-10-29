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
use std::env;
use std::fs::OpenOptions;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::fs::Permissions;
use std::path::Path;

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
	assert!(env::var("PKGDEST").is_err(), "PKGDEST environment is set, but RUA needs to modify it. Please run RUA without it");
	assert!(env::var("PKGEXT").is_err(), "PKGEXT environment is set, but RUA needs to modify it. Please run RUA without it");
	ensure_env("PKGEXT", ".pkg.tar");

	let dirs = ProjectDirs::from("com.gitlab", "vn971", "rua").unwrap();
	std::fs::create_dir_all(dirs.cache_dir().join(".rua")).unwrap();
	std::fs::create_dir_all(dirs.config_dir()).unwrap();
	ensure_env("RUA_CONFIG_DIR", dirs.config_dir().to_str().unwrap());
	let seccomp_file = dirs.cache_dir().join(".rua/seccomp.bpf");
	if cfg!(target_arch = "i686") {
		ensure_file(&seccomp_file, include_bytes!("../res/seccomp-i686.bpf"));
	} else if cfg!(target_arch = "x86_64") {
		ensure_file(&seccomp_file, include_bytes!("../res/seccomp-x86_64.bpf"));
	} else if seccomp_file.exists() == false {
		panic!("Unable to find seccomp file for your architecture. Please create it and put it to {:?}", seccomp_file);
	}
	ensure_env("RUA_SECCOMP_FILE", seccomp_file.to_str().unwrap());
	ensure_script(&dirs.cache_dir().join(".rua/get_deps.sh"), include_bytes!("../res/get_deps.sh"));
	ensure_script(&dirs.cache_dir().join(".rua/wrap.sh"), include_bytes!("../res/wrap.sh"));


	let opts = parse_opts::parse_opts();
	if let Some(matches) = opts.subcommand_matches("install") {
		let target = matches.value_of("TARGET").unwrap();
		wrapped::install(target, &dirs);
	} else if let Some(matches) = opts.subcommand_matches("jailbuild") {
		let target_dir = matches.value_of("DIR").unwrap_or(".");
		wrapped::build_directory(target_dir, &dirs);
		for file in fs::read_dir("target").unwrap() {
			tar_check::tar_check(file.unwrap().path());
		}
	} else if let Some(matches) = opts.subcommand_matches("tarcheck") {
		let target_dir = matches.value_of("TARGET").unwrap();
		tar_check::tar_check(Path::new(target_dir).to_path_buf());
	}
}
