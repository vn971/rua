extern crate clap;

use clap::Shell;

include!("src/cli_args.rs");

fn main() {
    let directory = match std::env::var_os("COMPLETIONS_DIR") {
        None => return,
        Some(out_dir) => out_dir,
    };
    let mut app = build_cli();
    app.gen_completions(env!("CARGO_PKG_NAME"), Shell::Bash, &directory);
    app.gen_completions(env!("CARGO_PKG_NAME"), Shell::Fish, &directory);
    app.gen_completions(env!("CARGO_PKG_NAME"), Shell::Zsh, &directory);
}
