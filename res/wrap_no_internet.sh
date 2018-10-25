#!/bin/bash -euET
{
set -o pipefail

exec "$RUA_CONFIG_DIR"/wrap_yes_internet.sh --unshare-net "$@"

exit
}
