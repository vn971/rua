use std::fs::File;
use std::io::Read;
use std::io;
use std::path::PathBuf;
use std::process;
use tar::*;
use util;


pub fn tar_check(package_path: PathBuf) {
	let package_str = package_path.to_str().unwrap();
	let mut install_file = String::new();
	let mut archive = Archive::new(File::open(&package_path).expect(&format!("cannot open file {}", package_str)));
	let archive_files = archive.entries().expect(&format!("cannot open archive {}", package_str));
	for file in archive_files {
		let mut file = file.expect(&format!("cannot access tar file in {}", package_str));
		let mode = file.header().mode().unwrap();
		if mode > 0o777 {
			eprintln!("ERROR! File {} / {:?} has mode {}, which is out of 0o777 permission zone",
				package_str, file.header().path(), mode);
			process::exit(-1);
		}
		if file.header().path().unwrap().to_str() == Some(".INSTALL") {
			file.read_to_string(&mut install_file).unwrap();
		}
	}
	loop {
		let has_install = !install_file.is_empty();
		eprint!("\nPackage {} has no SUID files.\n\
			[E]=list executable files, [L]=list all files, {}[S]=run shell to inspect, [O]=ok, proceed. ",
			package_str,
			if has_install { "[I]=show install file, " } else { "" }
		);
		let mut string = String::new();
		io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
		eprintln!();
		let string = string.trim().to_lowercase();
		if string == "l" {
			for file in Archive::new(File::open(&package_path).unwrap()).entries().unwrap() {
				let mut file = file.unwrap();
				let path = file.header().path().unwrap();
				let path = path.to_str().unwrap();
				let is_normal = !path.ends_with("/") && !path.starts_with(".");
				if is_normal {
					eprintln!("{}", path);
				}
			}
		} else if string == "e" {
			for file in Archive::new(File::open(&package_path).unwrap()).entries().unwrap() {
				let mut file = file.unwrap();
				let mode = file.header().mode().unwrap();
				let path = file.header().path().unwrap();
				let path = path.to_str().unwrap();
				let is_normal = !path.ends_with("/") && !path.starts_with(".");
				if is_normal && (mode & 0o111 > 0) {
					eprintln!("{}", path);
				}
			}
		} else if string == "i" && has_install {
			eprintln!("{}", &install_file);
		} else if string == "s" {
			eprintln!("Exit the shell with `logout` or Ctrl-D...");
			util::run_env_command("SHELL", "bash", &[]);
		} else if string == "o" {
			break;
		}
	}
}
