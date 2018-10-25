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
mod wrapped;

use chrono::Utc;
use directories::ProjectDirs;
use std::env;
use std::fs::OpenOptions;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
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
	ensure_script(&dirs.config_dir().join("wrap.sh"), include_bytes!("../res/wrap.sh"));

	let opts = parse_opts::parse_opts();
	if let Some(matches) = opts.subcommand_matches("install") {
		let target = matches.value_of("TARGET").unwrap();
		wrapped::download_if_absent(&target, &dirs);
		let deps = wrapped::get_deps(&target, &dirs);
		debug!("deps: {:?}", deps); // TODO: build those deps!
		wrapped::jail_build(dirs.cache_dir().join("build").join(target).to_str().unwrap(), &dirs);
	} else if let Some(matches) = opts.subcommand_matches("jailbuild") {
		let target_dir = matches.value_of("DIR").unwrap_or(".");
		wrapped::jail_build(target_dir, &dirs);
	}
}
