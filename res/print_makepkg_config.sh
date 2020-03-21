#!/usr/bin/bash

# config.sh is a publicly exposed utility library:
# https://git.archlinux.org/pacman.git/commit/scripts/libmakepkg/util/config.sh.in?id=a00615bfdad628299352b94e0f44d211a758fd17
source "${LIBRARY:-/usr/share/makepkg}/util/config.sh";

load_makepkg_config;

# config entries which can be overriden with environment variables; taken from util/config.sh
for var in PKGDEST SRCDEST SRCPKGDEST LOGDEST BUILDDIR PKGEXT SRCEXT GPGKEY PACKAGER CARCH; do
	[[ -v $var ]] && printf "%s=%s\0" "$var" "${!var}";
done
