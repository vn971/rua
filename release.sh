#!/bin/bash -euET
{
# script to release RUA, probably has no use to anybody except maintainers.

set -o pipefail

rustup update
cargo upgrade
cargo update
cargo fmt --all -- --check
if ! test -z "$(git status --porcelain)"; then
  >&2 printf '%s\n' "error: uncommitted changes"
  exit 1
fi

shellcheck -e SC1090 res/wrapper/security-wrapper.sh
cargo test
cargo clippy --all-targets --features= -- -D warnings
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
