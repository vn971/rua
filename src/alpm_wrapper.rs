use crate::error::RuaError;
use anyhow::Context;
use anyhow::Result;
use itertools::Itertools;
use std::cmp::Ordering;
use std::process::Command;
use std::process::Stdio;

#[cfg(feature = "libalpm")]
pub fn new_alpm_wrapper() -> Box<dyn AlpmWrapper> {
	// Create an `Alpm` instance with no registered databases except local
	let alpm = alpm::Alpm::new("/", "/var/lib/pacman"); // default locations on arch linux
	let alpm = alpm.unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to initialize alpm library, {}",
			file!(),
			line!(),
			err
		)
	});
	// register all databases
	for repo in get_repository_list() {
		alpm.register_syncdb(&*repo, alpm::SigLevel::NONE)
			.unwrap_or_else(|e| panic!("Failed to register {} in libalpm, {}", &repo, e));
	}
	Box::new(AlpmLibWrapper { alpm })
}

#[cfg(not(feature = "libalpm"))]
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

#[cfg(feature = "libalpm")]
struct AlpmLibWrapper {
	alpm: alpm::Alpm,
}

#[cfg(feature = "libalpm")]
fn get_repository_list() -> Vec<String> {
	let command = Command::new("pacman-conf")
		.arg("--repo-list")
		.output()
		.expect("cannot execute pacman-conf --repo-list");
	let output = String::from_utf8(command.stdout)
		.expect("Failed to parse output of `pacman-conf --repo-list`");
	output.lines().map(ToOwned::to_owned).collect()
}

#[cfg(feature = "libalpm")]
impl AlpmWrapper for AlpmLibWrapper {
	fn is_installed(&self, package: &str) -> Result<bool> {
		Ok(self
			.alpm
			.localdb()
			.pkgs()
			.find_satisfier(package)
			.map_or(false, |sat| sat.install_date().is_some()))
	}

	fn is_installable(&self, package: &str) -> Result<bool> {
		Ok(self.alpm.syncdbs().find_satisfier(package).is_some())
	}

	fn get_non_pacman_packages(&self) -> Result<Vec<(String, String)>> {
		let all_pkgs = self.alpm.localdb().pkgs();
		let mut result = Vec::new();
		for pkg in all_pkgs.into_iter() {
			let installable = AlpmLibWrapper::is_installable(&self, pkg.name())?;
			if !installable {
				result.push((pkg.name().to_string(), pkg.version().to_string()));
			}
		}
		Ok(result)
	}

	fn version_compare(&self, a: &str, b: &str) -> Result<Ordering> {
		Ok(alpm::vercmp(a, b))
	}
}

struct AlpmBinWrapper {}

impl AlpmWrapper for AlpmBinWrapper {
	fn is_installed(&self, package: &str) -> Result<bool> {
		let result = Command::new("pacman")
			.arg("-Qi")
			.arg(package)
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_err(|err| RuaError {
				msg: format!(
					"Failed to determine if package {} is installed, {}",
					package, err
				),
			})?
			.success();
		Ok(result)
	}

	fn is_installable(&self, package: &str) -> Result<bool> {
		let result = Command::new("pacman")
			.arg("-Sddp")
			.arg(package)
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_err(|err| RuaError {
				msg: format!(
					"Failed to determine if package {} is installable, {}",
					package, err
				),
			})?;
		Ok(result.success())
	}

	fn get_non_pacman_packages(&self) -> Result<Vec<(String, String)>> {
		let mut command = Command::new("pacman");
		command.args(&["-Q", "--foreign", "--color=never"]);
		let output = command.output()?;
		let stdout = String::from_utf8(output.stdout)?;
		let mut result = Vec::new();
		for line in stdout.lines() {
			let split: Vec<_> = line.split(' ').collect_vec();
			match split[..] {
				[package, version] => result.push((package.to_string(), version.to_string())),
				_ => {
					return Err(RuaError {
						msg: format!(
							"Failed to parse (package,version) from pacman line {}",
							line
						),
					}
					.into())
				}
			}
		}
		Ok(result)
	}

	fn version_compare(&self, a: &str, b: &str) -> Result<Ordering> {
		let mut command = Command::new("vercmp");
		command.args(&[a, b]);
		let output = command.output()?;
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
mod tests {
	use crate::alpm_wrapper::AlpmBinWrapper;
	use crate::alpm_wrapper::AlpmWrapper;

	#[test]
	fn test_alpm_bin_wrapper() {
		let alpm_bin = AlpmBinWrapper {};
		let packages = alpm_bin.get_non_pacman_packages().unwrap();
		assert!(packages.iter().all(|(pkg, _ver)| pkg != "pacman"));
		// assert!(packages.iter().any(|(pkg, _ver)| pkg == "rua"));
	}
}
