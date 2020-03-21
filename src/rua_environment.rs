use crate::cli_args;
use crate::cli_args::CLIColorType;
use crate::cli_args::CliArgs;
use crate::rua_files::RuaDirs;
use chrono::Utc;
use colored::Colorize;
use env_logger::Env;
use log::debug;
use std::env;
use std::io::Write;
use std::process::Command;

pub struct RuaEnv {
	pub dirs: RuaDirs,
	pub pkgext: String,
}

pub fn set_env_if_not_set(key: &str, value: &str) {
	if env::var_os(key).is_none() {
		env::set_var(key, value);
	}
}

// sets environment and other things applicable to all RUA commands
pub fn prepare_environment(config: &CliArgs) -> RuaEnv {
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

	let dirs = RuaDirs::new();
	let mut pkgext = None;

	let config = Command::new(&dirs.makepkg_config_loader)
		.output()
		.unwrap_or_else(|e| panic!("Internal error: failed to run makepkg config loader: {}", e))
		.stdout;
	let config = String::from_utf8(config).expect("makepkg config contains non-UTF-8 data");

	// format: `VAR=VALUE\0`
	let config_entries = config.split_terminator('\0').map(|line| {
		let sep_pos = line.find('=').expect("Malformed config loader output");
		(&line[..sep_pos], &line[sep_pos + 1..])
	});

	// config entries won't appear here unless set
	for (var, value) in config_entries {
		debug!("makepkg option: {} = {:?}", var, value);

		match var {
			"PKGDEST" | "SRCDEST" | "SRCPKGDEST" | "LOGDEST" | "BUILDDIR" => {
				let warn = "WARNING".yellow();
				eprintln!("{}: custom ${} location is not supported.", warn, var);
			}

			"PKGEXT" => match value {
				".pkg.tar" | ".pkg.tar.xz" | ".pkg.tar.lzma" | ".pkg.tar.gz" | ".pkg.tar.gzip"
				| ".pkg.tar.zst" | ".pkg.tar.zstd" => {
					pkgext = Some(value.to_owned());
				}

				_ => panic!(
					"$PKGEXT is set to an unsupported value: {:?}. \
					Only .pkg.tar or .pkg.tar.xz or .pkg.tar.gz or .pkg.tar.zst archives are \
					allowed for now. RUA needs those extensions to look inside the archives for \
					'tar_check' analysis.",
					value
				),
			},

			_ => {}
		}
	}

	for &var in &["PKGDEST", "SRCDEST", "SRCPKGDEST", "LOGDEST", "BUILDDIR"] {
		env::set_var(var, "/dev/null"); // make sure we override it later
	}

	RuaEnv {
		dirs,
		pkgext: pkgext.expect("Internal error: no $PKGEXT entry in makepkg configuration?!"),
	}
}

pub fn sudo_command() -> String {
	std::env::var(cli_args::SUDO_ENVIRONMENT_VARIABLE_NAME).unwrap_or_else(|_| "sudo".to_string())
}
