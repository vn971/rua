use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::PathBuf;

pub struct FlatSrcinfo {
	map: HashMap<String, Vec<String>>,
	empty_vec: Vec<String>,
}

impl FlatSrcinfo {
	pub fn new(path: PathBuf) -> FlatSrcinfo {
		let mut result: HashMap<String, Vec<String>> = HashMap::new();
		let file = File::open(&path).unwrap();
		let file = BufReader::new(file);
		for line in file.lines() {
			let line = line.expect(&format!("Failed to parse .SRCINFO in {:?}", path));
			let line = line.trim();
			if line.is_empty() || line.starts_with('#') { continue; }
			let split: Vec<&str> = line.splitn(2, '=').map(|s| s.trim()).collect();
			let key = split.get(0).expect(&format!("Unexpected line {} in .SRCINFO", line)).to_string();
			assert!(!key.is_empty(), "Unexpected empty key in .SRCINFO");
			let value = split.get(1).expect(&format!("Unexpected line {} in .SRCINFO", line)).to_string();
			assert!(!value.is_empty(), "Unexpected empty value in .SRCINFO");
			if result.contains_key(&key) {
				result.get_mut(&key).map(|vec| vec.push(value));
			} else {
				result.insert(key.to_string(), vec![value]);
			}
		}
		FlatSrcinfo { map: result, empty_vec: Vec::new() }
	}
	pub fn get(&self, key: &str) -> &Vec<String> {
		self.map.get(key).unwrap_or(&self.empty_vec)
	}
}


pub fn static_pkgbuild(path: PathBuf) -> String {
	let mut bash = Vec::new();
	let file = File::open(&path).expect(&format!("Cannot find file {:?}", path));
	let file = BufReader::new(file);
	for line in file.lines() {
		let line = line.expect(&format!("Failed to parse .SRCINFO in {:?}", path));
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') { continue; }
		let split: Vec<&str> = line.splitn(2, '=').map(|s| s.trim()).collect();
		let key = split.get(0).expect(&format!("Unexpected line {} in .SRCINFO", line)).to_string();
		lazy_static! {
			static ref key_regex: Regex = Regex::new(r"[a-zA-Z][a-zA-Z_]*").unwrap();
		}
		assert!(key_regex.is_match(&key), "unexpected SRCINFO key {}", key);
		let value = split.get(1).expect(&format!("Unexpected line {} in .SRCINFO", line)).to_string();
		lazy_static! {
			static ref value_regex: Regex = Regex::new(r"[^']*").unwrap();
		}
		assert!(value_regex.is_match(&value), "unexpected SRCINFO value {}", value);
		bash.push(format!("{}+=( '{}' )", key, value));
	}
	bash.push("unset pkgdesc; pkgdesc=ignore;".to_owned());
	bash.push("unset pkgver; pkgver=1;".to_owned());
	bash.push("unset pkgrel; pkgrel=1;".to_owned());
	bash.push("unset url; url=ignore;".to_owned());
	bash.join("\n")
}
