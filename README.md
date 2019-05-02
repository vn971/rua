## RUA  [![Build Status](https://travis-ci.org/vn971/rua.svg?branch=master)](https://travis-ci.org/vn971/rua)  [![crates.io](https://img.shields.io/crates/v/rua.svg)](https://crates.io/crates/rua)

RUA is a build tool for ArchLinux, AUR. Its features:

- Uses a namespace [jail](https://github.com/projectatomic/bubblewrap) to build packages:
  * supports "offline" builds (network namespace)
  * builds in isolated filesystem, see [safety](#Safety) section below
  * PKGBUILD script is run under seccomp rules (e.g. the build cannot call `ptrace`)
  * filesystem is mounted with "nosuid" (e.g. the build cannot call `sudo`)
- Show the user what they are about to install:
  * warn if SUID files are present, and show them
  * show INSTALL script (if present), executable and file list preview
- Minimize user interaction:
  * verify all PKGBUILD-s once, build without interruptions
  * group built dependencies for batch review/install
- Written in Rust

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
We'll consider the "install" command. RUA will:

1. Fetch the AUR package and all recursive dependencies.
1. Prepare a summary of all pacman and AUR packages that will need installing.
  Show the summary to the user, confirm proceeding.
1. Iterate over all AUR dependencies and ask to review the repo-s (PKGBUILDs, etc).
1. Propose installing all pacman dependencies in one batch.
  (No need to do it for each AUR package individually, save user-s time).
1. Build all AUR packages of maximum dependency "depth".
1. Let the user review built artifacts (in batch).
1. Install them. If any more packages are left, go two steps up.

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

IRC: #rua @freenode.net  (no promises are made for availability)

Project is shared under GPLv3+.
