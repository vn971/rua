use std::path::PathBuf;

use srcinfo::{ArchVec, Srcinfo};

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

fn push_arch_vec(pkgbuild: &mut String, field: &str, items: &[ArchVec]) {
	for source in items {
		if let Some(ref arch) = source.arch {
			let field = &format!("{}_{}", field, arch);
			push_array(pkgbuild, field, &source.vec);
		} else {
			push_array(pkgbuild, field, &source.vec);
		};
	}
}

pub fn static_pkgbuild(path: PathBuf) -> String {
	let srcinfo = Srcinfo::parse_file(path).expect("Failed to parse srcinfo");
	let mut pkgbuild = String::new();

	push_field(&mut pkgbuild, "pkgname", "tmp");
	push_field(&mut pkgbuild, "pkgver", "1");
	push_field(&mut pkgbuild, "pkgrel", "1");
	push_array(&mut pkgbuild, "arch", &srcinfo.pkg.arch);
	push_arch_vec(&mut pkgbuild, "source", &srcinfo.base.source);
	push_arch_vec(&mut pkgbuild, "md5sums", &srcinfo.base.md5sums);
	push_arch_vec(&mut pkgbuild, "sha1sums", &srcinfo.base.sha1sums);
	push_arch_vec(&mut pkgbuild, "sha224sums", &srcinfo.base.sha224sums);
	push_arch_vec(&mut pkgbuild, "sha256sums", &srcinfo.base.sha256sums);
	push_arch_vec(&mut pkgbuild, "sha384sums", &srcinfo.base.sha384sums);
	push_arch_vec(&mut pkgbuild, "sha512sums", &srcinfo.base.sha512sums);

	pkgbuild
}
