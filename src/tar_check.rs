use crate::util;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tar::*;
use xz2::read::XzDecoder;

pub fn tar_check(tar_file: PathBuf) {
    let tar_str = tar_file
        .to_str()
        .unwrap_or_else(|| panic!("{}:{} Failed to parse tar file name", file!(), line!()));
    let archive = File::open(&tar_file).unwrap_or_else(|_| panic!("cannot open file {}", tar_str));
    if tar_str.ends_with(".tar.xz") {
        tar_check_archive(Archive::new(XzDecoder::new(archive)), tar_str);
    } else if tar_str.ends_with(".tar") {
        tar_check_archive(Archive::new(archive), tar_str);
    } else {
        panic!(
            "Unsupported file format for tar_check function: {}",
            tar_str
        )
    };
}

fn tar_check_archive<R: Read>(mut archive: Archive<R>, path_str: &str) {
    let mut install_file = String::new();
    let mut all_files = Vec::new();
    let mut executable_files = Vec::new();
    let mut suid_files = Vec::new();
    let archive_files = archive
        .entries()
        .unwrap_or_else(|_| panic!("cannot open archive {}", path_str));
    for file in archive_files {
        let mut file = file.unwrap_or_else(|_| panic!("cannot access tar file in {}", path_str));
        let path = {
            let path = file.header().path().unwrap_or_else(|_| {
                panic!(
                    "Failed to extract tar file metadata for file in {}",
                    path_str
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
    loop {
        if suid_files.is_empty() {
            eprint!("\nPackage {} has no SUID files.\n", path_str);
        } else {
            eprint!(
                "\n!!!WARNING!!! Package {} has SUID files.\n[S]=list SUID files, ",
                path_str
            )
        };
        if has_install {
            eprint!("[I]=show install file, ");
        };
        eprint!(
            "[E]=list executable files, [L]=list all files, \
             [T]=run shell to inspect, [O]=ok, proceed. "
        );
        let string = util::console_get_line();
        eprintln!();
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
