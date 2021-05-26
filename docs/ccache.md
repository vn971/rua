## Context

This is about using your user's `ccache` from inside a RUA sandbox.
If you configure `makepkg` to use `ccache` as described in [ArchWiki](https://wiki.archlinux.org/title/Ccache#Enable_ccache_for_makepkg),
this will fail with error messages like:

```
ccache: error: Failed to create temporary file for /run/user/1000/ccache-tmp/tmp.cpp_stdout.zdMorR: Read-only file system
ccache: error: Failed to create temporary file for /run/user/1000/ccache-tmp/tmp.cpp_stdout.A7CGmu: Read-only file system
ccache: error: Failed to create temporary file for /run/user/1000/ccache-tmp/tmp.cpp_stdout.hAwjZX: Read-only file system
```

This is intentional, as no program running in the sandbox is allowed to have access to `/run/user/1000`.

On top of that, even if `ccache` would have access to this folder, the cache would be lost after the package was built since `ccache` has no access to the user's `ccache` folder in `~/.ccache`.

## What to do?

You can work around the issue by

1. giving `ccache` access to your `~/.ccache` folder. In theory you can attach any local foder to `~/.ccache` in the sandbox, but for simplicity it is assumed that your `ccache` folder in in your home directory.
2. asking `ccache` to use a different temporary directory. As `/tmp` is created only for this sandbox, we simply choose a folder below.

In order to implement the aforementioned changes, all you need to do is create `~/.config/rua/wrap_args.d/ccache.sh` with the following content:

```
wrap_args+=(--bind-try ~/.ccache ~/.ccache)
export CCACHE_TEMPDIR=/tmp/ccache
```
