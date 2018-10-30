use std::fs::File;
use std::io::Read;
use std::io;
use std::path::PathBuf;
use tar::*;
use util;


pub fn tar_check(tar_file: PathBuf) {
	let tar_str = tar_file.to_str().unwrap();
	let mut install_file = String::new();
	let mut all_files = Vec::new();
	let mut executable_files = Vec::new();
	let mut suid_files = Vec::new();

	let mut archive = Archive::new(File::open(&tar_file)
		.expect(&format!("cannot open file {}", tar_str)));
	let archive_files = archive.entries().expect(&format!("cannot open archive {}", tar_str));
	for file in archive_files {
		let mut file = file.expect(&format!("cannot access tar file in {}", tar_str));
		let path = {
			let path = file.header().path()
				.expect(&format!("Failed to extract tar file metadata for file in {}", tar_str));
			path.to_str().unwrap().to_owned()
		};
		let mode = file.header().mode().unwrap();
		let is_normal = !path.ends_with("/") && !path.starts_with(".");
		if is_normal { all_files.push(path.clone()); }
		if is_normal && (mode & 0o111 > 0) { executable_files.push(path.clone()); }
		if mode > 0o777 { suid_files.push(path.clone()); }
		if &path == ".INSTALL" {
			file.read_to_string(&mut install_file)
				.expect(&format!("Failed to read INSTALL script from tar file {}", tar_str));
		}
	}

	let has_install = !install_file.is_empty();
	let notice = {
		let suid_warning = if suid_files.is_empty() {
			format!("Package {} has no SUID files.\n", tar_str)
		} else {
			format!("!!!WARNING!!! Package {} has SUID files.\n[S]=list SUID files, ", tar_str)
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
