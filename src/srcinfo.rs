use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use srcinfo::Srcinfo;

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

fn push_field(pkgbuild: &mut String, field: &str, s: &str) {
	let s = s.replace("'", "'\\''");
	pkgbuild.push_str(&format!("{}='{}'\n", field, s));
}

fn push_array(pkgbuild: &mut String, field: &str, items: &[String]) {
	pkgbuild.push_str(&format!("{}=(", field));

	for item in items {
		pkgbuild.push_str(&format!("\n  '{}'", item.replace("'", "'\\''")))
	}

	pkgbuild.push_str(")\n");
}

pub fn static_pkgbuild(path: PathBuf) -> String {
	let srcinfo = Srcinfo::parse_file(path).expect("Failed to parse srcinfo");
	let mut pkgbuild = String::new();

	push_field(&mut pkgbuild, "pkgname", "tmp");
	push_field(&mut pkgbuild, "pkgver", "1");
	push_field(&mut pkgbuild, "pkgrel", "1");
	push_array(&mut pkgbuild, "arch", &srcinfo.pkg.arch);

	for source in &srcinfo.base.source {
		if let Some(ref arch) = source.arch {
			let field = format!("{}_{}", "source", arch);
			push_array(&mut pkgbuild, &field, &source.vec);
		} else {
			push_array(&mut pkgbuild, "source", &source.vec);
		};
	}

	pkgbuild
}
