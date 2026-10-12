#!/usr/bin/env bash
# Boot the OS-0 guest in QEMU (x86_64, std VGA -> bochs DRM -> /dev/fb0 @1920x1080).
#   ./guest/run-qemu.sh            window (GTK/SDL, whatever QEMU defaults to)
#   ./guest/run-qemu.sh --headless no window; HMP monitor on build/guest/monitor.sock
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"; G="$ROOT/build/guest"
DISPLAY_ARGS=(); MON=(-monitor stdio)
if [ "${1:-}" = "--headless" ]; then
  DISPLAY_ARGS=(-display none); MON=(-monitor "unix:$G/monitor.sock,server,nowait"); shift
fi
ACCEL=(); [ "${VELA_KVM:-0}" = 1 ] && ACCEL=(-enable-kvm -cpu host)   # opt-in: nested KVM can hang
exec qemu-system-x86_64 "${ACCEL[@]}" -m 1024 -smp 2 \
  -vga std "${DISPLAY_ARGS[@]}" "${MON[@]}" \
  -serial "file:$G/serial.log" \
  -kernel "$G/vmlinuz" -initrd "$G/initramfs.cpio.gz" \
  -append "console=ttyS0 quiet loglevel=3 video=1920x1080 vt.global_cursor_default=0 ${VELA_APPEND:-}" "$@"
