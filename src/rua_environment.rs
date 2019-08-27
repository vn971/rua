use crate::rua_files;
use crate::wrapped;
use chrono::Utc;
use directories::ProjectDirs;
use env_logger::Env;
use log::debug;
use std::env;
use std::fs;
use std::fs::{OpenOptions, Permissions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::exit;
use std::process::Command;

fn set_env_if_not_set(key: &str, value: &str) {
	if env::var_os(key).is_none() {
		env::set_var(key, value);
	}
}

fn overwrite_file(path: &Path, content: &[u8]) {
	let mut file = OpenOptions::new()
		.create(true)
		.write(true)
		.truncate(true)
		.open(path)
		.unwrap_or_else(|err| panic!("Failed to overwrite (initialize) file {:?}, {}", path, err));
	file.write_all(content).unwrap_or_else(|e| {
		panic!(
			"Failed to write to file {:?} during initialization, {}",
			path, e
		)
	});
}

fn ensure_script(path: &Path, content: &[u8]) {
	if !path.exists() {
		let mut file = OpenOptions::new()
			.create(true)
			.write(true)
			.open(path)
			.unwrap_or_else(|e| panic!("Failed to overwrite (initialize) file {:?}, {}", path, e));
		file.write_all(content).unwrap_or_else(|e| {
			panic!(
				"Failed to write to file {:?} during initialization, {}",
				path, e
			)
		});
		fs::set_permissions(path, Permissions::from_mode(0o755))
			.unwrap_or_else(|e| panic!("Failed to set permissions for {:?}, {}", path, e));
	}
}

fn overwrite_script(path: &Path, content: &[u8]) {
	overwrite_file(path, content);
	fs::set_permissions(path, Permissions::from_mode(0o755))
		.unwrap_or_else(|e| panic!("Failed to set permissions for {:?}, {}", path, e));
}

// sets environment and other things applicable to all RUA commands
pub fn prepare_environment() {
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
}

pub fn prepare_for_jailed_action(dirs: &ProjectDirs) {
	if users::get_current_uid() == 0 {
		eprintln!("RUA does not allow building as root.");
		eprintln!("Also, makepkg will not allow you building as root anyway.");
		exit(1)
	}
	assert!(
		env::var_os("PKGDEST").is_none(),
		"Cannot work with PKGDEST environment being set. Please run RUA without it"
	);
	assert!(
		env::var_os("SRCDEST").is_none(),
		"Cannot work with SRCDEST environment being set. Please run RUA without it"
	);
	assert!(
		env::var_os("BUILDDIR").is_none(),
		"Cannot work with BUILDDIR environment being set. Please run RUA without it"
	);
	if let Some(extension) = std::env::var_os("PKGEXT") {
		assert!(
			extension == ".pkg.tar" || extension == ".pkg.tar.xz",
			"PKGEXT environment is set to an incompatible value. \
			 Only .pkg.tar and .pkg.tar.xz are supported for now.\
			 RUA needs those extensions to look inside the archives for 'tar_check' analysis."
		);
	} else {
		env::set_var("PKGEXT", ".pkg.tar.xz");
	};
	if !Command::new("bwrap")
		.args(&["--ro-bind", "/", "/", "true"])
		.status()
		.expect("bwrap binary not found. RUA uses bubblewrap for security isolation.")
		.success()
	{
		eprintln!("Failed to run bwrap.");
		eprintln!("A possible cause for this is if RUA itself is run in jail (docker, bwrap, firejail,..).");
		eprintln!("If so, see https://github.com/vn971/rua/issues/8");
		exit(4)
	}
	std::fs::create_dir_all(dirs.cache_dir()).expect("Failed to create project cache directory");
	rm_rf::force_remove_all(dirs.config_dir().join(".system")).ok();
	std::fs::create_dir_all(dirs.config_dir().join(".system"))
		.expect("Failed to create project config directory");
	std::fs::create_dir_all(dirs.config_dir().join("wrap_args.d"))
		.expect("Failed to create project config directory");
	overwrite_file(
		&dirs.config_dir().join(".system/seccomp-i686.bpf"),
		rua_files::SECCOMP_I686,
	);
	overwrite_file(
		&dirs.config_dir().join(".system/seccomp-x86_64.bpf"),
		rua_files::SECCOMP_X86_64,
	);
	let seccomp_path = format!(
		".system/seccomp-{}.bpf",
		uname::uname()
			.expect("Failed to get system architecture via uname")
			.machine
	);
	set_env_if_not_set(
		"RUA_SECCOMP_FILE",
		dirs.config_dir().join(seccomp_path).to_str().unwrap(),
	);
	overwrite_script(
		&dirs.config_dir().join(wrapped::WRAP_SCRIPT_PATH),
		rua_files::WRAP_SH,
	);
	ensure_script(
		&dirs.config_dir().join(".system/wrap_args.sh.example"),
		rua_files::WRAP_ARGS_EXAMPLE,
	);
}
