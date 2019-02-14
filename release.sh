#!/bin/bash -euET
{
set -o pipefail

cargo upgrade
cargo update

if ! test -z "$(git status --porcelain)"; then
  >&2 printf '%s\n' "error: uncommitted changes"
  exit 1
fi

cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo publish

tag=$(cat Cargo.toml | head | grep version | sed 's/.*"\(.*\)"/\1/')
git tag -m "release" "$tag"

# personal script to prepare and test AUR package
if test -e .vasya-personal/aur_prepare.sh; then
  source .vasya-personal/aur_prepare.sh
fi

git push
git push lab

exit
}
