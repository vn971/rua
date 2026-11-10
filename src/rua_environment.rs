use crate::cli_args;
use crate::cli_args::CLIColorType;
use crate::cli_args::Cli;
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

/// Set and check environment applicable to all RUA commands
pub fn prepare_environment(config: &Cli) {
	env_logger::Builder::from_env(Env::default().filter_or("RUST_LOG", "info"))
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
		CLIColorType::Auto => {
			env::remove_var("NOCOLOR");
			env::remove_var("CLICOLOR_FORCE");
			env::remove_var("CLICOLOR");
		}
		CLIColorType::Never => {
			env::set_var("NOCOLOR", "1");
			env::remove_var("CLICOLOR_FORCE");
			env::set_var("CLICOLOR", "0");
		}
		CLIColorType::Always => {
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
}

pub fn sudo_command() -> String {
	std::env::var(cli_args::SUDO_ENVIRONMENT_VARIABLE_NAME).unwrap_or_else(|_| "sudo".to_string())
}
