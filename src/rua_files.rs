use crate::rua_environment;
use crate::wrapped;
use directories::ProjectDirs;
use fs2::FileExt;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs::Permissions;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;

pub struct RuaDirs {
	/// Subdirectory of ~/.cache/rua where packages are built after review
	pub global_build_dir: PathBuf,
	/// Subdirectory of ~/.config/rua where the package is reviewed by user, and changes are kept
	global_review_dir: PathBuf,
	/// Directory where built and user-reviewed package artifacts are stored
	global_checked_tars_dir: PathBuf,
	/// Script used to wrap `makepkg` and related commands
	pub wrapper_bwrap_script: PathBuf,
	/// Global lock to prevent concurrent access to project dirs
	_global_lock: File,
}

impl RuaDirs {
	pub fn new() -> RuaDirs {
		let dirs = ProjectDirs::from("com.gitlab", "vn971", "rua")
			.expect("Failed to determine XDG directories");
		std::fs::create_dir_all(dirs.cache_dir())
			.expect("Failed to create project cache directory");
		rm_rf::force_remove_all(dirs.config_dir().join(".system")).ok();
		std::fs::create_dir_all(dirs.config_dir().join(".system"))
			.expect("Failed to create project config directory");
		std::fs::create_dir_all(dirs.config_dir().join("wrap_args.d"))
			.expect("Failed to create project config directory");
		overwrite_file(
			&dirs.config_dir().join(".system/seccomp-i686.bpf"),
			SECCOMP_I686,
		);
		overwrite_file(
			&dirs.config_dir().join(".system/seccomp-x86_64.bpf"),
			SECCOMP_X86_64,
		);
		let seccomp_path = format!(
			".system/seccomp-{}.bpf",
			uname::uname()
				.expect("Failed to get system architecture via uname")
				.machine
		);
		rua_environment::set_env_if_not_set(
			"RUA_SECCOMP_FILE",
			dirs.config_dir().join(seccomp_path).to_str().unwrap(),
		);
		overwrite_script(&dirs.config_dir().join(WRAP_SCRIPT_PATH), WRAP_SH);
		ensure_script(
			&dirs.config_dir().join(".system/wrap_args.sh.example"),
			WRAP_ARGS_EXAMPLE,
		);
		if users::get_current_uid() == 0 {
			eprintln!("RUA does not allow building as root.");
			eprintln!("Also, makepkg will not allow you building as root anyway.");
			exit(1)
		}
		wrapped::check_bubblewrap_runnable();
		let locked_file = File::open(dirs.config_dir()).unwrap_or_else(|err| {
			panic!(
				"Failed to open config dir {:?} for locking, {}",
				dirs.config_dir(),
				err
			);
		});
		locked_file.try_lock_exclusive().unwrap_or_else(|_| {
			eprintln!("Another RUA instance already running.");
			std::process::exit(2)
		});
		RuaDirs {
			global_build_dir: dirs.cache_dir().join("build"),
			global_review_dir: dirs.config_dir().join("pkg"),
			global_checked_tars_dir: dirs.cache_dir().join("checked_tars"),
			wrapper_bwrap_script: dirs.config_dir().join(WRAP_SCRIPT_PATH),
			_global_lock: locked_file,
		}
	}

	/// Same as `global_review_dir`, but for a specific pkgbase
	pub fn review_dir(&self, pkgbase: &str) -> PathBuf {
		self.global_review_dir.join(pkgbase)
	}

	/// Same as `global_build_dir`, but for a specific pkgbase
	pub fn build_dir(&self, pkgbase: &str) -> PathBuf {
		self.global_build_dir.join(pkgbase)
	}

	/// Same as `global_checked_tars_dir`, but for a specific pkgbase
	pub fn checked_tars_dir(&self, pkg_name: &str) -> PathBuf {
		self.global_checked_tars_dir
			.join("checked_tars")
			.join(pkg_name)
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

pub const SHELLCHECK_WRAPPER_BYTES: &str = include_str!("../res/shellcheck-wrapper");
pub const SECCOMP_I686: &[u8] = include_bytes!("../res/seccomp-i686.bpf");
pub const SECCOMP_X86_64: &[u8] = include_bytes!("../res/seccomp-x86_64.bpf");
pub const WRAP_SH: &[u8] = include_bytes!("../res/wrap.sh");
pub const WRAP_ARGS_EXAMPLE: &[u8] = include_bytes!("../res/wrap_args.sh.example");

pub const WRAP_SCRIPT_PATH: &str = ".system/wrap.sh";
