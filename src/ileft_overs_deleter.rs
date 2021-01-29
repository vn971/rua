use crate::rua_paths::RuaPaths;
#[cfg(test)]
use mockall::{automock, predicate::*};

#[cfg_attr(test, automock)]
pub trait ILeftOversDeleter {
	fn delete_folders(&self, targets: &[String], rua_paths: &RuaPaths) -> rm_rf::Result<()>;
}
