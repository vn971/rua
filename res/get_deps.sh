#!/usr/bin/env bash
set -euET -o pipefail
{

depends=()
makedepends=()

# This happens after user confirmation,
# on a read-only filesystem, with seccomp rules and no internet access
source PKGBUILD

alldeps=( "${depends[@]}" "${makedepends[@]}" )
echo "${alldeps[@]}"

exit
}
