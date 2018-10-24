use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
pub struct Opt {
	/// Build the package in current directory "."
	/// This requires PKGBUILD to be present in current dir.
	#[structopt(short = "h", long = "build-here")]
	build_here: bool,
	#[structopt(short = "t", long = "target")]
	build_target: Option<String>,
}

pub fn parse_opts() -> Opt {
	let opt = Opt::from_args();
	info!("CLI options: {:?}", opt);
	opt
}
