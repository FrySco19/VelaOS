#!/bin/sh
# Vela OS 2 — quick remaster (minutes): reuse the chroot of the last full build
# ($VELA_BUILD_DIR/chroot), apply current component packages/overlays/hooks, re-squash,
# swap it into the last ISO. Cumulative: hooks must be idempotent (PLAN rule).
# Usage: build/quick.sh [--test|--no-test] [--components "base theme ..."]
# Default test flag = same as the last full build. Output: build/out/Vela-<ver>[-test]-<stamp>-q.iso
# Run a full build/build.sh when the package set changes a lot or before a release.
set -eu
. "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/lib.sh"
TEST=""
while [ $# -gt 0 ]; do case $1 in
  --test) TEST=1;; --no-test) TEST=0;; --components) VELA_COMPONENTS=$2; shift;;
  *) die "unknown option $1";; esac; shift; done
[ "$(id -u)" = 0 ] || exec sudo -E env VELA_BUILD_DIR="$VELA_BUILD_DIR" VELA_OUT="$VELA_OUT" VELA_COMPONENTS="$VELA_COMPONENTS" "$0" ${TEST:+$( [ "$TEST" = 1 ] && echo --test || echo --no-test )}
exec 9>"$VELA_LOCK"; flock -n 9 || { log "waiting for build lock"; flock 9; }
B=$VELA_BUILD_DIR; CH=$B/chroot
[ -d "$CH/usr" ] && [ -f "$B/last.iso" ] || die "no previous full build in $B (run build/build.sh first)"
[ -n "$TEST" ] || TEST=$(cat "$B/.vela-test" 2>/dev/null || echo 0)
[ "$TEST" = "$(cat "$B/.vela-test" 2>/dev/null || echo 0)" ] || log "WARNING: test flag differs from last full build; only additive (openssh is not removed)"
STAGE=$(mktemp -d /var/tmp/vela-quick.XXXXXX)
cleanup() {
  for m in dev/pts dev proc sys run; do mountpoint -q "$CH/$m" && umount -l "$CH/$m" || true; done
  rm -f "$CH/usr/sbin/policy-rc.d"; rm -rf "$STAGE"
}
trap cleanup EXIT INT TERM
vela_assemble "$STAGE" "$TEST"
for m in dev dev/pts proc sys run; do mount --bind "/$m" "$CH/$m"; done
printf '#!/bin/sh\nexit 101\n' > "$CH/usr/sbin/policy-rc.d"; chmod 755 "$CH/usr/sbin/policy-rc.d"
cp -L /etc/resolv.conf "$CH/etc/resolv.conf.vela-quick" 2>/dev/null || true
RESOLV_BAK=""; if [ -e "$CH/etc/resolv.conf" ] || [ -L "$CH/etc/resolv.conf" ]; then RESOLV_BAK=1; mv "$CH/etc/resolv.conf" "$CH/etc/resolv.conf.vela-orig"; fi
mv "$CH/etc/resolv.conf.vela-quick" "$CH/etc/resolv.conf"

log "packages"
PKGS=$(cat "$STAGE"/package-lists/*.list.chroot 2>/dev/null | sort -u | tr '\n' ' ')
if ! chroot "$CH" apt-get update -qq; then log "apt update failed (mirror problem); continuing with cached packages"; fi
MISSING=$(chroot "$CH" sh -c "for p in $PKGS; do dpkg -s \$p >/dev/null 2>&1 || echo \$p; done")
if [ -n "$MISSING" ]; then
  log "installing: $MISSING"
  chroot "$CH" sh -c "DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends $MISSING" || die "package install failed"
fi
log "overlays"
cp -a "$STAGE/includes.chroot/." "$CH/"
log "hooks"
for h in "$STAGE"/hooks/normal/*.hook.chroot; do
  [ -e "$h" ] || continue
  log "  $(basename "$h")"
  cp "$h" "$CH/root/"; chroot "$CH" "/root/$(basename "$h")" || die "hook $(basename "$h") failed"; rm -f "$CH/root/$(basename "$h")"
done
chroot "$CH" sh -c 'apt-get clean; rm -rf /var/lib/apt/lists/*_Packages /tmp/* 2>/dev/null; truncate -s0 /etc/machine-id; rm -f /var/lib/dbus/machine-id' || true
rm -f "$CH/etc/resolv.conf"; [ -n "$RESOLV_BAK" ] && mv "$CH/etc/resolv.conf.vela-orig" "$CH/etc/resolv.conf"
rm -f "$CH/usr/sbin/policy-rc.d"
for m in dev/pts dev proc sys run; do umount -l "$CH/$m" || true; done

log "squashfs"
LEVEL=${VELA_SQUASH_LEVEL:-6}
SQ=$STAGE/filesystem.squashfs
mksquashfs "$CH" "$SQ" -comp zstd -Xcompression-level "$LEVEL" -noappend -quiet -wildcards \
  -e 'proc/*' 'sys/*' 'dev/*' 'run/*' 'tmp/*' 'live-build' 'root/*.hook.chroot' >/dev/null
du -sx --block-size=1 "$CH" | cut -f1 > "$SQ.size"
chroot "$CH" dpkg-query -W --showformat='${Package} ${Version}\n' > "$SQ.packages" 2>/dev/null || true
VMLINUZ=$(ls -1 "$CH"/boot/vmlinuz-* | sort -V | tail -1); INITRD=$(ls -1 "$CH"/boot/initrd.img-* | sort -V | tail -1)
SUFFIX=""; [ "$TEST" = 1 ] && SUFFIX="-test"
NAME="Vela-$VELA_VERSION$SUFFIX-$(date +%Y%m%d-%H%M)-q.iso"
sh "$VELA_ROOT/build/finalize.sh" "$B/last.iso" "$VELA_OUT/$NAME" "$SQ" "$VMLINUZ" "$INITRD"
ln -sfn "$NAME" "$VELA_OUT/latest$SUFFIX.iso"
chown -R "${SUDO_UID:-0}:${SUDO_GID:-0}" "$VELA_OUT" 2>/dev/null || true
log "DONE: $VELA_OUT/$NAME ($(du -h "$VELA_OUT/$NAME" | cut -f1))"
