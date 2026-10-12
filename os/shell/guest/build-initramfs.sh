#!/usr/bin/env bash
# Build the OS-0 guest: Debian x86_64 kernel + busybox + velashell in an initramfs.
#   KERNEL_ROOT  extracted linux-image deb (has boot/vmlinuz-* and usr/lib/modules/*)
#   BUSYBOX      static busybox binary
# On a Debian/Ubuntu build host both are fetched automatically with apt-get download.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"; ROOT="$(cd "$HERE/.." && pwd)"
OUT="$ROOT/build/guest"; CACHE="$ROOT/build/cache"
mkdir -p "$OUT" "$CACHE"
export PATH="$PATH:/usr/sbin:/sbin"

if [ -z "${KERNEL_ROOT:-}" ]; then
  KERNEL_ROOT="$CACHE/kernel"
  if [ ! -d "$KERNEL_ROOT/boot" ]; then
    ( cd "$CACHE" && pkg=$(apt-cache depends linux-image-amd64 | awk '/Depends: linux-image-/{print $2; exit}') \
      && apt-get download "$pkg" && dpkg-deb -x "$pkg"_*.deb "$KERNEL_ROOT" )
  fi
fi
if [ -z "${BUSYBOX:-}" ]; then
  BUSYBOX="$CACHE/busybox/usr/bin/busybox"
  [ -x "$BUSYBOX" ] || ( cd "$CACHE" && apt-get download busybox-static && dpkg-deb -x busybox-static_*.deb "$CACHE/busybox" )
fi
VMLINUZ=$(ls "$KERNEL_ROOT"/boot/vmlinuz-* | head -1)
KVER=$(basename "$VMLINUZ" | sed 's/^vmlinuz-//')
MODSRC=$(ls -d "$KERNEL_ROOT"/usr/lib/modules/"$KVER" "$KERNEL_ROOT"/lib/modules/"$KVER" 2>/dev/null | head -1 || true)

STAGE="$OUT/rootfs"; rm -rf "$STAGE"
mkdir -p "$STAGE"/{bin,dev,proc,sys,tmp,usr/bin,usr/share/vela/fonts,lib/vela-modules}
install -m755 "$BUSYBOX" "$STAGE/bin/busybox"
install -m755 "$ROOT/build/velashell" "$STAGE/usr/bin/velashell"
install -m755 "$HERE/init" "$STAGE/init"
cp "$ROOT"/assets/fonts/*.otf "$ROOT"/assets/fonts/*.ttf "$STAGE/usr/share/vela/fonts/"

# Display (QEMU std VGA, virtio-gpu, VirtualBox VMSVGA/VBoxVGA) + input drivers, dependency-ordered.
DEPROOT="$CACHE/depmod"; rm -rf "$DEPROOT"; mkdir -p "$DEPROOT/lib/modules"
cp -r "$MODSRC" "$DEPROOT/lib/modules/"
depmod -b "$DEPROOT" "$KVER"
: > "$STAGE/lib/vela-modules.list"
for m in evdev psmouse bochs virtio_gpu vmwgfx vboxvideo; do
  modprobe -d "$DEPROOT" -S "$KVER" --show-depends "$m" 2>/dev/null | awk '/^insmod/{print $2}'
done | awk '!seen[$0]++' | while read -r ko; do
  name=$(basename "$ko" .xz); name=${name%.zst}
  case "$ko" in *.xz) xz -dc "$ko" > "$STAGE/lib/vela-modules/$name";; *.zst) zstd -dcq "$ko" > "$STAGE/lib/vela-modules/$name";; *) cp "$ko" "$STAGE/lib/vela-modules/$name";; esac
  echo "$name" >> "$STAGE/lib/vela-modules.list"
done

( cd "$STAGE" && find . -print0 | cpio --null -o -H newc --quiet | gzip -9 ) > "$OUT/initramfs.cpio.gz"
cp "$VMLINUZ" "$OUT/vmlinuz"
echo "guest kernel : $OUT/vmlinuz ($KVER)"
echo "guest initrd : $OUT/initramfs.cpio.gz ($(du -h "$OUT/initramfs.cpio.gz" | cut -f1))"
echo "modules      : $(tr '\n' ' ' < "$STAGE/lib/vela-modules.list")"
