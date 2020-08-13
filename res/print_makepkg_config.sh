#!/usr/bin/bash

# script used by RUA to read makepkg configuration
# makepkg is heavily tied to bash, so we need to source some makepkg files
# in order to access the configuration

# config.sh is safe to use because it is a publicly exposed utility library:
# https://git.archlinux.org/pacman.git/commit/scripts/libmakepkg/util/config.sh.in?id=a00615bfdad628299352b94e0f44d211a758fd17
source "${LIBRARY:-/usr/share/makepkg}/util/config.sh";

load_makepkg_config;

# config entries which can be overriden with environment variables; taken from util/config.sh
for var in PKGDEST SRCDEST SRCPKGDEST LOGDEST BUILDDIR PKGEXT SRCEXT GPGKEY PACKAGER CARCH; do
	[[ -v $var ]] && printf "%s=%s\0" "$var" "${!var}";
done
