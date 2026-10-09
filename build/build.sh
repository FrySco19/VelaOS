#!/bin/sh
# Vela OS 2 — full ISO build with live-build (Debian 13 trixie, amd64, BIOS+EFI hybrid).
# Usage: build/build.sh [--test] [--purge] [--release]
#   --test     add openssh-server + dev SSH key (for QEMU/dev VM). NEVER for user releases.
#   --purge    drop the package cache too (slow; normally not needed)
#   --release  stronger squashfs compression (zstd 19) and release file name
# Output: build/out/Vela-<ver>[-test]-<stamp>.iso (+ .sha256) and build/out/latest[-test].iso symlink.
set -eu
. "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

TEST=0; PURGE=0; RELEASE=0
for a in "$@"; do case $a in
  --test) TEST=1;; --purge) PURGE=1;; --release) RELEASE=1;;
  -h|--help) sed -n 2,8p "$0"; exit 0;;
  *) die "unknown option $a";; esac; done

[ "$(id -u)" = 0 ] || exec sudo -E env VELA_BUILD_DIR="$VELA_BUILD_DIR" VELA_OUT="$VELA_OUT" "$0" "$@"

exec 9>"$VELA_LOCK"
flock -n 9 || { log "waiting for build lock $VELA_LOCK"; flock 9; }

LEVEL=${VELA_SQUASH_LEVEL:-9}; [ "$RELEASE" = 1 ] && LEVEL=19
BOOTAPPEND="boot=live components username=vela hostname=vela locales=it_IT.UTF-8 keyboard-layouts=it timezone=Europe/Rome quiet splash loglevel=3 rd.udev.log_level=3 systemd.show_status=auto zswap.enabled=0"
[ "$TEST" = 1 ] && BOOTAPPEND="$BOOTAPPEND console=ttyS0,115200 console=tty0"

mkdir -p "$VELA_BUILD_DIR" "$VELA_OUT"
cd "$VELA_BUILD_DIR"
if [ -d config ]; then
  if [ "$PURGE" = 1 ]; then lb clean --purge; else lb clean; fi
  rm -rf config
fi

log "lb config (test=$TEST, squashfs zstd $LEVEL)"
lb config \
  --distribution trixie --architectures amd64 \
  --archive-areas 'main contrib non-free non-free-firmware' \
  --binary-images iso-hybrid --bootloaders 'syslinux grub-efi' \
  --debian-installer none --memtest none \
  --apt-recommends false \
  --firmware-chroot false --firmware-binary false \
  --cache true --cache-packages true --cache-stages bootstrap \
  --debootstrap-options '--include=ca-certificates' \
  --mirror-bootstrap http://deb.debian.org/debian/ --mirror-chroot http://deb.debian.org/debian/ --mirror-binary http://deb.debian.org/debian/ \
  --mirror-chroot-security http://deb.debian.org/debian-security/ --mirror-binary-security http://deb.debian.org/debian-security/ \
  --chroot-squashfs-compression-type zstd --chroot-squashfs-compression-level "$LEVEL" \
  --iso-volume VELA_2 --iso-application "Vela OS $VELA_VERSION" --iso-publisher "Vela" \
  --bootappend-live "$BOOTAPPEND" \
  >/dev/null

# Archives: Debian fasttrack (VirtualBox guest additions 7.2.x matching VirtualBox)
for kind in chroot binary; do
  printf '%s\n' 'deb https://fasttrack.debian.net/debian-fasttrack trixie-fasttrack main contrib' > "config/archives/fasttrack.list.$kind"
  cp "$VELA_ROOT/build/fasttrack.asc" "config/archives/fasttrack.key.$kind"
done
mkdir -p config/includes.bootstrap/etc/ssl/certs config/includes.bootstrap/etc/apt/apt.conf.d
cp /etc/ssl/certs/ca-certificates.crt config/includes.bootstrap/etc/ssl/certs/
printf '%s\n' 'Acquire::https::CaInfo "/etc/ssl/certs/ca-certificates.crt";' > config/includes.bootstrap/etc/apt/apt.conf.d/99-vela-ca

# Package pool on the ISO: Calamares (Debian helpers) installs the bootloader and LUKS tools
# from the live medium ("deb file:/run/live/medium trixie main"), so they must be in /pool.
# live-build resolves each list independently against an empty dpkg status.
# BIOS and EFI metapackages conflict and must be downloaded in separate lists.
printf '%s\n' grub-pc > config/package-lists/vela-installer-bios.list.binary
printf '%s\n' grub-efi-amd64 grub-efi > config/package-lists/vela-installer-efi.list.binary
printf '%s\n' cryptsetup-initramfs keyutils > config/package-lists/vela-installer.list.binary

log "assembling components: $VELA_COMPONENTS"
vela_assemble config "$TEST"
ls config/package-lists config/hooks/normal | sed 's/^/  /' >&2

STAMP=$(date +%Y%m%d-%H%M)
LOGF="$VELA_OUT/build-$STAMP.log"
log "lb build (log: $LOGF)"
START=$(date +%s)
if ! lb build >"$LOGF" 2>&1; then
  tail -40 "$LOGF" >&2; die "lb build failed, see $LOGF"
fi
[ -f live-image-amd64.hybrid.iso ] || { tail -40 "$LOGF" >&2; die "no ISO produced, see $LOGF"; }
log "lb build ok in $(( $(date +%s) - START ))s"
# Inspect the compressed filesystem that will actually be published.
VERIFY=$(mktemp -d /var/tmp/vela-verify.XXXXXX)
trap 'rm -rf "$VERIFY"' EXIT INT TERM
unsquashfs -quiet -d "$VERIFY/root" binary/live/filesystem.squashfs \
  etc/vela usr/bin/vela-* usr/share/plasma/look-and-feel/org.vela.desktop \
  usr/share/plasma/layout-templates/org.vela.desktop.defaultPanels \
  usr/share/vela/store/featured.json \
  usr/lib/x86_64-linux-gnu/qt6/plugins/plasma/kcms/systemsettings/kcm_vela_*.so
sh "$VELA_ROOT/build/verify-integrated.sh" "$VERIFY/root"
rm -rf "$VERIFY"
trap - EXIT INT TERM
# Sanity: the kernel the ISO boots must have its modules in the squashfs
KV=$(file -bL binary/live/vmlinuz | sed -n 's/.*version \([^ ]*\).*/\1/p')
if [ -n "$KV" ] && ! unsquashfs -l binary/live/filesystem.squashfs "usr/lib/modules/$KV" 2>/dev/null | grep -q "modules/$KV"; then
  die "kernel $KV in /live/vmlinuz has no modules in filesystem.squashfs"
fi

SUFFIX=""; [ "$TEST" = 1 ] && SUFFIX="-test"
NAME="Vela-$VELA_VERSION$SUFFIX-$STAMP.iso"; [ "$RELEASE" = 1 ] && NAME="Vela-$VELA_VERSION.iso"
sh "$VELA_ROOT/build/finalize.sh" live-image-amd64.hybrid.iso "$VELA_OUT/$NAME"
# remember what the chroot was built as (quick.sh reuses it)
echo "$TEST" > "$VELA_BUILD_DIR/.vela-test"
cp "$VELA_OUT/$NAME" "$VELA_BUILD_DIR/last.iso" 2>/dev/null || true
ln -sfn "$NAME" "$VELA_OUT/latest$SUFFIX.iso"
chown -R "${SUDO_UID:-0}:${SUDO_GID:-0}" "$VELA_OUT" 2>/dev/null || true
log "DONE: $VELA_OUT/$NAME ($(du -h "$VELA_OUT/$NAME" | cut -f1))"
