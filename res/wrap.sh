#!/bin/bash -euET

wrap_args=()

if test -e ~/.config/rua/wrap_args.sh; then
  source ~/.config/rua/wrap_args.sh
fi

exec nice -n19 \
	ionice -c idle \
	bwrap --unshare-user --unshare-ipc --unshare-pid --unshare-uts --unshare-cgroup \
	--new-session --die-with-parent \
	--ro-bind / / \
	--dev /dev \
	--tmpfs /tmp \
	--tmpfs ~ \
	--ro-bind "$RUA_CONFIG_DIR" "$RUA_CONFIG_DIR" \
	--ro-bind "$PWD" "$PWD" \
	--seccomp 3 \
	"${wrap_args[@]}" \
	"$@" 3< "$RUA_SECCOMP_FILE"
