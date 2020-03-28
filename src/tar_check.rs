use crate::terminal_util;
extern crate libflate;
extern crate ruzstd;
use colored::*;
use libflate::gzip::Decoder as GzipDecoder;
use log::debug;
use ruzstd::StreamingDecoder as ZstdDecoder;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use tar::Archive as TarArchive;
use xz2::read::XzDecoder;

pub fn tar_check_unwrap(tar: &Path) {
	tar_check(tar).unwrap_or_else(|e| {
		eprintln!("{}", e);
		std::process::exit(1)
	})
}

pub fn tar_check(path: &Path) -> Result<(), String> {
	debug!("Checking file {}", path.display());

	let mut archive =
		File::open(&path).unwrap_or_else(|e| panic!("cannot open file {}: {}", path.display(), e));

	let mut lzma_decoder;
	let mut gzip_decoder;
	let mut zstd_decoder;

	let extension = path.extension().unwrap_or_else(|| OsStr::new(""));
	let decoder: &mut dyn Read = match extension.as_bytes() {
		b"tar" => &mut archive,

		b"xz" | b"lzma" => {
			lzma_decoder = XzDecoder::new(&archive);
			&mut lzma_decoder
		}

		b"gz" | b"gzip" => {
			gzip_decoder = GzipDecoder::new(archive)
				.map_err(|e| format!("Corrupted gzip archive {}?. Error: {}", path.display(), e))?;
			&mut gzip_decoder
		}

		b"zst" | b"zstd" => {
			zstd_decoder = ZstdDecoder::new(&mut archive)
				.map_err(|e| format!("Corrupted zstd archive {}?. Error: {}", path.display(), e))?;
			&mut zstd_decoder
		}

		_ => {
			return Err(format!(
				"Archive {} cannot be analyzed. \
				Only .tar or .tar.xz or .tar.gz or .tar.zst files are supported",
				path.display()
			))
		}
	};

	tar_check_archive(TarArchive::new(decoder), path);

	Ok(())
}

fn tar_check_archive(mut archive: TarArchive<&mut dyn Read>, path: &Path) {
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
