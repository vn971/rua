## RUA  [![Rust](https://github.com/vn971/rua/workflows/Rust/badge.svg)](https://github.com/vn971/rua/actions)  [![ShellCheck](https://github.com/vn971/rua/workflows/ShellCheck/badge.svg)](https://github.com/vn971/rua/actions)  [![crates.io](https://img.shields.io/crates/v/rua.svg)](https://crates.io/crates/rua)

RUA is a build tool for ArchLinux, AUR. Its features:

- Allows local patch application
- Provides detailed information:
  * show upstream changes upon package upgrade
  * see code problems in PKGBUILD via `shellcheck`, taking care of special variables
  * warn if SUID files are present in an already built package, and show them
  * show file list, executable list and INSTALL script in already built packages
- Minimize user distractions:
  * verify all build scripts once, build without interruptions
  * group built packages for batch review
- Uses a security namespace jail:
  * supports `--offline` builds
  * builds in isolated filesystem, see [safety](#Safety) section below
  * uses `seccomp` to limit available syscalls (e.g. the build cannot call `ptrace`)
  * the build cannot execute `sudo` (filesystem is mounted with `nosuid`)
- Written in Rust


## Use

`rua search wesnoth`

`rua info freecad`

`rua install pinta`  # install or upgrade a package

`rua upgrade`  # upgrade AUR packages. By default, all packages are upgraded, unless specific packages are given as arguments. You can selectively ignore packages by using `--ignore` or adding them to `IgnorePkg` in `pacman.conf` (same as with non-AUR packages and `pacman`).

`rua shellcheck path/to/my/PKGBUILD`  # run `shellcheck` on a PKGBUILD, discovering potential problems with the build instruction. Takes care of PKGBUILD-specific variables.

`rua tarcheck xcalib.pkg.tar`  # if you already have a *.pkg.tar package built, run RUA checks on it (SUID, executable list, INSTALL script review etc).

`rua builddir --offline /path/to/pkgbuild/directory`  # build a directory.

`rua --help; rua subcommand --help`  # shows CLI help


## Install dependencies
```sh
sudo pacman -S --needed --asdeps git base-devel bubblewrap-suid libseccomp xz shellcheck cargo
```


## Install (the AUR way)
```sh
sudo pacman -S --needed base-devel git
git clone https://aur.archlinux.org/rua.git
cd rua
makepkg -si
```
In the web interface, package is [rua](https://aur.archlinux.org/packages/rua/).


## Install (the Rust way)
```sh
RUSTUP_TOOLCHAIN=stable cargo install --force rua
```

This will not include bash/zsh/fish completions, but everything else should work.


## How it works / directories
| directory | meaning |
| ------------- | ------------- |
| `~/.config/rua/pkg/` | Step 1, directory where AUR packages are cloned into. You review and make local modifications here |
| `~/.cache/rua/build/` | Step 2, reviewed packages are copied here, and then built |
| `~/.local/share/rua/checked_tars/` | Step 3, directory where built and tarcheck-ed packages are stored (*.pkg.tar.xz) |
| `~/.config/rua/wrap_args.d/` | entrypoint for basic configuration of the security wrapper script |
| `~/.config/rua/.system/` | internal files |
| `$GNUPGHOME/pubring.kbx` <br/> `$GNUPGHOME/pubring.gpg` | read-only access to these two files is granted when building, to allow signature verification |
| All other files | All other files in `~` are not accessed by RUA and inaccessible by built packages (see Safety section below) |

Note that directories above follow the XDG specification,
so `XDG_CONFIG_HOME` environment variable would override `~/.config`,
`XDG_CACHE_HOME` would override `~/.cache` and
`XDG_DATA_HOME` would override `~/.local/share`.

## How it works / reviewing
Knowing the underlying machinery is not required to work with RUA,
but if you're curious anyway, this section is for you.

All AUR packages are stored in designated `git` repositories,
with `upstream/master` pointing to remote AUR head and
local `master` meaning your reviewed and accepted state.
Local branch does not track the remote one.

RUA works by fetching remote updates when needed,
presenting remote changes to you and merging them if you accept them.
Merging and basic diff view are built-in commands in RUA, and you can
drop to shell and do more from git CLI if you want.


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

* This tool focuses on AUR packages only, you cannot `-Suy` your system with it. Please use pacman for that.
* Optional dependencies (optdepends) are not installed. They are skipped. Please check them out manually when you review PKGBUILD.
* The tool does not handle versions. It will always install the latest version possible, and it will always assume that latest version is enough.
* Development packages such as "-git" packages are only rebuilt when running `rua upgrade --devel`. No version checks are done to avoid unnecessary rebuilds. Merge requests welcomed.
* Unless you explicitly enable it, builds do not share user home (~). This may result in maven/npm/cargo/whatever dependencies re-downloading with each build. See [safety](#safety) section below on how to whitelist certain directories.
* Environment variables "PKGDEST" and "BUILDDIR" of makepkg.conf are not supported. Packages are built in isolation from each other, artifacts are stored in standard locations of this tool.
* Due of safety restrictions, [X11 access might not work](./docs/x11access.md) during build.
* Also due to safety restrictions, [ccache usage will fail](./docs/ccache.md) during build.
* Due to a [bug in fakeroot](https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=909727), creation of root-owned packages inside PKGBUILD-s `package()` does not work. This happens when archives are extracted in `package()` function. Doing it in `prepare()` or giving a key like `tar --no-same-owner` is the work-around.


## Safety
Do not install AUR packages you don't trust. RUA only adds build-time isolation and install-time control/review.

When building packages, RUA uses the following filesystem isolation:

* Build directory is mounted read-write.
* Files `"$GNUPGHOME"/pubring.kbx` and `"$GNUPGHOME"/pubring.gpg` are mounted read-only (if exists). This allows signature verification to work.
* The rest of `~` is not visible to the build process, mounted under tmpfs.
* `/tmp` and `/dev` and `/proc` are re-mounted with empty tmpfs, devtmpfs and procfs accordingly.
* The rest of `/` is mounted read-only.
* You can whitelist/add your mount points by configuring "wrap_args". See example in ~/.config/rua/.system/wrap_args.sh.example.

Additionally, all builds are run in a namespace jail, with `seccomp` enabled
and `user`, `ipc`, `pid`, `uts`, `cgroup` being unshared by default.
If asked from CLI, builds can be run in offline mode.


## Other

The RUA name is an inversion of "AUR".

This work was made possible by the excellent libraries of
[raur](https://gitlab.com/davidbittner/raur),
[srcinfo](https://github.com/Morganamilo/srcinfo.rs)
and many others.

Project is shared under GPLv3+.
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project (rua) by you,
shall be licensed as GPLv3+, without any additional terms or conditions.

For authors, see [Cargo.toml](Cargo.toml) and git history.
