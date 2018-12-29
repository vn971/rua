## RUA  [![Build Status](https://travis-ci.org/vn971/rua.svg?branch=master)](https://travis-ci.org/vn971/rua)  [![crates.io](https://img.shields.io/crates/v/rua.svg)](https://crates.io/crates/rua)

RUA is a build tool for ArchLinux, AUR. Its features:

* Show the user what they are about to install:
* * warn if SUID files are present, and show them
* * show INSTALL script (if present)
* * show file list preview
* * show executable list preview
* Minimize user interaction:
* * verify all PKGBUILD-s once, build everything later
* * group dependencies for batch review/install
* Uses a namespace [jail](https://github.com/projectatomic/bubblewrap) to build packages:
* * supports "offline" builds (no internet access given to PKGBUILD)
* * uses isolated filesystem, e.g. no access to home directory (`~`). See [safety](#Safety) section below
* * PKGBUILD script is run under seccomp rules
* * etc
* Written in Rust

Planned features include AUR upstream git diff and local patch application.


# Use

`rua install xcalib`  # install AUR package (with user confirmation)

`rua install --offline xcalib`  # same as above, but PKGBUILD is run without internet access. Sources are downloaded using .SRCINFO only.

`rua tarcheck xcalib.pkg.tar`  # if you already have a *.pkg.tar package built, run RUA checks on it (SUID, executable list, INSTALL script review etc).

`rua jailbuild --offline /path/to/pkgbuild/directory`  # build a directory. Don't fetch any dependencies. Assumes a clean directory.

`rua --help && rua install --help`  # shows CLI help

Jail arguments can be overridden in ~/.config/rua/wrap_args.d/ .


## Install dependencies
```sh
sudo pacman -S --needed git base-devel bubblewrap cargo
```


## Install (the AUR way)
```sh
git clone https://aur.archlinux.org/rua.git
cd rua
makepkg -si
```
In the web interface, package is [rua](https://aur.archlinux.org/packages/rua/).


## Install (the Rust way)
* `cargo install rua`

There won't be bash/zsh/fish completions this way, but everything else should work.


## How it works
We'll consider the "install" command as it's the most advanced one. RUA will:

1. Fetch the AUR package (via git)
1. Check .SRCINFO for other AUR dependencies, repeat the process for them
1. Once all dependencies are fetched, show user the summary of all pacman packages to install, AUR packages to build and install.
1. Ask user to install pacman dependencies (in batch for all recursive dependencies)
1. Let the user review all packages, including their PKGBUILDs.
1. Build all AUR packages of maximum dependency "depth"
1. Let the user review and install them (in batch)
1. The lowest (dependency-wise) packages are now installed. Go two steps up.
1. Exit when all packages are installed.

## Limitations

* Smart caching is not implemented yet. To avoid outdated builds, RUA wipes caches in case of possible conflict. This may change in the future.
* Optional dependencies (optdepends) are not installed. They are skipped. Check them out manually when you review PKGBUILD. This may change in the future.
* The tool does not show you outdated packages yet (those that have updates in AUR). Pull requests are welcomed.
* Unless you explicitly enable it, builds do not share user home (~). This may result in rust/maven/npm/whatever packages being re-downloaded each build. If you want to override some of that, take a look at ~/.config/rua/wrap_args.d/ and the parent directory for examples.


## Safety
RUA only adds build-time safety and install-time control. Once/if packages pass your review, they are as run-time safe as they were in the first place. Do not install AUR packages you don't trust.

When building packages, RUA uses the following filesystem isolation by default:

* Build directory is mounted read-write.
* ~/.gnupg directory is mounted read-only, excluding ~/.gnupg/private-keys-v1.d, which is blocked. This allows signature verification to work.
* The rest of `~` is not visible to the build process, mounted under tmpfs.
* The rest of `/` is mounted read-only.
* You can add your mount points by configuring "wrap_args".


## Other

The RUA name can be read as "RUst Aur jail", also an inversion of "AUR".

Project is shared under GPLv3+.
