#!/bin/sh
# Persistent Vela dev VM (installed qcow2 with SSH), for fast component iteration.
#   tools/devvm.sh create [ISO] [--efi]   install ISO (test build, default build/out/latest-test.iso) into a new disk,
#                                  then take internal snapshot 'clean'. Replaces any existing dev disk.
#   tools/devvm.sh start [--efi] [--vga std|virtio|vmware]   boot the dev disk (VNC 127.0.0.1:5912, SSH 127.0.0.1:2222)
#   tools/devvm.sh stop | status | ssh [cmd] | screenshot OUT.png
#   tools/devvm.sh revert [SNAP]  stop, revert to snapshot (default clean), start
#   tools/devvm.sh snapshot NAME  stop, create snapshot NAME, start      | tools/devvm.sh snapshots
# Login: vela / vela (passwordless sudo, autologin Plasma Wayland). Disk: $VELA_ROOT/.local/devvm/vela-dev.qcow2
# Concurrency: state-changing sessions should hold `flock $VELA_ROOT/coordination/devvm.lock`
#   (deploy.sh does it for you). This script takes the lock for create/revert/snapshot.
set -eu
TOOLS=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd); ROOT=$(dirname "$TOOLS")
VELA_ROOT=$ROOT
DISK=${VELA_DEV_DISK:-$VELA_ROOT/.local/devvm/vela-dev.qcow2}
LOCK=$ROOT/coordination/devvm.lock
VM="$TOOLS/vm.sh -n dev"
die() { echo "devvm.sh: $*" >&2; exit 1; }
withlock() { exec 8>"$LOCK"; flock -w 1800 8 || die "devvm lock busy"; }
cmd=${1:-}; [ -n "$cmd" ] || { sed -n 2,12p "$0"; exit 2; }; shift
case $cmd in
  create)
    withlock
    ISO=$ROOT/build/out/latest-test.iso; BOOTOPT=""
    for a in "$@"; do case $a in --efi) BOOTOPT=--efi;; *) ISO=$a;; esac; done
    [ -f "$ISO" ] || die "no ISO $ISO"
    $VM stop 30 >/dev/null 2>&1 || true
    mkdir -p "$(dirname "$DISK")"; rm -f "$DISK"
    qemu-img create -f qcow2 "$DISK" 40G >/dev/null
    rm -f $VELA_ROOT/.local/run/dev/OVMF_VARS.fd
    $VM boot-iso "$ISO" --disk "$DISK" $BOOTOPT
    $VM wait-ssh 900
    $VM scp-to "$TOOLS/devvm-install.sh" /tmp/devvm-install.sh
    $VM ssh "sudo sh /tmp/devvm-install.sh /dev/vda" 2>&1 | tee $VELA_ROOT/.local/devvm/install.log
    $VM stop 60
    qemu-img snapshot -c clean "$DISK"
    echo "dev VM created; snapshot 'clean' taken. Start with: tools/devvm.sh start";;
  start) [ -f "$DISK" ] || die "no dev disk; run tools/devvm.sh create"; $VM boot-disk "$DISK" "$@"; $VM wait-ssh 900;;
  stop) $VM stop "${1:-90}";;
  status) $VM status; qemu-img snapshot -l "$DISK" 2>/dev/null | tail -n +3 || true;;
  snapshots) qemu-img snapshot -l "$DISK";;
  ssh) $VM ssh "$@";;
  screenshot) $VM screenshot "$@";;
  revert)
    withlock; s=${1:-clean}; $VM stop 60 >/dev/null; qemu-img snapshot -a "$s" "$DISK"; $VM boot-disk "$DISK"; $VM wait-ssh 900
    echo "reverted to '$s'";;
  snapshot)
    withlock; s=${1:?name}; $VM stop 60 >/dev/null; qemu-img snapshot -d "$s" "$DISK" 2>/dev/null || true
    qemu-img snapshot -c "$s" "$DISK"; $VM boot-disk "$DISK"; $VM wait-ssh 900; echo "snapshot '$s' created";;
  *) die "unknown command $cmd";;
esac
