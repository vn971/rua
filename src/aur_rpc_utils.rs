use crate::alpm_wrapper::AlpmWrapper;
use crate::to_install::PkgInfo;
use crate::to_install::ToInstall;
use anyhow::Context;
use anyhow::Result;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use log::trace;
use raur::blocking::Handle as RaurHandle;
use raur::blocking::Raur;
use raur::Package;
use regex::Regex;

const BATCH_SIZE: usize = 200;

pub fn get_packages_to_install(
	root_packages_to_process: &[String],
	alpm: &dyn AlpmWrapper,
) -> Result<ToInstall> {
	let mut to_install = ToInstall::new(root_packages_to_process);
	aur_info(root_packages_to_process.to_vec(), &mut to_install, alpm)
		.context("Could not build the initial dependency tree")?;
	Ok(to_install)
}

pub fn aur_info(
	mut queue: Vec<String>,
	to_install: &mut ToInstall,
	alpm: &dyn AlpmWrapper,
) -> Result<()> {
	let raur_handle = RaurHandle::default();
	while !queue.is_empty() {
		let split_at = queue.len().max(BATCH_SIZE) - BATCH_SIZE;
		let to_process = queue.split_off(split_at);
		trace!("to_process: {:?}", to_process);
		for info in raur_handle
			.info(&to_process)
			.context("Could not rertieve package info from AUR")?
		{
			let info = PkgInfo::from(info);
			let new_deps = to_install
				.add_package(info, alpm)
				.context("Could not add package")?;
			queue.extend(new_deps)
		}
	}
	Ok(())
}

/// Queries the AUR for the provided given package names and returns a map of all packages
/// that match.
///
/// # Arguments
/// * `packages_to_query` - A slice of package names to find in the AUR
pub fn info_map<S: AsRef<str>>(packages_to_query: &[S]) -> Result<IndexMap<String, Package>> {
	let raur_handle = RaurHandle::new();
	let mut result = IndexMap::new();
	for group in packages_to_query.chunks(BATCH_SIZE) {
		let group_info = raur_handle.info(group)?;
		for pkg in group_info.into_iter() {
			result.insert(pkg.name.to_string(), pkg);
		}
	}
	Ok(result)
}

pub fn clean_and_check_package_name(name: &str) -> String {
	match clean_package_name(name) {
		Some(name) => name,
		None => {
			eprintln!("Unexpected package name {}", name);
			std::process::exit(1)
		}
	}
}

fn clean_package_name(name: &str) -> Option<String> {
	lazy_static! {
		static ref CLEANUP: Regex = Regex::new(r"(=.*|>.*|<.*)").unwrap_or_else(|err| panic!(
			"{}:{} Failed to parse regexp, {}",
			file!(),
			line!(),
			err
		));
	}
	let name: String = CLEANUP.replace_all(name, "").to_string();
	lazy_static! {
		// From PKGBUILD manual page:
		// Valid characters are alphanumerics, and any of the following characters: “@ . _ + -”.
		// Additionally, names are not allowed to start with hyphens or dots.
		static ref NAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9@_+][a-zA-Z0-9@_+.-]*$").unwrap_or_else(
			|err| panic!("{}:{} Failed to parse regexp, {}", file!(), line!(), err)
		);
	}
	if NAME_REGEX.is_match(&name) {
		Some(name)
	} else {
		None
	}
}

#[cfg(test)]
mod tests {
	use crate::aur_rpc_utils::*;

	#[test]
	fn test_clean_package_name() {
		assert_eq!(clean_package_name("test"), Some("test".to_string()));
		assert_eq!(
			clean_package_name("abcdefghijklmnopqrstuvwxyz0123456789@_+.-"),
			Some("abcdefghijklmnopqrstuvwxyz0123456789@_+.-".to_string())
		);

		assert_eq!(clean_package_name(""), None);
		assert_eq!(clean_package_name("-test"), None);
		assert_eq!(clean_package_name(".test"), None);
		assert_eq!(clean_package_name("!"), None);
		assert_eq!(clean_package_name("german_ö"), None);

		assert_eq!(clean_package_name("@"), Some("@".to_string()));
		assert_eq!(clean_package_name("_"), Some("_".to_string()));
		assert_eq!(clean_package_name("+"), Some("+".to_string()));

		assert_eq!(clean_package_name("test>=0"), Some("test".to_string()));
		assert_eq!(clean_package_name("test>0"), Some("test".to_string()));
		assert_eq!(clean_package_name("test<0"), Some("test".to_string()));
		assert_eq!(clean_package_name("test<=0"), Some("test".to_string()));
		assert_eq!(clean_package_name("test=0"), Some("test".to_string()));
		assert_eq!(clean_package_name("test==0"), Some("test".to_string()));
	}
}
