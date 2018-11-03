#!/bin/bash -euET

wrap_args=()

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
  --ro-bind ~/.gnupg ~/.gnupg --tmpfs ~/.gnupg/private-keys-v1.d \
  --ro-bind ~/.config/rua ~/.config/rua \
  --seccomp 3 \
  "${wrap_args[@]}" \
  --ro-bind "$PWD" "$PWD" \
  "$@" 3< "$RUA_SECCOMP_FILE"
