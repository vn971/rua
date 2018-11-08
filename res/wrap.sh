#!/bin/bash -euET

namespace_args=(--unshare-user --unshare-ipc --unshare-pid --unshare-uts --unshare-cgroup)
gnupg_args=(
  --tmpfs "${GNUPGHOME:-$HOME/.gnupg}"/private-keys-v1.d
  --tmpfs "${GNUPGHOME:-$HOME/.gnupg}"/openpgp-revocs.d
  --ro-bind-try "${GNUPGHOME:-$HOME/.gnupg}" "${GNUPGHOME:-$HOME/.gnupg}"
)
wrap_args=()

for filename in ~/.config/rua/wrap_args.d/*.sh ; do
  test -e "$filename" || continue
  source "$filename"
done

exec nice -n19 \
  ionice -c idle \
  bwrap \
  "${namespace_args[@]}" \
  --new-session --die-with-parent \
  --ro-bind / / \
  --dev /dev \
  --tmpfs /tmp \
  --tmpfs ~ \
  "${gnupg_args[@]}" \
  "${wrap_args[@]}" \
  --ro-bind ~/.config/rua ~/.config/rua \
  --ro-bind "$PWD" "$PWD" \
  --seccomp 3 \
  "$@" 3< "$RUA_SECCOMP_FILE"
