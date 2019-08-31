use crate::alpm_wrapper::AlpmWrapper;
use alpm::Alpm;
use alpm::SigLevel;
use std::cmp::Ordering;
use std::process::Command;

pub struct AlpmImpl {
	alpm: alpm::Alpm,
}

fn new_local_alpm() -> Alpm {
	let alpm = Alpm::new("/", "/var/lib/pacman"); // default locations on arch linux
	alpm.unwrap_or_else(|err| {
		panic!(
			"{}:{} Failed to initialize alpm library, {}",
			file!(),
			line!(),
			err
		)
	})
}

pub fn new() -> AlpmImpl {
	let alpm = new_local_alpm();
	for repo in get_repository_list() {
		alpm.register_syncdb(&repo, SigLevel::NONE)
			.unwrap_or_else(|e| panic!("Failed to register {} in libalpm, {}", &repo, e));
	}
	AlpmImpl { alpm }
}

impl AlpmWrapper for AlpmImpl {
	fn is_package_installed(&self, name: &str) -> bool {
		self.alpm
			.localdb()
			.pkgs()
			.expect("failed to open alpm.localdb().pkgs()")
			.find_satisfier(name)
			.map_or(false, |sat| sat.install_date().is_some())
	}

	fn is_installed(&self, package: &str) -> bool {
		self.alpm.syncdbs().find_satisfier(package).is_some()
	}

	fn list_foreign_packages(&self) -> Vec<String> {
		let pkg_cache = self
			.alpm
			.localdb()
			.pkgs()
			.expect("Could not get alpm.localdb().pkgs() packages");
		pkg_cache
			.filter(|pkg| !self.is_installed(pkg.name()))
			.map(|pkg| pkg.name().to_string())
			.collect::<Vec<_>>()
	}

	fn is_package_older_than(&self, package: &str, version: &str) -> bool {
		self.alpm
			.localdb()
			.pkgs()
			.expect("failed to open alpm.localdb().pkgs()")
			.find_satisfier(package)
			.map_or(false, |sat| {
				let local_ver = sat.version();
				alpm::vercmp(local_ver, version) == Ordering::Less
			})
	}
}

fn get_repository_list() -> Vec<String> {
	let cmd = Command::new("pacman-conf")
		.arg("--repo-list")
		.output()
		.expect("cannot get repository list: pacman-conf --repo-list");
	let output = String::from_utf8(cmd.stdout)
		.expect("Failed to get repo list from `pacman-conf --repo-list`");
	output.lines().map(ToOwned::to_owned).collect()
}
