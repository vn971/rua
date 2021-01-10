#[cfg(test)]
use mockall::{automock, predicate::*};
use std::path::PathBuf;

#[cfg_attr(test, automock)]
pub trait IFolderDeleter {
	fn delete_folder(&self, path: &PathBuf) -> rm_rf::Result<()>;
}

pub struct FolderDeleter {}

impl FolderDeleter {
	pub fn new() -> FolderDeleter {
		FolderDeleter {}
	}
}

impl IFolderDeleter for FolderDeleter {
	fn delete_folder(&self, path: &PathBuf) -> rm_rf::Result<()> {
		rm_rf::remove(path.as_path())
	}
}
