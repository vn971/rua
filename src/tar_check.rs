use crate::terminal_util;
extern crate libflate;
extern crate ruzstd;
use colored::*;
use libflate::gzip::Decoder;
use log::debug;
use ruzstd::StreamingDecoder;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use tar::*;
use xz2::read::XzDecoder;

pub fn tar_check_unwrap(tar: &Path) {
	tar_check(tar).unwrap_or_else(|e| {
		eprintln!("{}", e);
		std::process::exit(1)
	})
}

pub fn tar_check(tar: &Path) -> Result<(), String> {
	let archive =
		File::open(&tar).unwrap_or_else(|e| panic!("cannot open file {}: {}", tar.display(), e));

	debug!("Checking file {}", tar.display());

	let extension = tar.extension().unwrap_or_else(|| OsStr::new(""));
	match extension.as_bytes() {
		b"tar" => tar_check_archive(Archive::new(archive), tar),

		b"xz" | b"lzma" => tar_check_archive(Archive::new(XzDecoder::new(archive)), tar),

		b"gz" | b"gzip" => {
			let decoder = Decoder::new(archive)
				.map_err(|e| format!("Corrupted gzip archive {}?. Error: {}", tar.display(), e))?;
			tar_check_archive(Archive::new(decoder), tar);
		}

		b"zst" | b"zstd" => {
			let mut archive = archive;
			let decoder = StreamingDecoder::new(&mut archive)
				.map_err(|e| format!("Corrupted zstd archive {}?. Error: {}", tar.display(), e))?;
			tar_check_archive(Archive::new(decoder), tar);
		}

		_ => {
			return Err(format!(
				"Archive {} cannot be analyzed. \
				Only .tar or .tar.xz or .tar.gz or .tar.zst files are supported",
				tar.display()
			))
		}
	};

	Ok(())
}

fn tar_check_archive<R: Read>(mut archive: Archive<R>, path: &Path) {
	let dir = path.parent().unwrap_or_else(|| Path::new("."));
	let path = path.display();
	let mut install_file = String::new();
	let mut all_files = Vec::new();
	let mut executable_files = Vec::new();
	let mut suid_files = Vec::new();
	let archive_files = archive
		.entries()
		.unwrap_or_else(|e| panic!("cannot open archive {}, {}", path, e));
	for file in archive_files {
		let mut file = file.unwrap_or_else(|e| panic!("cannot access tar file in {}, {}", path, e));
		let path = {
			let path = file.header().path().unwrap_or_else(|e| {
				panic!(
					"Failed to extract tar file metadata for file in {}, {}",
					path, e,
				)
			});
			path.to_str()
				.unwrap_or_else(|| panic!("{}:{} failed to parse file name", file!(), line!()))
				.to_owned()
		};
		let mode = file.header().mode().unwrap_or_else(|_| {
			panic!(
				"{}:{} Failed to get file mode for file {}",
				file!(),
				line!(),
				path
			)
		});
		let is_normal = !path.ends_with('/') && !path.starts_with('.');
		if is_normal {
			all_files.push(path.clone());
		}
		if is_normal && (mode & 0o111 > 0) {
			executable_files.push(path.clone());
		}
		if mode > 0o777 {
			suid_files.push(path.clone());
		}
		if &path == ".INSTALL" {
			file.read_to_string(&mut install_file)
				.unwrap_or_else(|_| panic!("Failed to read INSTALL script from tar file {}", path));
		}
	}

	let has_install = !install_file.is_empty();
	loop {
		if suid_files.is_empty() {
			eprint!("Package {} has no SUID files.\n", path);
		}
		eprint!(
			"[E]=list executable files, [L]=list all files, \
			 [T]=run shell to inspect, "
		);
		if has_install {
			eprint!("{}", "[I]=show install file, ".bright_red().bold());
		};
		if !suid_files.is_empty() {
			eprint!("{}, ", "!!! [S]=list SUID files!!!".bright_red().bold());
		};
		eprint!("[O]=ok, proceed. ");
		let string = terminal_util::read_line_lowercase();
		eprintln!();
		if &string == "s" && !suid_files.is_empty() {
			for path in &suid_files {
				eprintln!("{}", path);
			}
		} else if &string == "e" {
			for path in &executable_files {
				eprintln!("{}", path);
			}
		} else if &string == "l" {
			for path in &all_files {
				eprintln!("{}", path);
			}
		} else if &string == "i" && has_install {
			eprintln!("{}", &install_file);
		} else if &string == "t" {
			eprintln!("Exit the shell with `logout` or Ctrl-D...");
			terminal_util::run_env_command(dir, "SHELL", "bash", &[]);
		} else if &string == "o" {
			break;
		} else if &string == "q" {
			eprintln!("Exiting...");
			std::process::exit(-1);
		}
	}
}
