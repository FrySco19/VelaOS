#!/bin/sh
# Vela VM harness: QEMU/KVM headless with VNC + QMP + serial log + SSH forward.
#
#   tools/vm.sh [-n NAME] boot-iso ISO   [--efi] [--vga virtio|std|vmware|qxl] [--mem MB] [--cpus N]
#                                        [--disk FILE.qcow2] [--ssh PORT] [--vnc N] [--snapshot]
#   tools/vm.sh [-n NAME] boot-disk DISK [--efi] [--cdrom ISO] [same options]
#   tools/vm.sh [-n NAME] screenshot OUT.png
#   tools/vm.sh [-n NAME] key ctrl-alt-t           (QEMU qcode names joined by '-', e.g. meta_l, ret, esc, f2)
#   tools/vm.sh [-n NAME] type "testo"  [--layout it|us]   (guest layout, default it)
#   tools/vm.sh [-n NAME] mouse X Y [click|rclick|dclick|none]   (absolute, usb-tablet)
#   tools/vm.sh [-n NAME] ssh [cmd...]   |  wait-ssh [secs]  |  scp-to SRC DST  |  scp-from SRC DST
#   tools/vm.sh [-n NAME] stop [secs]    (ACPI power off, then hard quit)  |  kill  |  status  |  info
#   tools/vm.sh [-n NAME] qmp-path | vnc | serial
#
# Names: iso (default; VNC 127.0.0.1:5911, SSH 127.0.0.1:2223) and dev (VNC 5912, SSH 2222). Others: VNC 5919, SSH 2229.
# (5900-5902 are taken by the box desktops.)
# Runtime files: $VELA_ROOT/.local/run/<name>/{qmp.sock,pid,serial.log,qemu.log,OVMF_VARS.fd}
# SSH: user 'vela' with key $VELA_ROOT/.local/ssh/id_ed25519 (test builds only),
#      password fallback: live ISO 'live', dev VM 'vela'.
# No GPU on the box: virtio-vga-gl is unavailable; default virtio-vga (guest renders with llvmpipe).
# KVM IS BROKEN ON THIS BOX (host kernel BUG in kvm_arch_vcpu_create, qemu hangs): default accel is TCG
# (software emulation, ~5-10x slower). --kvm / VELA_ACCEL=kvm only if someone confirms KVM works again.
set -eu
VELA_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TOOLS=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
RUNROOT=${VELA_RUN:-$VELA_ROOT/.local/run}
KEY=${VELA_SSH_KEY:-$VELA_ROOT/.local/ssh/id_ed25519}
NAME=iso
if [ "${1:-}" = "-n" ]; then NAME=$2; shift 2; fi
case $NAME in iso) DVNC=11; DSSH=2223;; dev) DVNC=12; DSSH=2222;; *) DVNC=19; DSSH=2229;; esac
RUN=$RUNROOT/$NAME; QMP=$RUN/qmp.sock
mkdir -p "$RUN"
die() { echo "vm.sh: $*" >&2; exit 1; }
running() { [ -f "$RUN/pid" ] && kill -0 "$(cat "$RUN/pid")" 2>/dev/null; }
sshport() { cat "$RUN/sshport" 2>/dev/null || echo "$DSSH"; }
SSHOPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ConnectTimeout=5"

boot() {  # $1=iso|disk $2=file
  mode=$1; file=$2; shift 2
  running && die "VM '$NAME' already running (pid $(cat "$RUN/pid")); use: tools/vm.sh -n $NAME stop"
  EFI=0; VGA=virtio; MEM=${VELA_MEM:-3072}; CPUS=2; DISK=""; SSHP=$DSSH; VNC=$DVNC; SNAP=""; CDROM=""; ACCEL=${VELA_ACCEL:-tcg}
  while [ $# -gt 0 ]; do case $1 in
    --efi) EFI=1;; --bios) EFI=0;; --vga) VGA=$2; shift;; --mem) MEM=$2; shift;; --cpus) CPUS=$2; shift;;
    --disk) DISK=$2; shift;; --ssh) SSHP=$2; shift;; --vnc) VNC=$2; shift;; --snapshot) SNAP=-snapshot;;
    --cdrom) CDROM=$2; shift;; --kvm) ACCEL=kvm;; --tcg) ACCEL=tcg;; *) die "unknown option $1";; esac; shift; done
  [ "$CPUS" -le 2 ] || die "max 2 vCPU (development VM resource limit)"; [ "$MEM" -le 4096 ] || die "max 4096 MB (development VM resource limit)"
  [ -f "$file" ] || die "no such file: $file"
  if [ "$ACCEL" = kvm ]; then
    [ -w /dev/kvm ] || sudo -n chmod 0666 /dev/kvm 2>/dev/null || die "/dev/kvm not writable"
    ACCELOPTS="-enable-kvm -machine q35,accel=kvm -cpu host"
  else
    ACCELOPTS="-machine q35 -accel tcg,thread=multi,tb-size=256 -cpu max"
  fi
  # shellcheck disable=SC2086
  set -- -name "vela-$NAME" $ACCELOPTS -smp "$CPUS" -m "$MEM" \
    -device qemu-xhci -device usb-tablet -device usb-kbd \
    -netdev "user,id=n0,hostfwd=tcp:127.0.0.1:$SSHP-:22" -device virtio-net-pci,netdev=n0 \
    -audiodev none,id=snd0 -device intel-hda -device hda-output,audiodev=snd0 \
    -device virtio-rng-pci \
    -display none -vnc "127.0.0.1:$VNC" -qmp "unix:$QMP,server=on,wait=off" \
    -serial "file:$RUN/serial.log" $SNAP
  case $VGA in
    virtio) set -- "$@" -device virtio-vga,xres=1280,yres=800;;
    std) set -- "$@" -vga std;; vmware) set -- "$@" -vga vmware;; qxl) set -- "$@" -vga qxl;;
    *) die "unknown --vga $VGA";; esac
  if [ "$EFI" = 1 ]; then
    [ -f "$RUN/OVMF_VARS.fd" ] || cp /usr/share/OVMF/OVMF_VARS_4M.fd "$RUN/OVMF_VARS.fd"
    set -- "$@" -drive if=pflash,format=raw,readonly=on,file=/usr/share/OVMF/OVMF_CODE_4M.fd \
                -drive "if=pflash,format=raw,file=$RUN/OVMF_VARS.fd"
  fi
  if [ "$mode" = iso ]; then
    set -- "$@" -drive "file=$file,media=cdrom,if=none,id=cd0,readonly=on" -device ide-cd,drive=cd0,bootindex=1
    [ -n "$DISK" ] && set -- "$@" -drive "file=$DISK,if=virtio,format=qcow2,discard=unmap"
  else
    set -- "$@" -drive "file=$file,if=virtio,format=qcow2,discard=unmap,id=hd0"
    [ -n "$CDROM" ] && set -- "$@" -drive "file=$CDROM,media=cdrom,if=none,id=cd0,readonly=on" -device ide-cd,drive=cd0
  fi
  : > "$RUN/serial.log"; rm -f "$QMP" "$RUN/pid"
  echo "$SSHP" > "$RUN/sshport"; echo "$VNC" > "$RUN/vnc"
  echo "qemu-system-x86_64 $*" > "$RUN/cmdline"
  # Avoid qemu -daemonize (spawns a zombie parent on this host); start in background ourselves.
  set -- "$@" -pidfile "$RUN/pid"
  # Drop -daemonize if present
  ARGS=""; for a in "$@"; do [ "$a" = "-daemonize" ] && continue; ARGS="$ARGS $a"; done
  # shellcheck disable=SC2086
  nohup qemu-system-x86_64 $ARGS >"$RUN/qemu.log" 2>&1 &
  i=0; while [ ! -S "$QMP" ] && [ $i -lt 80 ]; do sleep 0.1; i=$((i+1)); done
  [ -S "$QMP" ] || { cat "$RUN/qemu.log" >&2; die "qemu failed to create QMP socket"; }
  # Prefer the pidfile, else the background shell's child
  [ -f "$RUN/pid" ] || echo $! > "$RUN/pid"
  sleep 1; running || { cat "$RUN/qemu.log" >&2; rm -f "$RUN/pid"; die "qemu exited at startup"; }
  echo "VM '$NAME' up: pid $(cat "$RUN/pid") VNC 127.0.0.1:$((5900+VNC)) SSH 127.0.0.1:$SSHP QMP $QMP serial $RUN/serial.log"
}

vssh() {
  p=$(sshport)
  if [ -f "$KEY" ] && ssh $SSHOPTS -o BatchMode=yes -i "$KEY" -p "$p" vela@127.0.0.1 true 2>/dev/null; then
    exec ssh $SSHOPTS -o BatchMode=yes -i "$KEY" -p "$p" vela@127.0.0.1 "$@"
  fi
  for pw in vela live; do
    if sshpass -p "$pw" ssh $SSHOPTS -p "$p" vela@127.0.0.1 true 2>/dev/null; then
      exec sshpass -p "$pw" ssh $SSHOPTS -p "$p" vela@127.0.0.1 "$@"
    fi
  done
  die "ssh to $NAME (port $p) failed"
}

cmd=${1:-}; [ -n "$cmd" ] || { sed -n 2,22p "$0"; exit 2; }; shift
case $cmd in
  boot-iso) boot iso "$@";;
  boot-disk) boot disk "$@";;
  screenshot) running || die "VM '$NAME' not running"; python3 "$TOOLS/qmp.py" "$QMP" screenshot "${1:?out.png}"; echo "${1}";;
  key) python3 "$TOOLS/qmp.py" "$QMP" key "$@";;
  type) python3 "$TOOLS/qmp.py" "$QMP" type "$@";;
  mouse) python3 "$TOOLS/qmp.py" "$QMP" mouse "$@";;
  ssh) vssh "$@";;
  scp-to|scp-from)
    p=$(sshport); src=$1; dst=$2
    if [ "$cmd" = scp-to ]; then a=$src; b="vela@127.0.0.1:$dst"; else a="vela@127.0.0.1:$src"; b=$dst; fi
    if [ -f "$KEY" ] && scp $SSHOPTS -o BatchMode=yes -i "$KEY" -P "$p" -r "$a" "$b" 2>/dev/null; then exit 0; fi
    for pw in vela live; do sshpass -p "$pw" scp $SSHOPTS -P "$p" -r "$a" "$b" 2>/dev/null && exit 0; done; die "scp failed";;
  wait-ssh)
    t=${1:-300}; end=$(( $(date +%s) + t )); p=$(sshport)
    while [ "$(date +%s)" -lt "$end" ]; do
      running || die "VM '$NAME' died"
      if { [ -f "$KEY" ] && ssh $SSHOPTS -o BatchMode=yes -i "$KEY" -p "$p" vela@127.0.0.1 true 2>/dev/null; } || \
         sshpass -p vela ssh $SSHOPTS -p "$p" vela@127.0.0.1 true 2>/dev/null || \
         sshpass -p live ssh $SSHOPTS -p "$p" vela@127.0.0.1 true 2>/dev/null; then echo "ssh ready on $p"; exit 0; fi
      sleep 3
    done; die "ssh not ready after ${t}s";;
  stop)
    running || { echo "not running"; exit 0; }
    t=${1:-90}; python3 "$TOOLS/qmp.py" "$QMP" powerdown 2>/dev/null || true
    i=0; while running && [ $i -lt "$t" ]; do sleep 1; i=$((i+1)); done
    if running; then python3 "$TOOLS/qmp.py" "$QMP" quit 2>/dev/null || kill "$(cat "$RUN/pid")"; sleep 1; fi
    rm -f "$RUN/pid"; echo "stopped $NAME";;
  kill) running && kill "$(cat "$RUN/pid")"; rm -f "$RUN/pid"; echo "killed $NAME";;
  status) if running; then echo "running pid $(cat "$RUN/pid") $(python3 "$TOOLS/qmp.py" "$QMP" status 2>/dev/null)"; else echo stopped; fi;;
  info) running && echo "pid $(cat "$RUN/pid")"; echo "qmp $QMP"; echo "vnc 127.0.0.1:$((5900+$(cat "$RUN/vnc" 2>/dev/null || echo $DVNC)))"; echo "ssh 127.0.0.1:$(sshport) user vela"; echo "serial $RUN/serial.log"; cat "$RUN/cmdline" 2>/dev/null;;
  qmp-path) echo "$QMP";;
  vnc) echo "127.0.0.1:$((5900+$(cat "$RUN/vnc" 2>/dev/null || echo $DVNC)))";;
  serial) cat "$RUN/serial.log";;
  *) die "unknown command $cmd";;
esac
