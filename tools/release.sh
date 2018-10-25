#!/bin/bash -euET
{
set -o pipefail

err_exit() {
	>&2 printf '%s\n' "$*"
	exit 1
}

if ! test -z "$(git status --porcelain)"; then # no uncommited local changes
  err_exit "error: uncommitted changes"
fi

cargo build --release
cargo publish

version=$(cat Cargo.toml | head | grep version | sed 's/.*"\(.*\)"/\1/')
git tag -m "release" "$version"

exit
}
