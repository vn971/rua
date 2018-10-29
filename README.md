## RUA

RUA is a build tool for ArchLinux, AUR. Its features:

* Never allows you install SUID files
* Shows you file list preview before installing
* Minimizes user interaction:
* * verify all PKGBUILD-s once, build everything later
* * group dependencies to require fewer interaction times
* * (exit early in case of missing dependencies)
* Uses a namespace [jail](https://github.com/projectatomic/bubblewrap) to build packages:
* * filesystem is read-only except the build dir
* * PKGBUILD script is run under seccomp rules
* * optionally, no internet access is given to PKGBUILD when building packages
* * home directory (~) is not visible to PKGBUILD, except the build dir
* * etc
* Written in Rust

Planned features include AUR upstream git diff and local patch application.


## Install
* Install dependencies: `pacman -S --needed --asdeps bubblewrap`
* Build with Rust/cargo: `cargo install rua`

TODO: make AUR package :-)


## Limitations

* The tool does not allow you searching for packages, it only installs once you know the exact name. Author of this tool uses the [web page](https://aur.archlinux.org/packages/) to find packages.
* No smart caching is implemented yet. To avoid outdated builds, RUA wipes all caches in case of possible conflict.
* The tool does not show you outdated packages (those which have updates in AUR). Use web site email notifications for now. Hopefully I'll implement it over time. Pull requests are welcomed.
* Optional dependencies (optdepends) are not installed. They are skipped. Check them out manually when you review PKGBUILD.


## Safety
RUA only adds built-time safety. Even though you can be sure there are no SUID files and ugly stuff like that, the resulting package (run-time) is as safe as it was in the first place. Do not install AUR packages you don't trust.


## Other

The RUA name can be read as "RUst Aur jail", also an inversion of "AUR" word.

Project is shared under GPLv3+.
