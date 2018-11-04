#!/bin/bash -euET

wrap_args=()

if test -d ~/.gnupg; then
  wrap_args+=(--ro-bind ~/.gnupg ~/.gnupg)
fi
if test -d ~/.gnupg/private-keys-v1.d; then
  wrap_args+=(--tmpfs ~/.gnupg/private-keys-v1.d)
fi
if test -d ~/.gnupg/openpgp-revocs.d; then
  wrap_args+=(--tmpfs ~/.gnupg/openpgp-revocs.d)
fi

for filename in ~/.config/rua/wrap_args.d/*.sh ; do
  [ -e "$filename" ] || continue
  source "$filename"
done

exec nice -n19 \
  ionice -c idle \
  bwrap --unshare-user --unshare-ipc --unshare-pid --unshare-uts --unshare-cgroup \
  --new-session --die-with-parent \
  --ro-bind / / \
  --dev /dev \
  --tmpfs /tmp \
  --tmpfs ~ \
  --ro-bind ~/.config/rua ~/.config/rua \
  --seccomp 3 \
  "${wrap_args[@]}" \
  --ro-bind "$PWD" "$PWD" \
  "$@" 3< "$RUA_SECCOMP_FILE"
