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
		raw(possible_values = "&CLIColorType::variants()"),
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
	Jailbuild {
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
		#[structopt(help = "Target to search for", required = true)]
		target: String,
	},
	#[structopt(about = "Show package information")]
	Info {
		#[structopt(help = "Target to show for", multiple = true, required = true)]
		target: Vec<String>,
	},
	#[structopt(about = "Check *.tar or *.tar.xz archive")]
	Tarcheck {
		#[structopt(help = "Archive to check", required = true)]
		target: PathBuf,
	},
}
