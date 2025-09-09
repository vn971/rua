use std::env;
use std::io;
use std::path::Path;
use std::process::Command;

pub fn read_line_lowercase() -> String {
	let mut string = String::new();
	io::stdin()
		.read_line(&mut string)
		.expect("RUA requires console to get user input");
	string.trim().to_lowercase()
}

/// For example: SHELL, PAGER, EDITOR.
pub fn run_env_command(
	dir: &Path,
	env_variable_name: &str,
	alternative_executable: &str,
	arguments: &[&str],
) {
	let command = env::var(env_variable_name)
		.ok()
		.map(|s| s.trim().to_owned());
	let command: Vec<_> = command
		.iter()
		.flat_map(|e| e.split(' '))
		.map(str::trim)
		.filter(|e| !e.is_empty())
		.collect();
	let mut command = if let Some(first) = command.first() {
		let mut cmd = Command::new(first);
		cmd.args(&command[1..]);
		cmd
	} else {
		Command::new(alternative_executable)
	};
	command.args(arguments);
	command.current_dir(dir);
	let command = command.status();
	if let Some(err) = command.err() {
		eprintln!("Failed to run command, error: {}", err);
	}
}

/// From bash manual:
/// > Enclosing  characters in single quotes preserves the literal value of each character
/// > within the quotes. A single quote may not occur between single quotes,
/// > even when preceded by a backslash.
pub fn escape_bash_arg(str: &str) -> String {
	let result = str.replace('\'', "'\\''"); // end quoting, append the literal, start quoting
	format!("'{}'", result)
}

/// For terminals that support OSC 8 hyperlinks (like iTerm2, Windows Terminal, etc),
/// this will turn the given text into a clickable hyperlink to the given URL.
/// If the terminal does not support hyperlinks, the text is returned unchanged.
///
/// The text could be of any content, the url is constructed from `package_name`, not `text`.
pub fn try_hyperlink_package_name(mut text: String, package_name: &str) -> String {
	if !supports_hyperlinks::supports_hyperlinks() {
		return text;
	}
	let url = format_args!("https://aur.archlinux.org/packages/{}", package_name);
	text = format!("\x1B]8;;{}\x1B\\{}\x1B]8;;\x1B\\", url, text);
	text
}
