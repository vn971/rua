use std::ops::Not;
use std::path::Path;

use anyhow::Context;
use indexmap::IndexMap;
use indexmap::IndexSet;
use log::trace;
use log::warn;
use srcinfo::ArchVec;

use crate::alpm_wrapper::AlpmWrapper;
use crate::aur_rpc_utils::aur_info;
use crate::aur_rpc_utils::clean_and_check_package_name;
use crate::pacman::PACMAN_ARCH;
use crate::rua_paths::RuaPaths;
use crate::wrapped::generate_srcinfo;

#[derive(Debug)]
pub struct PkgInfo {
	name: String,
	base: String,
	dependencies: IndexSet<String>,
}
impl PkgInfo {
	fn new(name: String, base: String, dependencies: IndexSet<String>) -> Self {
		Self {
			name,
			base,
			dependencies,
		}
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn pkg_base(&self) -> &str {
		&self.base
	}
}
impl From<raur::Package> for PkgInfo {
	fn from(pkg: raur::Package) -> Self {
		let make_deps = pkg.make_depends.iter();
		let check_deps = pkg.check_depends.iter();
		let flat_deps = pkg.depends.iter();
		let deps = make_deps
			.chain(flat_deps)
			.chain(check_deps)
			.map(|d| clean_and_check_package_name(d))
			.collect::<IndexSet<_>>();
		Self::new(pkg.name, pkg.package_base, deps)
	}
}

pub fn srcinfo_to_pkginfo(src: srcinfo::Srcinfo) -> Vec<PkgInfo> {
	let makedepends =
		ArchVec::supported(&src.base.makedepends, PACMAN_ARCH.to_string()).collect::<IndexSet<_>>();
	let checkdepends = ArchVec::supported(&src.base.checkdepends, PACMAN_ARCH.to_string())
		.collect::<IndexSet<_>>();
	src.pkgs
		.into_iter()
		.map(|pkg| {
			let deps = ArchVec::supported(&pkg.depends, PACMAN_ARCH.to_string())
				.chain(makedepends.clone())
				.chain(checkdepends.clone())
				.map(clean_and_check_package_name)
				.collect();
			PkgInfo::new(pkg.pkgname, src.base.pkgbase.clone(), deps)
		})
		.collect()
}

pub type Infos = IndexMap<String, PkgInfo>;
pub type PacmanDependencies = IndexSet<String>;
pub type DepthMap = IndexMap<String, usize>;

#[derive(Debug)]
pub struct ToInstall {
	infos: Infos,
	pacman_deps: PacmanDependencies,
	depths: DepthMap,
}
impl ToInstall {
	pub fn new(root_packages: &[String]) -> Self {
		let dm = root_packages
			.iter()
			.map(|p| (p.to_string(), 0))
			.collect::<DepthMap>();
		Self {
			infos: Infos::new(),
			pacman_deps: PacmanDependencies::new(),
			depths: dm,
		}
	}
	pub fn into_inner(self) -> (Infos, PacmanDependencies, DepthMap) {
		let Self {
			infos: is,
			pacman_deps: pacs,
			depths: ds,
		} = self;
		(is, pacs, ds)
	}
	pub fn add_package(
		&mut self,
		info: PkgInfo,
		alpm: &dyn AlpmWrapper,
	) -> Result<Vec<String>, anyhow::Error> {
		let mut new_aur_dependencies = vec![];
		for dependency in &info.dependencies {
			if self.pacman_deps.contains(dependency) {
				trace!("{dependency} is already resolved as a pacman dependency");
			} else if alpm
				.is_installed(dependency)
				.context("Could not determine if dependency is installed")?
			{
				trace!("{dependency} is already installed");
			} else if alpm
				.is_installable(dependency)
				.context("Could not determine if dependency is installable")?
			{
				self.pacman_deps.insert(dependency.to_owned());
			} else {
				if self.depths.contains_key(dependency) {
					eprintln!("Skipping already resolved dependency {}", dependency);
					continue;
				}

				eprintln!(
					"Package {} depends on {}. Resolving...",
					info.name, dependency
				);
				new_aur_dependencies.push(dependency.to_string());

				let parent_depth = self
					.depths
					.get(&info.name)
					.expect("Internal error: queue element does not have depth");
				let new_depth = self
					.depths
					.get(dependency)
					.map_or(parent_depth + 1, |d| (*d).max(parent_depth + 1));
				self.depths.insert(dependency.to_string(), new_depth);
			}
		}
		self.infos.insert(info.name().to_string(), info);
		Ok(new_aur_dependencies)
	}

	pub fn infos(&self) -> &Infos {
		&self.infos
	}

	pub fn pacman_deps(&self) -> &PacmanDependencies {
		&self.pacman_deps
	}

	pub fn depths(&self) -> &DepthMap {
		&self.depths
	}

	pub fn not_found(&self) -> Vec<&str> {
		self.depths()
			.keys()
			.filter_map(|pkg| {
				(self.infos().contains_key(pkg))
					.not()
					.then_some(pkg.as_str())
			})
			.collect::<Vec<_>>()
	}

	pub fn update(&mut self, dir: &Path, rua_paths: &RuaPaths, alpm: &dyn AlpmWrapper) {
		let dir_str = dir
			.to_str()
			.unwrap_or_else(|| panic!("{}:{} Cannot parse CLI target directory", file!(), line!()));

		let src = match generate_srcinfo(dir_str, rua_paths) {
			Ok(src) => src,
			Err(e) => {
				warn!(
					"Could not generate .SRCINFO: {e}
The dependency tree will not be rebuilt!"
				);
				return;
			}
		};
		let pkgs = srcinfo_to_pkginfo(src);

		for pkg in pkgs {
			let new_deps = self.add_package(pkg, alpm);
			match new_deps {
				Ok(nd) => {
					let e = aur_info(nd, self, alpm);
					if let Err(e) = e {
						warn!(
							"Could not process new dependencies: {e}
The dependency tree will not be rebuilt!"
						);
					}
				}
				Err(e) => {
					warn!(
						"Could not add package to dependency tree: {e}
The dependency tree will not be rebuilt!",
					);
				}
			}
		}
	}
}
