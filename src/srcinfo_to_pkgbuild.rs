use std::path::PathBuf;

use srcinfo::{ArchVec, Srcinfo};

fn escape_value(value: &str) -> String {
	value.replace("'", "'\\''")
}

fn push_field(pkgbuild: &mut String, key: &str, value: &str) {
	pkgbuild.push_str(&format!("{}='{}'\n", key, escape_value(value)));
}

fn push_array(pkgbuild: &mut String, key: &str, values: &[String]) {
	pkgbuild.push_str(&format!("{}=(", key));

	for value in values {
		pkgbuild.push_str(&format!("\n  '{}'", escape_value(value)))
	}

	pkgbuild.push_str(")\n");
}

fn push_arrays(pkgbuild: &mut String, key: &str, arch_values: &[ArchVec]) {
	for values in arch_values {
		if let Some(ref arch) = values.arch {
			let key = &format!("{}_{}", key, arch);
			push_array(pkgbuild, key, &values.vec);
		} else {
			push_array(pkgbuild, key, &values.vec);
		};
	}
}

pub fn static_pkgbuild(path: PathBuf) -> String {
	let srcinfo = Srcinfo::parse_file(&path)
		.unwrap_or_else(|e| panic!("{}:{} Failed to parse {:?}, {}", file!(), line!(), &path, e));
	let mut pkgbuild = String::new();

	push_field(&mut pkgbuild, "pkgname", "tmp");
	push_field(&mut pkgbuild, "pkgver", "1");
	push_field(&mut pkgbuild, "pkgrel", "1");
	push_array(&mut pkgbuild, "arch", &srcinfo.pkg.arch);
	push_arrays(&mut pkgbuild, "source", &srcinfo.base.source);
	push_arrays(&mut pkgbuild, "md5sums", &srcinfo.base.md5sums);
	push_arrays(&mut pkgbuild, "sha1sums", &srcinfo.base.sha1sums);
	push_arrays(&mut pkgbuild, "sha224sums", &srcinfo.base.sha224sums);
	push_arrays(&mut pkgbuild, "sha256sums", &srcinfo.base.sha256sums);
	push_arrays(&mut pkgbuild, "sha384sums", &srcinfo.base.sha384sums);
	push_arrays(&mut pkgbuild, "sha512sums", &srcinfo.base.sha512sums);

	pkgbuild
}
