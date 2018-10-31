#!/bin/bash -euET

wrap_args=()
test -e ~/.config/rua/wrap_args.sh && source ~/.config/rua/wrap_args.sh

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
	--ro-bind "$PWD" "$PWD" \
	--seccomp 3 \
	"${wrap_args[@]}" \
	"$@" 3< "$RUA_SECCOMP_FILE"
