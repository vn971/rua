#!/usr/bin/env bash
set -euET -o pipefail
{

# This happens after user confirmation,
# on a read-only filesystem, with seccomp rules and no internet access
source PKGBUILD

echo "${depends[@]}"
exit
}
