use std::fs::File;
use std::io::Read;
use std::io;
use std::path::PathBuf;
use tar::*;
use util;


pub fn tar_check(package_path: PathBuf) {
	let package_str = package_path.to_str().unwrap();
	let mut install_file = String::new();
	let mut all_files = Vec::new();
	let mut executable_files = Vec::new();
	let mut suid_files = Vec::new();

	let mut archive = Archive::new(File::open(&package_path).expect(&format!("cannot open file {}", package_str)));
	let archive_files = archive.entries().expect(&format!("cannot open archive {}", package_str));
	for file in archive_files {
		let mut file = file.expect(&format!("cannot access tar file in {}", package_str));
		let mode = file.header().mode().unwrap();
		let path = {
			let path = file.header().path().unwrap();
			path.to_str().unwrap().to_owned()
		};
		let is_normal = !path.ends_with("/") && !path.starts_with(".");
		if is_normal { all_files.push(path.clone()); }
		if is_normal && (mode & 0o111 > 0) { executable_files.push(path.clone()); }
		if mode > 0o777 { suid_files.push(path.clone()); }
		if &path == ".INSTALL" {
			file.read_to_string(&mut install_file).unwrap();
		}
	}

	let has_install = !install_file.is_empty();
	let notice = {
		let suid_warning = if suid_files.is_empty() {
			format!("Package {} has no SUID files.\n", package_str)
		} else {
			format!("!!!WARNING!!! Package {} has SUID files.\n[S]=list SUID files, ", package_str)
		};
		format!("\n{}\
			{}[E]=list executable files, [L]=list all files, \
			[T]=run shell to inspect, [O]=ok, proceed. ",
			suid_warning,
			if has_install { "[I]=show install file, " } else { "" }
		)
	};
	loop {
		eprint!("{}", notice);
		let mut string = String::new();
		io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
		eprintln!();
		let string = string.trim().to_lowercase();
		if string == "s" && !suid_files.is_empty() {
			for path in &suid_files {
				eprintln!("{}", path);
			}
		} else if string == "e" {
			for path in &executable_files {
				eprintln!("{}", path);
			}
		} else if string == "l" {
			for path in &all_files {
				eprintln!("{}", path);
			}
		} else if string == "i" && has_install {
			eprintln!("{}", &install_file);
		} else if string == "t" {
			eprintln!("Exit the shell with `logout` or Ctrl-D...");
			util::run_env_command("SHELL", "bash", &[]);
		} else if string == "o" {
			break;
		}
	}
}
