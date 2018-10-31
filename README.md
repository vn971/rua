## RUA

RUA is a build tool for ArchLinux, AUR. Its features:

* Show the user what they are about to install:
* * show INSTALL script (if present)
* * warn if SUID files are present, and show them
* * show file list preview
* * show executable list preview
* Minimize user interaction:
* * verify all PKGBUILD-s once, build everything later
* * group dependencies for batch review/install
* Uses a namespace [jail](https://github.com/projectatomic/bubblewrap) to build packages:
* * supports "offline" builds (no internet access given to PKGBUILD when building)
* * filesystem is read-only except the build dir
* * PKGBUILD script is run under seccomp rules
* * home directory (~) is not visible to PKGBUILD, except the build dir
* * etc
* Written in Rust

Planned features include AUR upstream git diff and local patch application.


# Use

`rua install firefox-ublock-origin`  # install AUR package (with user confirmation)

`rua install --offline firefox-ublock-origin`  # same as above, but PKGBUILD is run without internet access.

`rua tarcheck my_built_package.pkg.tar`  # if you already have a *.tar package built, run RUA checks on it (SUID, executable list, INSTALL script review etc).

`rua jailbuild --offline /path/to/pkgbuild/directory`  # build a directory. Don't fetch any dependencies. Assumes a clean directory.


## Install (the AUR way)
Install [RUA](https://aur.archlinux.org/packages/rua/) package using the default manual build process (https://wiki.archlinux.org/index.php/Arch_User_Repository#Prerequisites). Or using another AUR helper, or an earlier version of RUA.


## Install (the Rust way)
* Install dependencies: `pacman -S --needed --asdeps bubblewrap rust`
* Build:
* * `cargo install`, to build in cloned repo
* * `cargo install rua`, to build from crates.io

There won't be bash/zsh/fish completions this way, but everything else should work.


## How it works
We'll consider the "install" command as it's the most advanced one. RUA will:

1. Fetch the AUR package (via git)
2. Let the user review PKGBUILD and the repo. Only go to next steps after user approval.
3. Check AUR dependencies, repeat the process for them
4. Ask the user to install all aggregated non-aur packages.
5. Build all AUR packages of maximum dependency "depth"
6. After all are built, let the user review them all
7. If review passes, let the user install these packages
8. The lowest (dependency-wise) packages are now installed. Go to 5.
9. Exit when all packages are installed.

## Limitations

* The tool does not allow you searching for packages, it only installs once you know the exact name. Author of this tool uses the [web page](https://aur.archlinux.org/packages/) to find packages.
* Smart caching is not implemented yet. To avoid outdated builds, RUA wipes all caches in case of possible conflict.
* The tool does not show you outdated packages (those which have updates in AUR). Use web site email notifications for now. Hopefully I'll implement it over time. Pull requests are welcomed.
* Optional dependencies (optdepends) are not installed. They are skipped. Check them out manually when you review PKGBUILD.
* Unless you explicitly enable it, builds do not share anything with normal user home (~). This may result in rust/maven/whatever packages being re-downloaded each build. Take a look at ~/.config/rua/wrap_args.sh to see which compromises you might want to take on that.


## Safety
RUA only adds built-time safety and review. Once the package passes your review, it's as safe (run-time) as it was in the first place. Do not install AUR packages you don't trust.


## Other

The RUA name can be read as "RUst Aur jail", also an inversion of "AUR".

Project is shared under GPLv3+.
