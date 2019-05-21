use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum CliArgs {
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
	Show {
		#[structopt(help = "Target to show for", multiple = true, required = true)]
		target: Vec<String>,
	},
	#[structopt(about = "Check *.tar or *.tar.xz archive")]
	Tarcheck {
		#[structopt(help = "Archive to check", required = true)]
		target: PathBuf,
	},
}
