#!/bin/sh
# tools/deploy.sh <component> [--no-hook] [--no-packages]
# Apply components/<component> to the RUNNING dev VM (tools/devvm.sh start) over SSH:
#   1. apt-get install packages.list entries that are missing
#   2. rsync overlay/ to / (owner root:root, existing dir perms/times untouched)
#   3. run hook.chroot.sh as root (it must be idempotent)
#   If components/<c>/deploy.sh exists it runs INSTEAD of 2+3, on the box, with env:
#   VELA_SSH (ssh command prefix), VELA_RSYNC_RSH, VELA_VM_HOST=vela@127.0.0.1, VELA_COMPONENT_DIR.
# Holds coordination/devvm.lock for the duration. Log out/in or restart plasmashell yourself if needed:
#   tools/vm.sh -n dev ssh 'systemctl --user restart plasma-plasmashell'
set -eu
VELA_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TOOLS=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd); ROOT=$(dirname "$TOOLS")
C=${1:?component}; shift; HOOK=1; PKGS=1
for a in "$@"; do case $a in --no-hook) HOOK=0;; --no-packages) PKGS=0;; esac; done
D=$ROOT/components/$C; [ -d "$D" ] || { echo "no component $C"; exit 1; }
exec 8>"$ROOT/coordination/devvm.lock"; flock -w 1800 8 || { echo "devvm lock busy"; exit 1; }
KEY=${VELA_SSH_KEY:-$VELA_ROOT/.local/ssh/id_ed25519}
PORT=$(cat $VELA_ROOT/.local/run/dev/sshport 2>/dev/null || echo 2222)
OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o BatchMode=yes -i $KEY -p $PORT"
SSH="ssh $OPTS vela@127.0.0.1"
$SSH true || { echo "dev VM not reachable on port $PORT (tools/devvm.sh start)"; exit 1; }
if [ "$PKGS" = 1 ] && [ -f "$D/packages.list" ]; then
  P=$(sed -e 's/#.*//' "$D/packages.list" | tr -s ' \t' '\n' | sed '/^$/d' | tr '\n' ' ')
  if [ -n "$P" ]; then
    echo "[deploy] packages: checking $(echo $P | wc -w)"
    # remote login shell may be fish: always run POSIX code through `sh -s`
    $SSH sh -s <<EOT
missing=''
for p in $P; do dpkg -s \$p >/dev/null 2>&1 || missing="\$missing \$p"; done
if [ -n "\$missing" ]; then echo "installing:\$missing"; sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -q \$missing; fi
EOT
  fi
fi
export VELA_SSH="$SSH" VELA_RSYNC_RSH="ssh $OPTS" VELA_VM_HOST=vela@127.0.0.1 VELA_COMPONENT_DIR="$D"
if [ -x "$D/deploy.sh" ]; then echo "[deploy] running $C/deploy.sh"; exec "$D/deploy.sh"; fi
if [ -d "$D/overlay" ]; then
  echo "[deploy] rsync overlay"
  rsync -rlt -O --chown=0:0 --rsync-path="sudo rsync" -e "ssh $OPTS" "$D/overlay/" vela@127.0.0.1:/
fi
if [ "$HOOK" = 1 ] && [ -f "$D/hook.chroot.sh" ]; then
  echo "[deploy] running hook"
  rsync -t -e "ssh $OPTS" "$D/hook.chroot.sh" vela@127.0.0.1:/tmp/vela-hook-$C.sh
  $SSH "sudo sh /tmp/vela-hook-$C.sh"
fi
echo "[deploy] $C deployed"
