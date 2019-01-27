use std::env;
use std::io;
use std::process::Command;

pub fn console_get_line() -> String {
    let mut string = String::new();
    io::stdin()
        .read_line(&mut string)
        .expect("RUA requires console to get user input");
    string.trim().to_lowercase()
}

/// For example: SHELL, PAGER, EDITOR.
pub fn run_env_command(env_variable_name: &str, alternative_executable: &str, arguments: &[&str]) {
    let command = env::var(env_variable_name)
        .ok()
        .map(|s| s.trim().to_owned());
    let command: Vec<_> = command
        .iter()
        .flat_map(|e| e.split(' '))
        .map(|e| e.trim())
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
    let command = command.status();
    if let Some(err) = command.err() {
        eprintln!("Failed to run command, error: {}", err);
    }
}
