## RUA

Work In Progress! No official announcement has been made yet.

RUA is a build tool for ArchLinux, AUR.

* It will never allow you install SUID files
* It shows you file list preview before installing.
* It fetches all dependencies (recursively) before building
* * minimizing user interaction (verify all PKGBUILD-s once, build everything later)
* * saving your time by exiting early in case of missing packages
* It uses a namespace [jail](https://github.com/projectatomic/bubblewrap) to build packages:
* * No internet access is given to PKGBUILD when building packages
* * PKGBUILD script is run under seccomp rules
* * Filesystem is read-only except the build dir
* * Home directory (~) is not visible to PKGBUILD, except the build dir
* * etc


## Install
* Install dependencies: `pacman -S --needed --asdeps bubblewrap`
* Build with Rust/cargo: `cargo install rua`

TODO: make AUR package :-)


## Limitations

* The tool does not allow you searching for packages, it only installs once you know the exact name. Author of this tool uses the [web page](https://aur.archlinux.org/packages/) to find packages.
* The tool does not show you outdated packages (those which have updates in AUR). Use web site email notifications for now. Hopefully I'll implement it over time. Pull requests are welcomed.
* Optional dependencies (optdepends) are not installed. They are skipped. Check them out manually when you review PKGBUILD.


## Safety
RUA only adds built-time safety. Even though you can be sure there are no SUID files and ugly stuff like that, the resulting package (run-time) is as safe as it was in the first place. Do not install AUR packages you don't trust.


## Other

The name can be read as "RUst Aur jail". Project is shared under GPLv3+.
