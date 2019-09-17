#!/bin/bash -euET
{
# script to release RUA, probably has no use to anybody else except for reference.

set -o pipefail

rustup update
cargo upgrade
cargo update
if ! test -z "$(git status --porcelain)"; then
  >&2 printf '%s\n' "error: uncommitted changes"
  exit 1
fi

shellcheck -e SC1090 res/wrap.sh
cargo fmt --all --
cargo clippy --all-targets --all-features -- -D warnings
cargo test
if ! test -z "$(git status --porcelain)"; then
  >&2 printf '%s\n' "error: uncommitted changes"
  exit 1
fi

cargo publish

tag=$(cat Cargo.toml | grep -m1 version | sed 's/.*"\(.*\)"/\1/')
git tag -m "release" "$tag"

git push hub
git push lab

# prepare and test AUR package
if test -e .vasya-personal/aur_prepare.sh; then
  source .vasya-personal/aur_prepare.sh
fi

exit
}
