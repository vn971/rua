## RUA  [![Build Status](https://travis-ci.org/vn971/rua.svg?branch=master)](https://travis-ci.org/vn971/rua)  [![crates.io](https://img.shields.io/crates/v/rua.svg)](https://crates.io/crates/rua)

RUA is a build tool for ArchLinux, AUR. Its features:

- Allows local patch application
- Provides detailed information:
  * warn if SUID files are present in an already built package, and show them
  * show upstream diff before building, or full diff if the package is new
  * see code problems in PKGBUILD via `shellcheck` (taking care of special variables)
  * show INSTALL script (if present), executable and file list previews in already built package
- Minimize user distractions:
  * verify all packages once, build without interruptions
  * group built dependencies for batch review/install
- Uses a security namespace [jail](https://github.com/projectatomic/bubblewrap):
  * supports "offline" builds
  * builds in isolated filesystem, see [safety](#Safety) section below
  * uses `seccomp` to limit available syscalls (e.g. the build cannot call `ptrace`)
  * the build cannot execute `sudo` (filesystem is mounted with `nosuid`)
- Written in Rust


# Use

`rua install xcalib`  # installs or upgrades a package

`rua search wesnoth`

`rua info freecad`

`rua upgrade`  # (new) find outdated AUR packages and offer upgrade

`rua shellcheck path/to/my/PKGBUILD`  # run `shellcheck` on a PKGBUILD, discovering potential problems with the build instruction. Takes care of PKGBUILD-specific variables.

`rua tarcheck xcalib.pkg.tar`  # if you already have a *.pkg.tar package built, run RUA checks on it (SUID, executable list, INSTALL script review etc).

`rua builddir --offline /path/to/pkgbuild/directory`  # build a directory. Don't fetch any dependencies. Assumes a clean directory.

`rua --help && rua install --help`  # shows CLI help

Jail arguments can be overridden in ~/.config/rua/wrap_args.d/ .


## Install dependencies
```sh
sudo pacman -S --needed git base-devel bubblewrap-suid lz shellcheck cargo
```


## Install (the AUR way)
```sh
git clone https://aur.archlinux.org/rua.git
cd rua
makepkg -si
```
In the web interface, package is [rua](https://aur.archlinux.org/packages/rua/).


## Install (the Rust way)
```sh
cargo install rua
```

Does not include bash/zsh/fish completions, but everything else should work.
If you use development version `pacman-git`, use `cargo install --features git rua` instead.


## How it works / reviewing
When a new AUR package is fetched by RUA for the first time, it is stored in `~/.config/rua/pkg/pkg_name`.
This is done via git, with an empty branch as your currently accepted state.

If you review upstream changes and accept them, upstream is merged into your local branch.
RUA will only allow you building once you have upstream as your ancestor, making sure you merged it.

When you later install a new version of the package, RUA will fetch the new version and show you the diff since your last review.

## How it works / dependency grouping and installation
RUA will:

1. Fetch the AUR package and all recursive dependencies.
1. Prepare a summary of all pacman and AUR packages that will need installing.
  Show the summary to the user, confirm proceeding.
1. Iterate over all AUR dependencies and ask to review the repo-s. 
  Once we know that user really accepts all recursive changes, proceed.
1. Propose installing all pacman dependencies.
1. Build all AUR packages of maximum dependency "depth".
1. Let the user review built artifacts (in batch).
1. Install them. If any more packages are left, go two steps up.

If you have a dependency structure like this:
```
your_original_package
├── dependency_a
│   ├── a1
│   └── a2
└── dependency_b
    ├── b1
    └── b2
```
RUA will thus interrupt you 3 times, not 7 as if it would be plainly recursive. It also won't disrupt you if it knows recursion breaks down the line (with unsatisfiable dependencies).

## Limitations

* This tool focuses on AUR packages only, you cannot `-Suy` your system with it. Use pacman for that.
* Optional dependencies (optdepends) are not installed. They are skipped. Check them out manually when you review PKGBUILD.
* The tool does not handle versions. It will always install the latest version possible, and it will always assume that latest version is enough.
* Development packages such as "-git" packages are only rebuilt when running `rua upgrade --devel`. No version checks are done to avoid unnecessary rebuilds. Merge requests welcomed.
* Unless you explicitly enable it, builds do not share user home (~). This may result in maven/npm/cargo/whatever re-downloading dependencies with each build. If you want to override some of that, take a look at ~/.config/rua/wrap_args.d/ and the parent directory for examples.
* Shared directories such as makepkg.conf-s "PKGDEST", "BUILDDIR" are not supported. Packages are built in isolation from each other, artifacts are stored in standard locations of this tool. Merge requests for moving built artifacts elsewhere are welcomed.


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

This work was made possible by the excellent libraries of
[libalpm](https://github.com/jameslzhu/alpm),
[raur](https://gitlab.com/davidbittner/raur),
[srcinfo](https://github.com/Morganamilo/srcinfo.rs)
and many others.

IRC: #rua @freenode.net

Project is shared under GPLv3+. For authors, see [Cargo.toml](Cargo.toml) and git history.
