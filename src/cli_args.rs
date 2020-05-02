use std::path::PathBuf;
use structopt::clap::arg_enum;
use structopt::StructOpt;

arg_enum! {
	#[allow(non_camel_case_types)]
	#[derive(Debug)]
	pub enum CLIColorType {
		auto, never, always
	}
}

#[derive(StructOpt, Debug)]
#[structopt(
	rename_all = "kebab-case",
	after_help = "ENVIRONMENT:\n    RUA_SUDO_COMMAND: Sets the alternative command for sudo, such as gosu, doas, runas, suex etc."
)]
pub struct CliArgs {
	#[structopt(
		possible_values = &CLIColorType::variants(),
		case_insensitive = true,
		default_value = "auto",
		long = "color",
		help = "set colors", // the rest of the description is filled in by structopt
	)]
	pub color: CLIColorType,
	#[structopt(subcommand)]
	pub action: Action,
}

#[derive(StructOpt, Debug)]
#[structopt(rename_all = "kebab-case")]
pub enum Action {
	#[structopt(about = "Show package information")]
	Info {
		#[structopt(help = "Target to show for", multiple = true, required = true)]
		target: Vec<String>,
	},
	#[structopt(about = "Download a package by name and build it in jail")]
	Install {
		#[structopt(long = "asdeps", help = "Install package as dependency")]
		asdeps: bool,
		#[structopt(
			short = "o",
			long = "offline",
			help = "Forbid internet access while building packages.
Sources are downloaded using .SRCINFO only"
		)]
		offline: bool,
		#[structopt(help = "Target package", multiple = true, required = true)]
		target: Vec<String>,
	},
	#[structopt(about = "Build package in specified directory, in jail")]
	Builddir {
		#[structopt(
			short = "o",
			long = "offline",
			help = "Forbid internet access while building packages.
Sources are downloaded using .SRCINFO only"
		)]
		offline: bool,
		#[structopt(
			short = "f",
			long = "force",
			help = "use --force option with makepkg, see makepkg(8)"
		)]
		force: bool,
		#[structopt(
			help = "Target directory. Defaults to current directory '.' if not specified."
		)]
		target: Option<PathBuf>,
	},
	#[structopt(about = "Opens AUR web search page")]
	Search {
		#[structopt(help = "Target to search for", multiple = true, required = true)]
		target: Vec<String>,
	},
	#[structopt(
		about = "Run shellcheck on a PKGBUILD, taking care of PKGBUILD-specific variables"
	)]
	Shellcheck {
		#[structopt(
			help = "PKGBUILD or directory to check. Defaults to /dev/stdin if not specified. Appends ./PKGBUILD for directories"
		)]
		target: Option<PathBuf>,
	},
	#[structopt(
		about = "Check *.pkg.tar or *.pkg.tar.xz  or *.pkg.tar.gz or *.pkg.tar.zst archive"
	)]
	Tarcheck {
		#[structopt(help = "Archive to check", required = true)]
		target: PathBuf,
	},
	#[structopt(
		about = "Upgrade AUR packages. To ignore packages, add them to IgnorePkg in /etc/pacman.conf"
	)]
	Upgrade {
		#[structopt(
			long = "devel",
			short = "d",
			help = "Also rebuild development packages.
Supports: git, hg, bzr, svn, cvs, darcs. Currently by suffix only."
		)]
		devel: bool,
		#[structopt(
			long = "printonly",
			help = "Print the list of outdated packages to stdout, delimited by newline. Don't upgrade anything, don't ask questions (for use in scripts)."
		)]
		printonly: bool,
		#[structopt(
			long = "ignore",
			help = "Don't upgrade the specified package(s). Accepts multiple arguments separated by `,`."
		)]
		ignored: Option<String>,
	},
}

/// environment variable that we expect the user might fill
// !WARNING! If you change this, make sure the value the same as documented in CliArgs above.
pub const SUDO_ENVIRONMENT_VARIABLE_NAME: &str = "RUA_SUDO_COMMAND";
