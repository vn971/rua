use std::path::PathBuf;

use srcinfo::Srcinfo;

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
