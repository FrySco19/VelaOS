#!/bin/sh
# Headless QEMU smoke test: boots the ISO, captures splash + desktop screenshots (PNG).
set -e
ISO=${1:-build/velaos-x86_64.iso}; OUT=${2:-build/shots}
mkdir -p "$OUT"; MON=/tmp/vela-qmon.$$
qemu-system-x86_64 -m 256 -cdrom "$ISO" -display none -serial file:"$OUT/serial.log" -monitor unix:$MON,server,nowait &
QPID=$!
mon() { echo "$1" | socat - unix-connect:$MON >/dev/null; }
sleep 7; mon "screendump $OUT/splash.ppm"
sleep 8
for k in n e o f e t c h ret; do mon "sendkey $k"; sleep 0.2; done
sleep 1; mon "screendump $OUT/desktop.ppm"
kill $QPID
for f in splash desktop; do python3 -c "from PIL import Image; Image.open('$OUT/$f.ppm').save('$OUT/$f.png')"; rm "$OUT/$f.ppm"; done
cat "$OUT/serial.log"
