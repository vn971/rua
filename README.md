## RUA

Work In Progress! No official announcement has been made yet.

RUA is a build tool for ArchLinux, AUR. It's unique properties are:

* It's written in Rust
* It uses a namespace jail to build packages ("bubblewrap"):
* * No internet access is given to PKGBUILD when building packages
* * PKGBUILD script is run under seccomp rules
* * Filesystem is read-only except the build dir
* * etc
* It fetches dependencies recursively before building
* * saving your time by exiting early in case of missing packages
* * minimizing user interaction (verify all PKGBUILD-s once, build everything later)


## Install
* Install dependencies: `pacman -S --needed --asdeps bubblewrap`
* Build with Rust/cargo: `cargo install rua`

TODO: make AUR package :-)

## Safety
It's still **not safe** to install arbitrary packages from AUR, even inside this jail:

* Packages can install to dangerous locations like /etc/cron.d, if you're not paying attention to package file list preview.
* Packages can break out of bubblewrap via kernel vulnerabilities. It's _a bit_ harder from under normal user, with seccomp rules and the like -- but still possible.
* It's all not really about the build time. Even though this project tries to build as secure as possible, the most dangerous step is probably still running the built packages. Anyway, you should know what you're doing.

