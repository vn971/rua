use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

pub static AUR_RPC_URL: &str = "https://aur.archlinux.org/rpc/";

/// The package info that a query will return from the AUR RPC.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Package {
	#[serde(rename = "ID")]
	pub id: u32,
	pub name: String,
	#[serde(rename = "PackageBaseID")]
	pub package_base_id: u32,
	pub package_base: String,
	pub version: String,
	pub description: Option<String>,
	#[serde(default, rename = "URL")]
	pub url: Option<String>,
	pub num_votes: u32,
	pub popularity: f64,
	pub out_of_date: Option<i64>,
	pub maintainer: Option<String>,
	pub submitter: Option<String>,
	pub first_submitted: i64,
	pub last_modified: i64,
	#[serde(default, rename = "URLPath")]
	pub url_path: String,
	#[serde(default)]
	pub groups: Vec<String>,
	#[serde(default)]
	pub depends: Vec<String>,
	#[serde(default)]
	pub make_depends: Vec<String>,
	#[serde(default)]
	pub opt_depends: Vec<String>,
	#[serde(default)]
	pub check_depends: Vec<String>,
	#[serde(default)]
	pub conflicts: Vec<String>,
	#[serde(default)]
	pub replaces: Vec<String>,
	#[serde(default)]
	pub provides: Vec<String>,
	#[serde(default)]
	pub license: Vec<String>,
	#[serde(default)]
	pub keywords: Vec<String>,
	#[serde(default, rename = "CoMaintainers")]
	pub co_maintainers: Vec<String>,
}

#[derive(Deserialize)]
struct RpcResponse {
	#[serde(rename = "type")]
	response_type: String,
	error: Option<String>,
	results: Vec<Package>,
}

/// A handle for making AUR RPC requests.
pub struct Handle {
	client: reqwest::blocking::Client,
	url: String,
}

impl Handle {
	/// Creates a new Handle configured with rua's User-Agent header.
	pub fn new() -> Result<Self> {
		let client = reqwest::blocking::Client::builder()
			.user_agent(concat!("rua/", env!("CARGO_PKG_VERSION")))
			.build()
			.context("Failed to build HTTP client for AUR RPC")?;
		Ok(Self {
			client,
			url: AUR_RPC_URL.to_string(),
		})
	}

	/// Performs an AUR info request for package names, splitting requests into chunks of 200.
	pub fn info<S: AsRef<str>>(&self, pkg_names: &[S]) -> Result<Vec<Package>> {
		let mut packages = Vec::with_capacity(pkg_names.len());
		for chunk in pkg_names.chunks(200) {
			let mut params = chunk
				.iter()
				.map(|name| ("arg[]", name.as_ref()))
				.collect::<Vec<_>>();
			params.extend([("v", "5"), ("type", "info")]);
			packages.extend(self.request(&params)?);
		}
		Ok(packages)
	}

	/// Performs an AUR search request by name and description.
	pub fn search<S: AsRef<str>>(&self, query: S) -> Result<Vec<Package>> {
		self.request(&[
			("v", "5"),
			("type", "search"),
			("by", "name-desc"),
			("arg", query.as_ref()),
		])
	}

	fn request(&self, params: &[(&str, &str)]) -> Result<Vec<Package>> {
		let response = self
			.client
			.post(&self.url)
			.form(params)
			.send()
			.context("AUR RPC request failed")?;
		let response = response
			.error_for_status()
			.context("AUR RPC returned error status")?;
		let response: RpcResponse = response
			.json()
			.context("Failed to decode AUR RPC response as JSON")?;

		if response.response_type == "error" {
			bail!(
				"AUR RPC error: {}",
				response
					.error
					.unwrap_or_else(|| "No error message provided".to_string())
			);
		}
		Ok(response.results)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_handle_new() {
		assert!(Handle::new().is_ok());
	}

	#[test]
	fn test_deserialize_package() {
		let json = r#"{
			"ID": 1,
			"Name": "test-pkg",
			"PackageBaseID": 10,
			"PackageBase": "test-pkg",
			"Version": "1.0.0-1",
			"Description": "A test package",
			"NumVotes": 5,
			"Popularity": 0.5,
			"FirstSubmitted": 1000000,
			"LastModified": 2000000,
			"URLPath": "/test.tar.gz",
			"Depends": ["bash"]
		}"#;
		let pkg: Package = serde_json::from_str(json).unwrap();
		assert_eq!(pkg.name, "test-pkg");
		assert_eq!(pkg.version, "1.0.0-1");
		assert_eq!(pkg.depends, vec!["bash"]);
	}
}
