pub trait AlpmWrapper {
	fn is_package_installed(&self, name: &str) -> bool;
	fn is_installed(&self, package: &str) -> bool;
	fn list_foreign_packages(&self) -> Vec<String>;
	fn is_package_older_than(&self, package: &str, version: &str) -> bool;
}
