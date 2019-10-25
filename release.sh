#!/bin/bash -euET
{
# script to release RUA, probably has no use to anybody else except for reference.

set -o pipefail

rustup update
cargo upgrade
cargo update
cargo fmt --all --
if ! test -z "$(git status --porcelain)"; then
  >&2 printf '%s\n' "error: uncommitted changes"
  exit 1
fi

shellcheck -e SC1090 res/wrap.sh
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo publish

tag=$(cat Cargo.toml | grep -m1 version | sed 's/.*"\(.*\)"/\1/')
export tag
git tag -m "release" "$tag"

git push hub
git push lab

# prepare and test AUR package
if test -e .vasya-personal/aur_prepare.sh; then
  ./.vasya-personal/aur_prepare.sh
fi

exit
}
