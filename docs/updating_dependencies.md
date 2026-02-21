# Updating dependencies

This document describes how to update the project's Rust dependencies (crates) in `Cargo.toml` and `Cargo.lock`.

# Prerequisites
You'll need the `cargo-edit` package to add the cargo upgrade/update commands, and `cargo-audit` to add the audit commands
```sh
pacman -S cargo-edit cargo-audit
```

# Updating all dependencies

Upgrade version requirements in `Cargo.toml` to the latest compatible (semver) versions, then refresh the lockfile:

```sh
cargo upgrade
cargo update
```

`cargo upgrade` rewrites dependency version bounds in `Cargo.toml`. `cargo update` resolves them and updates `Cargo.lock`.

To preview changes without modifying files:

```sh
cargo upgrade --dry-run
```

The output may show some crates as `incompatible`: the latest version bumps the major (or minor, for 0.x) version and may break the API. By default `cargo upgrade` only bumps to the latest *compatible* version for those; `new req` stays within semver.

## Finding missing upgrades
`cargo upgrade` still misses some dependencies. You can see which ones were not updated by running

```sh
cargo update --verbose
```
and then manually updating the cargo.toml with those versions


# After updating

Build, test, and run linters (see [local_development.md](local_development.md)). CI runs `cargo audit` on dependency changes; run it locally to check for known vulnerabilities:

```sh
cargo audit
```

Commit both `Cargo.toml` and `Cargo.lock` when updating dependencies.


