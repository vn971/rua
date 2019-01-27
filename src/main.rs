#[global_allocator]
static GLOBAL: std::alloc::System = std::alloc::System;

extern crate chrono;
extern crate clap;
extern crate directories;
extern crate env_logger;
extern crate fs2;
extern crate itertools;
extern crate libalpm_fork as libalpm;
extern crate regex;
extern crate rm_rf;
extern crate tar;
extern crate uname;
extern crate xz2;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

mod aur_download;
mod cli_args;
mod pacman;
mod srcinfo;
mod tar_check;
mod util;
mod wrapped;

use chrono::Utc;
use directories::ProjectDirs;
use env_logger::Env;
use fs2::FileExt;
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs::Permissions;
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
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .unwrap_or_else(|_| panic!("Failed to overwrite (initialize) file {:?}", path));
    file.write_all(content)
        .unwrap_or_else(|_| panic!("Failed to write to file {:?} during initialization", path));
}

fn ensure_script(path: &PathBuf, content: &[u8]) {
    if !path.exists() {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(path)
            .unwrap_or_else(|_| panic!("Failed to overwrite (initialize) file {:?}", path));
        file.write_all(content)
            .unwrap_or_else(|_| panic!("Failed to write to file {:?} during initialization", path));
        fs::set_permissions(path, Permissions::from_mode(0o755))
            .unwrap_or_else(|_| panic!("Failed to set permissions for {:?}", path));
    }
}

fn overwrite_script(path: &PathBuf, content: &[u8]) {
    overwrite_file(path, content);
    fs::set_permissions(path, Permissions::from_mode(0o755))
        .unwrap_or_else(|_| panic!("Failed to set permissions for {:?}", path));
}

fn main() {
    ensure_env("RUST_BACKTRACE", "1"); // if it wasn't set to "0" explicitly, set it to 1.
    env_logger::Builder::from_env(Env::default().filter_or("LOG_LEVEL", "info"))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Utc::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();
    debug!(
        "{} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        env::var("PKGDEST").is_err(),
        "PKGDEST environment is set, but RUA needs to modify it. Please run RUA without it"
    );
    let is_extension_compatible = env::var_os("PKGEXT").map_or(true, |ext| {
        let ext = ext.to_string_lossy();
        ext.ends_with(".tar") || ext.ends_with(".tar.xz")
    });
    assert!(
        is_extension_compatible,
        "PKGEXT environment is set to an incompatible value. \
         Only *.tar and *.tar.xz are supported."
    );
    ensure_env("PKGEXT", ".pkg.tar.xz");

    let dirs = ProjectDirs::from("com.gitlab", "vn971", "rua")
        .expect("Failed to determine XDG directories");
    std::fs::create_dir_all(dirs.cache_dir()).expect("Failed to create project cache directory");
    rm_rf::force_remove_all(dirs.config_dir().join(".system"), true).ok();
    std::fs::create_dir_all(dirs.config_dir().join(".system"))
        .expect("Failed to create project config directory");
    std::fs::create_dir_all(dirs.config_dir().join("wrap_args.d"))
        .expect("Failed to create project config directory");
    overwrite_file(
        &dirs.config_dir().join(".system/seccomp-i686.bpf"),
        include_bytes!("../res/seccomp-i686.bpf"),
    );
    overwrite_file(
        &dirs.config_dir().join(".system/seccomp-x86_64.bpf"),
        include_bytes!("../res/seccomp-x86_64.bpf"),
    );
    let seccomp_path = format!(
        ".system/seccomp-{}.bpf",
        uname::uname()
            .expect("Failed to get system architecture via uname")
            .machine
    );
    ensure_env(
        "RUA_SECCOMP_FILE",
        dirs.config_dir().join(seccomp_path).to_str().unwrap(),
    );
    overwrite_script(
        &dirs.config_dir().join(wrapped::WRAP_SCRIPT_PATH),
        include_bytes!("../res/wrap.sh"),
    );
    ensure_script(
        &dirs.config_dir().join(".system/wrap_args_example.sh"),
        include_bytes!("../res/wrap_args.sh"),
    );
    let opts = cli_args::build_cli().get_matches();
    let locked_file = File::open(dirs.config_dir()).expect("Failed to find config dir for locking");
    locked_file
        .try_lock_exclusive()
        .expect("Another RUA instance is already running.");

    if let Some(matches) = opts.subcommand_matches("install") {
        let target = matches
            .value_of("TARGET")
            .expect("Cannot get installation TARGET");
        let is_offline = matches.is_present("offline");
        wrapped::install(target, &dirs, is_offline);
    } else if let Some(matches) = opts.subcommand_matches("jailbuild") {
        let target_dir = matches.value_of("TARGET").unwrap_or(".");
        let is_offline = matches.is_present("offline");
        wrapped::build_directory(target_dir, &dirs, is_offline, false);
        for file in fs::read_dir("target").expect("'target' directory not found") {
            tar_check::tar_check(
                file.expect("Failed to open file for tar_check analysis")
                    .path(),
            );
        }
        eprintln!(
            "Package built and checked in: {:?}",
            Path::new(target_dir).join("target")
        );
    } else if let Some(matches) = opts.subcommand_matches("tarcheck") {
        let target = matches
            .value_of("TARGET")
            .expect("Cannot get tarcheck TARGET");
        tar_check::tar_check(Path::new(target).to_path_buf());
        eprintln!("Package passed all checks: {}", target);
    } else if let Some(matches) = opts.subcommand_matches("search") {
        let target = matches
            .value_of("TARGET")
            .expect("Cannot get TARGET argument");
        eprintln!(
            "Results for '{}', sorted by popularity: \
             https://aur.archlinux.org/packages/?K={}&SB=p&SO=d",
            target, target
        );
    }
}
