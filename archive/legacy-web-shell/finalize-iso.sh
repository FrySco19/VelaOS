#!/bin/sh
set -eu
INPUT=$1
OUTPUT=$2
SRC=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
TMP=$(mktemp -d /tmp/vela-menu.XXXXXX)
xorriso -osirrox on -indev "$INPUT" -extract /isolinux/isolinux.cfg "$TMP/isolinux.cfg" -extract /boot/grub/config.cfg "$TMP/config.cfg"
xorriso -osirrox on -indev "$INPUT" -extract /isolinux/live.cfg "$TMP/live.cfg" -extract /boot/grub/grub.cfg "$TMP/grub.cfg"
sed -i 's/^timeout 0$/timeout 10/' "$TMP/isolinux.cfg"
printf '\nset timeout=1\n' >> "$TMP/config.cfg"
sed -i 's/boot=live components/boot=live noeject components/g' "$TMP/live.cfg" "$TMP/grub.cfg"
xorriso -indev "$INPUT" -outdev "$OUTPUT" -volid VELA_PREVIEW -boot_image any replay -map "$TMP/isolinux.cfg" /isolinux/isolinux.cfg -map "$TMP/config.cfg" /boot/grub/config.cfg -map "$TMP/live.cfg" /isolinux/live.cfg -map "$TMP/grub.cfg" /boot/grub/grub.cfg -map "$SRC/LICENSE" /vela-info/LICENSE -map "$SRC/THIRD_PARTY.md" /vela-info/THIRD_PARTY.md -commit -end
