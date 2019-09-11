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
#[structopt(rename_all = "kebab-case")]
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
			help = "forbid internet access while building packages. Sources are downloaded using .SRCINFO only"
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
			help = "forbid internet access while building packages. Sources are downloaded using .SRCINFO only"
		)]
		offline: bool,
		#[structopt(help = "Target directory", required = true)]
		target: PathBuf,
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
		#[structopt(help = "PKGBUILD to check (or ./PKGBUILD if not provided)")]
		target: Option<PathBuf>,
	},
	#[structopt(about = "Check *.pkg.tar or *.pkg.tar.xz archive")]
	Tarcheck {
		#[structopt(help = "Archive to check", required = true)]
		target: PathBuf,
	},
	#[structopt(about = "Upgrade AUR packages")]
	Upgrade {
		#[structopt(long, help = "Also rebuild all *-git packages")]
		devel: bool,
	},
}
