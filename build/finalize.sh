#!/bin/sh
# finalize.sh IN.iso OUT.iso [new-squashfs] [new-vmlinuz] [new-initrd]
# Writes Vela boot menus (isolinux for BIOS, GRUB for EFI) with theme assets from
# components/theme/boot (if present), adds docs under /vela, optionally swaps the live
# filesystem/kernel/initrd (quick.sh), writes OUT.iso + OUT.iso.sha256.
# Test ISOs (detected by console=ttyS0 in the original menu) keep the serial console.
set -eu
. "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)/lib.sh"
IN=$1; OUT=$2; SQ=${3:-}; VM=${4:-}; RD=${5:-}
TMP=$(mktemp -d /tmp/vela-fin.XXXXXX); trap 'rm -rf "$TMP"' EXIT
xorriso -osirrox on -indev "$IN" -extract /isolinux/live.cfg "$TMP/orig-live.cfg" 2>/dev/null
EXTRA=""; grep -q 'console=ttyS0' "$TMP/orig-live.cfg" && EXTRA=" console=ttyS0,115200 console=tty0"
COMMON="boot=live noeject components username=vela hostname=vela timezone=Europe/Rome"
QUIET="quiet splash loglevel=3 rd.udev.log_level=3 systemd.show_status=auto zswap.enabled=0"
IT="$COMMON locales=it_IT.UTF-8 keyboard-layouts=it $QUIET$EXTRA"
EN="$COMMON locales=en_US.UTF-8 keyboard-layouts=us $QUIET$EXTRA"
SAFE="$COMMON locales=it_IT.UTF-8 keyboard-layouts=it nomodeset vela.session=plasmax11$EXTRA"
THEME=$VELA_ROOT/components/theme/boot

# --- BIOS: isolinux ---
cat > "$TMP/isolinux.cfg" <<EOT
include menu.cfg
default vesamenu.c32
prompt 0
timeout 30
EOT
{ echo "menu hshift 0"; echo "menu width 82"; echo "menu title Vela"; echo "include stdmenu.cfg"
  [ -f "$THEME/isolinux/vela.cfg" ] && echo "include vela.cfg"
  echo "include live.cfg"; echo "menu clear"; } > "$TMP/menu.cfg"
cat > "$TMP/live.cfg" <<EOT
label vela
	menu label ^Avvia Vela
	menu default
	linux /live/vmlinuz
	initrd /live/initrd.img
	append $IT

label vela-en
	menu label Start Vela in ^English
	linux /live/vmlinuz
	initrd /live/initrd.img
	append $EN

label vela-safe
	menu label Avvia Vela in modalita' ^sicura (grafica di base)
	linux /live/vmlinuz
	initrd /live/initrd.img
	append $SAFE
EOT
# --- EFI: GRUB ---
{
  echo 'set default=0'; echo 'set timeout=3'; echo 'set timeout_style=menu'
  echo 'insmod all_video'; echo 'insmod gfxterm'; echo 'insmod png'
  echo 'set gfxmode=1024x768,800x600,auto'; echo 'set gfxpayload=keep'
  echo 'if loadfont $prefix/unicode.pf2 ; then true ; fi'
  if [ -f "$THEME/grub/theme.txt" ]; then
    for f in "$THEME"/grub/*.pf2; do echo "loadfont /boot/grub/theme/vela/$(basename "$f")"; done
    echo 'terminal_output gfxterm'; echo 'set theme=/boot/grub/theme/vela/theme.txt'
  else
    echo 'terminal_output gfxterm'; echo 'set menu_color_normal=light-gray/black'; echo 'set menu_color_highlight=white/blue'
  fi
  cat <<EOT
menuentry "Avvia Vela" --hotkey=a {
	linux /live/vmlinuz $IT findiso=\${iso_path}
	initrd /live/initrd.img
}
menuentry "Start Vela in English" --hotkey=e {
	linux /live/vmlinuz $EN findiso=\${iso_path}
	initrd /live/initrd.img
}
menuentry "Avvia Vela in modalità sicura (grafica di base)" --hotkey=s {
	linux /live/vmlinuz $SAFE findiso=\${iso_path}
	initrd /live/initrd.img
}
menuentry "Impostazioni del firmware UEFI" {
	fwsetup
}
EOT
} > "$TMP/grub.cfg"
printf 'set timeout=3\n' > "$TMP/config.cfg"

set -- -map "$TMP/isolinux.cfg" /isolinux/isolinux.cfg -map "$TMP/menu.cfg" /isolinux/menu.cfg \
       -map "$TMP/live.cfg" /isolinux/live.cfg -map "$TMP/grub.cfg" /boot/grub/grub.cfg \
       -map "$TMP/config.cfg" /boot/grub/config.cfg
if [ -d "$THEME" ]; then
  [ -f "$THEME/isolinux/splash.png" ] && set -- "$@" -map "$THEME/isolinux/splash.png" /isolinux/splash.png
  [ -f "$THEME/isolinux/vela.cfg" ] && set -- "$@" -map "$THEME/isolinux/vela.cfg" /isolinux/vela.cfg
  [ -d "$THEME/grub" ] && set -- "$@" -map "$THEME/grub" /boot/grub/theme/vela
fi
for f in README.md VIRTUALBOX.md RELEASE_NOTES.md LICENSE; do
  [ -f "$VELA_ROOT/$f" ] && set -- "$@" -map "$VELA_ROOT/$f" "/vela/$f"
done
if [ -n "$SQ" ]; then
  set -- "$@" -map "$SQ" /live/filesystem.squashfs
  [ -f "$SQ.size" ] && set -- "$@" -map "$SQ.size" /live/filesystem.size
  [ -f "$SQ.packages" ] && set -- "$@" -map "$SQ.packages" /live/filesystem.packages
fi
[ -n "$VM" ] && set -- "$@" -map "$VM" /live/vmlinuz
[ -n "$RD" ] && set -- "$@" -map "$RD" /live/initrd.img
rm -f "$OUT"
xorriso -indev "$IN" -outdev "$OUT" -volid VELA_2 -boot_image any replay "$@" -commit -end 2>"$TMP/xorriso.log" \
  || { cat "$TMP/xorriso.log" >&2; die "xorriso failed"; }
( cd "$(dirname "$OUT")" && sha256sum "$(basename "$OUT")" > "$(basename "$OUT").sha256" )
log "finalized $OUT"
