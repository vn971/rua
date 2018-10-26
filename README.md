## RUA

Work In Progress! No official announcement has been made yet.

RUA is a build tool for ArchLinux, AUR. It's unique features are:

* It uses a namespace [jail](https://github.com/projectatomic/bubblewrap) to build packages:
* * No internet access is given to PKGBUILD when building packages
* * PKGBUILD script is run under seccomp rules
* * Filesystem is read-only except the build dir
* * Home directory (~) is not visible to PKGBUILD, except the build dir
* * etc
* It fetches dependencies recursively before building
* * saving your time by exiting early in case of missing packages
* * minimizing user interaction (verify all PKGBUILD-s once, build everything later)
* TODO: it shows you file preview of built packages, before proposing you to install it.
* TODO: it does NOT allow you installing packages that have SUID files

## Install
* Install dependencies: `pacman -S --needed --asdeps bubblewrap`
* Build with Rust/cargo: `cargo install rua`

TODO: make AUR package :-)

## Limitations

* Build-time dependencies are not distinguished from run-time dependencies (all are requested to install).
* Optional dependencies (optdepends) are not installed. They are skipped. Check them out manually while you review PKGBUILD, for now.

## Safety
It's **not safe** to install arbitrary packages from AUR, even inside this jail:

* RUA only adds some protection and convenience for _build time_. The safety properties of the resulting package are not changed in any way.
* Packages can install to dangerous locations like /etc/cron.d, if you're not paying attention to package file list preview.
* Packages can break out of bubblewrap via kernel vulnerabilities. It's _a bit_ harder from under normal user, with seccomp rules and the like -- but still possible.
* It's all not really about the build time. Even though this project tries to build as secure as possible, the most dangerous step is probably still running the built packages. Anyway, you should know what you're doing.

## Other

The name can be read as "RUst Aur jail". Project is shared under GPLv3+.
