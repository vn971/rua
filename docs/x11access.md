## What

Access to your X11 session might not work from inside a RUA build. This is intentional.
Sometimes, however, the current restricting measures are not enough. And sometimes you, on the contrary, may want explicit X11 access.
This is explained in sections below.

## When?

Whether your X11 session is available to the build depends not only on RUA, but also on configuration of your system.

* If you build the package with internet access being disabled, X11 is disallowed.
* If you run your Xorg server with `-nolisten tcp -nolisten local`, X11 is disallowed.
* If none of the above apply, a build might totally skip file access restrictions
and access your X11 via the abstract socket (as opposed to the normal unix sockets).
You can see the full list of abstract sockets on your system via:
```
ss -l | grep @
```
If you have `@/tmp/.X11-unix` here, a build might access it.
This has nothing to do with RUA specifically, just an explanation how RUA fits
into the bigger picture here.

Now, what can or should you do about it?

## What to do?

If you're writing a PKGBUILD and want to access X11 for e.g. testing/CI,
don't rely on already existing session (it might not even exist!).
Instead, create a new virtual one. It is as easy as running
```
Xvfb :12345 &
export DISPLAY=':12345'
your_normal_command
```

If you encounter a package that needs X11 access,
modify it locally with the lines above and ask the package maintainer to do a better job here as well.
