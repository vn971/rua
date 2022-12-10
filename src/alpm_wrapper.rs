use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use itertools::Itertools;
use std::cmp::Ordering;
use std::process::Command;
use std::process::Stdio;

pub fn new_alpm_wrapper() -> Box<dyn AlpmWrapper> {
	Box::new(AlpmBinWrapper {})
}

pub trait AlpmWrapper {
	/// Checks if either this package is installed, or anything that provides the name is
	fn is_installed(&self, package: &str) -> Result<bool>;

	/// Checks if either this package is installable, or anything that provides the name is
	fn is_installable(&self, package: &str) -> Result<bool>;

	/// Returns a list of (package, version)
	fn get_non_pacman_packages(&self) -> Result<Vec<(String, String)>>;

	/// Compares package versions according to
	/// https://archlinux.org/pacman/vercmp.8.html
	fn version_compare(&self, a: &str, b: &str) -> Result<Ordering>;
}

struct AlpmBinWrapper {}

impl AlpmWrapper for AlpmBinWrapper {
	fn is_installed(&self, package: &str) -> Result<bool> {
		let result = Command::new("pacman")
			.args(["-Qi", "--", package])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_err(|err| {
				anyhow!(
					"Failed to determine if package {} is installed, {}",
					package,
					err
				)
			})?
			.success();
		Ok(result)
	}

	fn is_installable(&self, package: &str) -> Result<bool> {
		let result = Command::new("pacman")
			.args(["-Sddp", "--", package])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_err(|err| {
				anyhow!(
					"Failed to determine if package {} is installable, {}",
					package,
					err
				)
			})?;
		Ok(result.success())
	}

	fn get_non_pacman_packages(&self) -> Result<Vec<(String, String)>> {
		let mut command = Command::new("pacman");
		command.args(["-Q", "--foreign", "--color=never"]);
		let output = command.output().context("failed to execute pacman")?;
		let stdout =
			String::from_utf8(output.stdout).context("failed to parse pacman output as utf8")?;
		let mut result = Vec::new();
		for line in stdout.lines() {
			let split: Vec<_> = line.split(' ').collect_vec();
			match split[..] {
				[package, version] => result.push((package.to_string(), version.to_string())),
				_ => {
					return Err(anyhow!(
						"Failed to parse (package,version) from pacman line {}",
						line
					))
				}
			}
		}
		Ok(result)
	}

	fn version_compare(&self, a: &str, b: &str) -> Result<Ordering> {
		let mut command = Command::new("vercmp");
		command.args([a, b]);
		let output = command.output().context("Failed to execute vercmp")?;
		let stdout =
			String::from_utf8(output.stdout).context("Failed to parse vercmp response as utf8")?;
		let stdout = stdout.trim();
		let asint = stdout
			.parse::<i64>()
			.with_context(|| format!("Failed to parse vercmp response as i64: {}", stdout))?;
		Ok(asint.cmp(&0i64))
	}
}

#[cfg(test)]
#[cfg(feature = "testpacman")]
mod tests {
	use crate::alpm_wrapper::AlpmBinWrapper;
	use crate::alpm_wrapper::AlpmWrapper;

	#[test]
	fn test_alpm_bin_wrapper() {
		let alpm_bin = AlpmBinWrapper {};
		let packages = alpm_bin.get_non_pacman_packages().unwrap();
		assert!(packages.iter().all(|(pkg, _ver)| pkg != "pacman"));
	}
}
