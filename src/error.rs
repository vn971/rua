#[derive(Debug, PartialEq, Eq)]
pub struct RuaError {
	// pub exit_code: u8,
	pub msg: String,
}

impl std::error::Error for RuaError {}

impl core::fmt::Display for RuaError {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "Error {}", self.msg)
	}
}
