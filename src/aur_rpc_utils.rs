use crate::alpm_impl::AlpmImpl;
use crate::alpm_wrapper::AlpmWrapper;
use indexmap::IndexMap;
use indexmap::IndexSet;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::trace;
use raur::Package;
use regex::Regex;

type RaurInfo = IndexMap<String, Package>;
type PacmanDependencies = IndexSet<String>;
type DepthMap = IndexMap<String, i32>;
type RecursiveInfo = (RaurInfo, PacmanDependencies, DepthMap);

pub fn recursive_info(
	root_packages_to_process: &[String],
	alpm: &AlpmImpl,
) -> Result<RecursiveInfo, raur::Error> {
	let mut queue: Vec<String> = Vec::from(root_packages_to_process);
	let mut depth_map = IndexMap::new();
	for pkg in &queue {
		depth_map.insert(pkg.to_string(), 0);
	}
	let mut pacman_deps: IndexSet<String> = IndexSet::new();
	let mut info_map: IndexMap<String, Package> = IndexMap::new();
	while !queue.is_empty() {
		let split_at = queue.len().max(200) - 200;
		let to_process = queue.split_off(split_at);
		trace!("to_process: {:?}", to_process);
		for info in raur::info(&to_process)? {
			let lower_dependencies = info
				.make_depends
				.iter()
				.map(|d| (clean_and_check_package_name(d), 1));
			let flat_dependencies = info
				.depends
				.iter()
				.map(|d| (clean_and_check_package_name(d), 0));
			let deps = lower_dependencies.chain(flat_dependencies).collect_vec();

			for (dependency, depth_diff) in deps.into_iter() {
				if alpm.is_package_installed(&dependency) {
					// skip if already installed
				} else if !alpm.is_installed(&dependency) {
					if !depth_map.contains_key(&dependency) {
						eprintln!(
							"Package {} depends on {}. Resolving...",
							info.name, dependency
						);
						queue.push(dependency.to_string())
					} else {
						eprintln!("Skipping already resolved dependency {}", dependency);
					}
					let parent_depth = depth_map
						.get(&info.name)
						.expect("Internal error: queue element does not have depth");
					let new_depth = depth_map
						.get(&dependency)
						.map_or(parent_depth + depth_diff, |d| {
							(*d).max(parent_depth + depth_diff)
						});
					depth_map.insert(dependency.to_string(), new_depth);
				} else {
					pacman_deps.insert(dependency.to_owned());
				}
			}
			info_map.insert(info.name.to_string(), info);
		}
	}
	Ok((info_map, pacman_deps, depth_map))
}

fn clean_and_check_package_name(name: &str) -> String {
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
	let name: String = CLEANUP.replace_all(name, "").to_lowercase();
	lazy_static! {
		// From PKGBUILD manual page:
		// Valid characters are alphanumerics, and any of the following characters: “@ . _ + -”.
		// Additionally, names are not allowed to start with hyphens or dots.
		static ref NAME_REGEX: Regex = Regex::new(r"^[a-z0-9@_+][a-z0-9@_+.-]*$").unwrap_or_else(
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
	fn test_starting_hyphen() {
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
	}
}
