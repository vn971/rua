use directories::ProjectDirs;
use std::path::PathBuf;

/// subdirectory of ~/.config/rua where the package is reviewed by user, and changes are kept
pub fn global_review_dir(dirs: &ProjectDirs) -> PathBuf {
	dirs.config_dir().join("pkg")
}

pub fn review_dir(dirs: &ProjectDirs, pkg_name: &str) -> PathBuf {
	global_review_dir(dirs).join(pkg_name)
}

/// Directory where packages are built after review
pub fn global_build_dir(dirs: &ProjectDirs) -> PathBuf {
	dirs.cache_dir().join("build")
}

pub fn build_dir(dirs: &ProjectDirs, pkg_name: &str) -> PathBuf {
	global_build_dir(dirs).join(pkg_name)
}

/// Directory where built and user-reviewed package artifacts are stored,
pub fn checked_tars_dir(dirs: &ProjectDirs, pkg_name: &str) -> PathBuf {
	dirs.cache_dir().join("checked_tars").join(pkg_name)
}

pub const SHELLCHECK_WRAPPER_BYTES: &str = include_str!("../res/shellcheck-wrapper");
pub const SECCOMP_I686: &[u8] = include_bytes!("../res/seccomp-i686.bpf");
pub const SECCOMP_X86_64: &[u8] = include_bytes!("../res/seccomp-x86_64.bpf");
pub const WRAP_SH: &[u8] = include_bytes!("../res/wrap.sh");
pub const WRAP_ARGS_EXAMPLE: &[u8] = include_bytes!("../res/wrap_args.sh.example");
