use crate::alpm_wrapper::AlpmWrapper;
use itertools::Itertools;
use std::process::{Command, Stdio};

pub struct AlpmImpl {}

pub fn new() -> AlpmImpl {
	AlpmImpl {}
}

impl AlpmWrapper for AlpmImpl {
	fn is_package_installed(&self, package: &str) -> bool {
		Command::new("pacman")
			.arg("-T")
			.arg(&package)
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.expect("Failed to execute pacman")
			.success()
	}

	fn is_installed(&self, package: &str) -> bool {
		Command::new("pacman")
			.arg("-Sddp")
			.arg(&package)
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.expect("Failed to execute pacman")
			.success()
	}

	fn list_foreign_packages(&self) -> Vec<String> {
		let output = Command::new("pacman")
			.arg("-Qqm")
			.output()
			.expect("Failed to execute pacman");
		if !output.status.success() {
			eprintln!("Failed to get foreign packages via pacman -Qqm");
			std::process::exit(1)
		}
		let output = String::from_utf8(output.stdout)
			.expect("Failed to convert pacman -Qqm output to unicode");
		output.lines().map(|s| s.to_string()).collect_vec()
	}

	fn is_package_older_than(&self, package: &str, version: &str) -> bool {
		let full = format!("{}<{}", package, version);
		execute_pacman_and_suppress_output(&["-T", &full])
	}
}

fn execute_pacman_and_suppress_output(args: &[&str]) -> bool {
	Command::new("pacman")
		.args(args)
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.expect("Failed to execute pacman")
		.success()
}
