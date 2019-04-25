/// Directory to `git clone` into, first step of the build pipeline
pub const PREFETCH_DIR: &str = "aur.tmp";

/// Directory from AUR that passed user review
pub const REVIEWED_BUILD_DIR: &str = "build";

/// Directory where built package artifacts are stored, *.pkg.tar.xz
pub const TARGET_SUBDIR: &str = "target";

/// Directory where built and user-reviewed package artifacts are stored,
pub const CHECKED_TARS: &str = "checked_tars";
