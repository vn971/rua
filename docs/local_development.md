# Local development
When creating new features for rua, this document describes how to set up your computer, build and test the changes

# Prerequisites

You'll need rust installed on your system to begin development. The [Arch wiki](https://wiki.archlinux.org/title/Rust) describes the various ways to accomplish this (but installing the rustup package is suggested). Use the stable toolchain `rustup default stable`

Next, install the system dependencies listed in project's [README.md](/README.md). You'll also need to add clippy `rustup component add clippy` in order to run the linters.
 With that, assuming you have rust, a rust toolchain, cargo, you should be ready to build. You can verify your system is setup correctly by following the [Compiling](#compiling) section 

# Compiling

Debug build (faster compile, slower binary):

```sh
cargo build
```

Release build (optimized binary):

```sh
cargo build --release
```

Binaries are written to `target/debug/rua` and `target/release/rua` respectively.

### Running the locally built binary

Using cargo:

```sh
cargo run -- <args>        # debug build
cargo run --release -- <args>   # release build
```

Or run the binary directly:

```sh
./target/debug/rua --help
./target/release/rua install pinta
```

# Testing
Unit tests can be run with cargo

```sh
cargo test
```

To run a single test, pass a substring of the test name (all matching tests run):

```sh
cargo test test_name_substring
```

After a failure, re-run only that test by using its name (or a unique substring) from the failure output.

## Linters
The following linters are applied during CI build, and can be run locally to format and fix the code before committing
```sh
cargo fmt --all
cargo ci-clippy
shellcheck -e SC1090 res/wrapper/*.sh
```
