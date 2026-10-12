#!/bin/sh
# Headless QEMU smoke test: boots the ISO and captures PNGs:
# splash, first-boot welcome, desktop, focus mode.
set -e
ISO=${1:-build/velaos-x86_64.iso}; OUT=${2:-build/shots}
mkdir -p "$OUT"; MON=/tmp/vela-qmon.$$
qemu-system-x86_64 -m 512 -vga std -cdrom "$ISO" -display none -serial file:"$OUT/serial.log" -monitor unix:$MON,server,nowait &
QPID=$!
mon() { echo "$1" | socat - unix-connect:$MON >/dev/null; }
keys() { for k in "$@"; do mon "sendkey $k"; sleep 0.15; done; }
shot() { mon "screendump $OUT/$1.ppm"; sleep 0.5; python3 -c "from PIL import Image; Image.open('$OUT/$1.ppm').save('$OUT/$1.png')"; rm "$OUT/$1.ppm"; }
sleep 6; shot splash
sleep 7; shot welcome
keys ret; sleep 1
keys n e o f e t c h ret; sleep 1.5; shot desktop
keys f11; sleep 1; shot focus
keys esc; sleep 0.5
keys t h e m e spc d e e p ret; sleep 2; shot desktop-deep
keys t h e m e spc d a w n ret; sleep 3; shot desktop-dawn
kill $QPID
cat "$OUT/serial.log"
