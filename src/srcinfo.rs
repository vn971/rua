use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use lazy_static::lazy_static;
use regex::Regex;

pub struct FlatSrcinfo {
	map: HashMap<String, Vec<String>>,
	empty_vec: Vec<String>,
}

impl FlatSrcinfo {
	pub fn new(path: PathBuf) -> FlatSrcinfo {
		let mut result: HashMap<String, Vec<String>> = HashMap::new();
		let file =
			File::open(&path).unwrap_or_else(|_| panic!("Cannot open SRCINFO at path {:?}", path));
		let file = BufReader::new(file);
		for line in file.lines() {
			let line = line.unwrap_or_else(|_| panic!("Failed to parse .SRCINFO in {:?}", path));
			let line = line.trim();
			if line.is_empty() || line.starts_with('#') {
				continue;
			}
			let split: Vec<&str> = line.splitn(2, '=').map(|s| s.trim()).collect();
			let key = split
				.get(0)
				.unwrap_or_else(|| panic!("Unexpected line {} in .SRCINFO", line))
				.to_string();
			assert!(!key.is_empty(), "Unexpected empty key in .SRCINFO");
			let value = split
				.get(1)
				.unwrap_or_else(|| panic!("Unexpected line {} in .SRCINFO", line))
				.to_string();
			assert!(!value.is_empty(), "Unexpected empty value in .SRCINFO");
			if let Some(vec) = result.get_mut(&key) {
				vec.push(value);
			} else {
				result.insert(key.to_string(), vec![value]);
			}
		}
		FlatSrcinfo {
			map: result,
			empty_vec: Vec::new(),
		}
	}
	pub fn get(&self, key: &str) -> &Vec<String> {
		self.map.get(key).unwrap_or(&self.empty_vec)
	}
}

pub fn static_pkgbuild(path: PathBuf) -> String {
	let unary_keys = [
		"epoch",
		"install",
		"changelog",
		"pkgdesc",
		"pkgrel",
		"pkgver",
		"url",
	];
	let mut bash = Vec::new();
	let file = File::open(&path).unwrap_or_else(|_| panic!("Cannot open SRCINFO in {:?}", path));
	let file = BufReader::new(file);
	for line in file.lines() {
		let line = line.unwrap_or_else(|_| panic!("Failed to parse .SRCINFO in {:?}", path));
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let split: Vec<&str> = line.splitn(2, '=').map(|s| s.trim()).collect();
		let key = split
			.get(0)
			.unwrap_or_else(|| panic!("Unexpected line {} in .SRCINFO", line))
			.to_string();
		lazy_static! {
			static ref KEY_REGEX: Regex = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_]*$").unwrap();
		}
		assert!(KEY_REGEX.is_match(&key), "unexpected SRCINFO key {}", key);
		let value = split
			.get(1)
			.unwrap_or_else(|| panic!("Unexpected line {} in .SRCINFO", line))
			.replace("'", "'\\''")
			.to_string();
		if unary_keys.contains(&key.as_str()) {
			bash.push(format!("{}='{}'", key, value));
		} else {
			bash.push(format!("{}+=('{}')", key, value));
		}
	}
	bash.join("\n")
}
