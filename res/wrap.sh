#!/bin/bash -euET

wrap_args=(--unshare-user --unshare-ipc --unshare-pid --unshare-uts --unshare-cgroup)

for filename in ~/.config/rua/wrap_args.d/*.sh ; do
  test -e "$filename" || continue
  source "$filename"
done

exec nice -n19 \
  ionice -c idle \
  bwrap \
  --new-session --die-with-parent \
  --ro-bind / / \
  --dev /dev \
  --proc /proc \
  --tmpfs /tmp \
  --tmpfs ~ \
  --ro-bind-try "${GNUPGHOME:-$HOME/.gnupg}/pubring.kbx" "${GNUPGHOME:-$HOME/.gnupg}/pubring.kbx" \
  --ro-bind-try "${GNUPGHOME:-$HOME/.gnupg}/pubring.gpg" "${GNUPGHOME:-$HOME/.gnupg}/pubring.gpg" \
  "${wrap_args[@]}" \
  --ro-bind ~/.config/rua ~/.config/rua \
  --ro-bind "$PWD" "$PWD" \
  --seccomp 3 3< "$RUA_SECCOMP_FILE" \
  "$@"
