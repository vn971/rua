#!/bin/bash
{

depends=()
makedepends=()

# This happens after user review of this file,
# in a restricted shell, on a read-only filesystem with seccomp rules.
source PKGBUILD

alldeps=( "${depends[@]}" "${makedepends[@]}" )
echo "${alldeps[@]}"

exit
}
