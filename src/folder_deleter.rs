#[cfg(test)]
use mockall::{automock, predicate::*};
use std::fs;
use std::io;
use std::path::PathBuf;

#[cfg_attr(test, automock)]
pub trait IFolderDeleter {
	fn delete_folder(&self, path: &PathBuf) -> io::Result<()>;
}

pub struct FolderDeleter {}

impl FolderDeleter {
	pub fn new() -> FolderDeleter {
		FolderDeleter {}
	}
}

impl IFolderDeleter for FolderDeleter {
	fn delete_folder(&self, path: &PathBuf) -> io::Result<()> {
		fs::remove_dir_all(path)
	}
}
