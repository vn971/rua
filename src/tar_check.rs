use crate::terminal_util;
extern crate libflate;
extern crate ruzstd;
use colored::*;
use indexmap::IndexSet;
use libflate::gzip::Decoder;
use log::debug;
use ruzstd::decoding::StreamingDecoder;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use tar::*;
use xz2::read::XzDecoder;

pub fn tar_check_unwrap(tar_file: &Path, file_name: &str) {
	let result = tar_check(tar_file, file_name);
	result.unwrap_or_else(|err| {
		eprintln!("{}", err);
		std::process::exit(1)
	})
}

pub fn tar_check(tar_file: &Path, tar_str: &str) -> Result<(), String> {
	let archive = File::open(tar_file).unwrap_or_else(|_| panic!("cannot open file {}", tar_str));
	debug!("Checking file {}", tar_str);
	if tar_str.ends_with(".tar") {
		tar_check_archive(Archive::new(archive), tar_str);
		Ok(())
	} else if tar_str.ends_with(".tar.xz") || tar_str.ends_with(".tar.lzma") {
		tar_check_archive(Archive::new(XzDecoder::new(archive)), tar_str);
		Ok(())
	} else if tar_str.ends_with(".tar.gz") || tar_str.ends_with(".tar.gzip") {
		match Decoder::new(archive) {
			Ok(decoded) => {
				tar_check_archive(Archive::new(decoded), tar_str);
				Ok(())
			},
			Err(err) => {
				Err(format!("File {:?} seems to be corrupted, could not decode the gzip contents. Underlying libflate error: {}", tar_file, err))
			},
		}
	} else if tar_str.ends_with(".tar.zst") || tar_str.ends_with(".tar.zstd") {
		let mut archive = archive;
		match StreamingDecoder::new(&mut archive) {
			Ok(decoder) => {
				tar_check_archive(Archive::new(decoder), tar_str);
				Ok(())
			},
			Err(err) => {
				Err(format!("File {:?} seems to be corrupted, could not decode the zstd contents. Underlying ruzstd error: {}", tar_file, err))
			},
		}
	} else {
		Err(format!(
			"Archive {:?} cannot be analyzed. Only .tar or .tar.xz or .tar.gz or .tar.zst files are supported",
			tar_file
		))
	}
}

fn tar_check_archive<R: Read>(mut archive: Archive<R>, path_str: &str) {
	let mut install_file = String::new();
	let mut all_files = Vec::new();
	let mut executable_files = Vec::new();
	let mut suid_files = Vec::new();
	let archive_files = archive
		.entries()
		.unwrap_or_else(|e| panic!("cannot open archive {}, {}", path_str, e));
	for file in archive_files {
		let mut file =
			file.unwrap_or_else(|e| panic!("cannot access tar file in {}, {}", path_str, e));
		let path = {
			let path = file.header().path().unwrap_or_else(|e| {
				panic!(
					"Failed to extract tar file metadata for file in {}, {}",
					path_str, e,
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
			file.read_to_string(&mut install_file).unwrap_or_else(|_| {
				panic!("Failed to read INSTALL script from tar file {}", path_str)
			});
		}
	}

	let has_install = !install_file.is_empty();
	let display_name = Path::new(path_str)
		.file_name()
		.and_then(|p| p.to_str())
		.unwrap_or(path_str);
	loop {
		if suid_files.is_empty() {
			eprintln!("Package {} has no SUID files.", display_name);
		}
		eprint!("{}=list executable files, ", "[E]".bold());
		eprint!("{}=list all files, ", "[L]".bold());
		eprint!("{}=list files not existing on filesystem, ", "[F]".bold());

		eprint!(
			"{}{}, ",
			"[T]".bold().cyan(),
			"=run shell to inspect".cyan()
		);

		if has_install {
			eprint!(
				"{}=show {}, ",
				"[I]".bold(),
				"install file".bold().bright_red()
			);
		};

		if !suid_files.is_empty() {
			eprint!(
				"{}=list {}, ",
				"[S]".bold(),
				"SUID files".bold().bright_red()
			);
		};
		eprint!("{}=ok, proceed. ", "[O]".bold());
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
		} else if &string == "f" {
			for path in &all_files {
				if !Path::exists(Path::new(&format!("/{}", &path))) {
					eprintln!("{}", path);
				}
			}
		} else if &string == "l" {
			for path in &all_files {
				eprintln!("{}", path);
			}
		} else if &string == "i" && has_install {
			eprintln!("{}", &install_file);
		} else if &string == "t" {
			let dir = PathBuf::from(path_str);
			let dir = dir
				.parent()
				.filter(|p| !p.as_os_str().is_empty())
				.unwrap_or_else(|| Path::new("."));
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

pub fn common_suffix_length(pkg_names: &[&str], archive_whitelist: &IndexSet<&str>) -> usize {
	let min_len = pkg_names.iter().map(|p| p.len()).min().unwrap_or(0);
	for suffix_length in 0..min_len {
		for pkg in pkg_names {
			let suffix_start = pkg.len() - suffix_length;
			let prefix = &pkg[..suffix_start];
			if archive_whitelist.contains(prefix) {
				return suffix_length;
			}
		}
	}
	min_len
}

#[cfg(test)]
mod tests {
	use crate::tar_check::*;
	use indexmap::IndexSet;

	fn test(files: &[&str], whitelist: &[&str], expected: usize) {
		let set: IndexSet<&str> = whitelist.iter().copied().collect();
		let result = common_suffix_length(files, &set);
		assert_eq!(result, expected)
	}

	#[test]
	fn test_all() {
		test(&["a-1.pkg.tar", "b-1.pkg.tar"], &["a"], 10);
		test(&["a-1.pkg.tar", "bbbb-1.pkg.tar"], &["a", "dinosaur"], 10);
		test(&["a-x-1.pkg.tar", "b-x-1.pkg.tar"], &["a-x"], 10);
		test(&["a-x-1.pkg.tar", "b-x-1.pkg.tar"], &["a"], 12);
	}
}
