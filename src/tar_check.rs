use std::fs::File;
use std::io::Read;
use std::io;
use std::path::PathBuf;
use std::process;
use tar::Archive;


pub fn tar_check(package_file: PathBuf) {
	let package_file = package_file.to_str().expect("unexpected characters in package name");
	let mut install_file = String::new();
	for file in Archive::new(File::open(package_file).unwrap()).entries().unwrap() {
		let mut file = file.unwrap();
		let mode = file.header().mode().unwrap();
		if mode > 0o777 {
			eprintln!("ERROR! File {} / {:?} has mode {}, which is out of 0o777 permission zone", package_file, file.header().path(), mode);
			process::exit(-1);
		}
		if file.header().path().unwrap().to_str() == Some(".INSTALL") {
			file.read_to_string(&mut install_file).unwrap();
		}
	}
	loop {
		let install_note = if install_file.is_empty() { "" } else { "[I] = show install file, " };
		eprint!("\nPackage {} has no SUID files.\n\
			[E] = list executable files, [L] = list all files, {}[O] = ok, proceed. ",
			package_file, install_note
		);
		let mut string = String::new();
		io::stdin().read_line(&mut string).expect("RUA requires console to ask confirmation.");
		let string = string.trim().to_lowercase();
		if string == "l" {
			for file in Archive::new(File::open(package_file).unwrap()).entries().unwrap() {
				let mut file = file.unwrap();
				let path = file.header().path().unwrap();
				let path = path.to_str().unwrap();
				let is_normal = !path.ends_with("/") && !path.starts_with(".");
				if is_normal {
					eprintln!("{}", path);
				}
			}
		} else if string == "e" {
			for file in Archive::new(File::open(package_file).unwrap()).entries().unwrap() {
				let mut file = file.unwrap();
				let mode = file.header().mode().unwrap();
				let path = file.header().path().unwrap();
				let path = path.to_str().unwrap();
				let is_normal = !path.ends_with("/") && !path.starts_with(".");
				if is_normal && (mode & 0o111 > 0) {
					eprintln!("{}", path);
				}
			}
		} else if string == "i" && !install_file.is_empty() {
			eprintln!("{}", &install_file);
		} else if string == "o" {
			break;
		}
	}
}
