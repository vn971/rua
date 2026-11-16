#!/bin/bash
{
# script to release RUA, probably has no use to anybody except maintainers.

set -x -euETo pipefail

rustup update

# Require the latest rust release. Fits the role of an ArchLinux AUR helper.
msrv="$(rustc --version)"
msrv="${msrv#* }"
msrv="${msrv%% *}"
sed -i "s/^rust-version = .*/rust-version = \"$msrv\"/" Cargo.toml

cargo upgrade
cargo update
cargo fmt --all -- --check
shellcheck -e SC1090 res/wrapper/security-wrapper.sh
cargo test
cargo ci-clippy
if ! test -z "$(git status --porcelain)"; then
  >&2 printf '%s\n' "error: uncommitted changes"
  exit 1
fi

cargo publish

ver=$(cat Cargo.toml | grep -m1 version | sed 's/.*"\(.*\)"/\1/')
export ver
git tag -m "release" "v$ver"

git push
git push lab

# prepare and test AUR package
if test -e .vasya-personal/aur_prepare.sh; then
  .vasya-personal/aur_prepare.sh
fi

exit
}
