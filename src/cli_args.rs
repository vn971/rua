use std::path::PathBuf;
use clap::{Parser, Subcommand, ValueEnum};


#[derive(Clone, ValueEnum)]
pub enum CLIColorType {
	Auto,
	Never,
	Always,
}

#[derive(Parser)]
#[command(version, about, long_about = None, after_help = "ENVIRONMENT:\n    RUA_SUDO_COMMAND: Sets the alternative command for sudo, such as gosu, doas, runas, suex etc.")]
pub struct Cli {
	/// Set colors. Respects NO_COLOR environment and CLICOLOR specification
	#[arg(long, value_enum, default_value_t = CLIColorType::Auto, ignore_case = true)]
	pub color: CLIColorType,

	#[command(subcommand)]
	pub action: Action,
}

#[derive(Subcommand)]
pub enum Action {
	/// Build package in specified directory, in jail
	Builddir {
		/// Forbid internet access while building packages.
		/// Sources are downloaded using .SRCINFO only
		#[arg(short, long)]
		offline: bool,

		/// Use --force option with makepkg, see makepkg(8)
		#[arg(short, long)]
		force: bool,

		/// Target directory. Defaults to current directory '.' if not specified.
		target: Option<PathBuf>,
	},
	/// Show package information
	Info {
		/// Target to show for
		target: Vec<String>,
	},
	/// Download a package by name and build it in jail
	Install {
		/// Install package as dependency
		#[arg(long)]
		asdeps: bool,

		/// Forbid internet access while building packages.
		/// Sources are downloaded using .SRCINFO only
		#[arg(short, long)]
		offline: bool,

		/// Target package(s)
		target: Vec<String>,
	},
	/// Search for packages by name or description. If multiple keywords are used, all of them must match.
	Search {
		/// Target to search for
		target: Vec<String>,
	},
	/// Run shellcheck on a PKGBUILD, taking care of PKGBUILD-specific variables
	Shellcheck {
		/// PKGBUILD or directory to check. Defaults to /dev/stdin if not specified. Appends ./PKGBUILD for directories
		target: Option<PathBuf>,
	},
	/// Check *.pkg.tar or *.pkg.tar.xz  or *.pkg.tar.gz or *.pkg.tar.zst archive
	Tarcheck {
		/// Archive to check
		target: PathBuf,
	},
	/// Upgrade AUR packages. To ignore packages, add them to IgnorePkg in /etc/pacman.conf
	Upgrade {
		/// Also rebuild development packages.
		/// Supports: git, hg, bzr, svn, cvs, darcs. Currently by suffix only.
		#[arg(short, long)]
		devel: bool,

		/// Print the list of outdated packages to stdout, delimited by newline. Don't upgrade anything, don't ask questions (for use in scripts). Exits with code 7 if no upgrades are available.
		#[arg(long)]
		printonly: bool,

		/// Don't upgrade the specified package(s). Accepts multiple arguments separated by `,`.
		#[arg(long)]
		ignored: Option<String>,
	},
}

/// environment variable that we expect the user might fill
// !WARNING! If you change this, make sure the value the same as documented in Cli above.
#[allow(dead_code)] // unused from inside build.rs
pub const SUDO_ENVIRONMENT_VARIABLE_NAME: &str = "RUA_SUDO_COMMAND";
