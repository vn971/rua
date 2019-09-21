use crate::cli_args::CLIColorType;
use crate::cli_args::CliArgs;
use chrono::Utc;
use env_logger::Env;
use log::debug;
use std::env;
use std::io::Write;

pub fn set_env_if_not_set(key: &str, value: &str) {
	if env::var_os(key).is_none() {
		env::set_var(key, value);
	}
}

// sets environment and other things applicable to all RUA commands
pub fn prepare_environment(config: &CliArgs) {
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
	match config.color {
		// see "colored" crate and referenced specs
		CLIColorType::auto => {
			env::remove_var("NOCOLOR");
			env::remove_var("CLICOLOR_FORCE");
			env::remove_var("CLICOLOR");
		}
		CLIColorType::never => {
			env::set_var("NOCOLOR", "1");
			env::remove_var("CLICOLOR_FORCE");
			env::set_var("CLICOLOR", "0");
		}
		CLIColorType::always => {
			env::remove_var("NOCOLOR");
			env::set_var("CLICOLOR_FORCE", "1");
			env::remove_var("CLICOLOR");
		}
	}
	debug!(
		"{} version {}",
		env!("CARGO_PKG_NAME"),
		env!("CARGO_PKG_VERSION")
	);
	assert!(
		env::var_os("PKGDEST").is_none(),
		"Cannot work with PKGDEST environment being set. Please run RUA without it"
	);
	assert!(
		env::var_os("SRCDEST").is_none(),
		"Cannot work with SRCDEST environment being set. Please run RUA without it"
	);
	assert!(
		env::var_os("SRCPKGDEST").is_none(),
		"Cannot work with SRCPKGDEST environment being set. Please run RUA without it"
	);
	assert!(
		env::var_os("LOGDEST").is_none(),
		"Cannot work with LOGDEST environment being set. Please run RUA without it"
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
}

pub fn extension() -> String {
	std::env::var("PKGEXT").expect("Internal error: variable PKGEXT is unset")
}
