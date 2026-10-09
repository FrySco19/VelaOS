#!/bin/bash
# Vela QA helpers — source me:  . components/qa/lib/qa.sh
# Env (override as needed; defaults follow tools/vm.sh conventions announced by base component):
#   QA_BUILD   build name (evidence dir)          default: devvm-$(date +%Y%m%d-%H%M)
#   QA_QMP     QMP unix socket of the VM under test (if unset, falls back to tools/vm.sh screenshot/key/type)
#   QA_SSH_PORT / QA_SSH_USER / QA_SSH_PASS / QA_SSH_KEY   guest SSH (default 2222 / vela / vela / none)
QA_REPO=${QA_REPO:-/workspace/vela/v2}
QA_LIB=$QA_REPO/components/qa/lib
QA_BUILD=${QA_BUILD:-devvm-$(date +%Y%m%d-%H%M)}
QA_OUT=${QA_OUT:-/workspace/vela/artifacts/qa/$QA_BUILD}
# QA_TARGET=dev (installed dev VM, default) | iso (ISO test VM) — sets SSH/QMP defaults from status/base.md
QA_TARGET=${QA_TARGET:-dev}
if [ "$QA_TARGET" = iso ]; then
  QA_SSH_PORT=${QA_SSH_PORT:-2223}; QA_SSH_PASS=${QA_SSH_PASS:-live}; QA_QMP=${QA_QMP:-/workspace/vela/run/iso/qmp.sock}
else
  QA_SSH_PORT=${QA_SSH_PORT:-2222}; QA_SSH_PASS=${QA_SSH_PASS:-vela}; QA_QMP=${QA_QMP:-/workspace/vela/run/dev/qmp.sock}
  [ -z "${QA_SSH_KEY:-}" ] && [ -r /workspace/vela/artifacts/base/ssh/id_ed25519 ] && QA_SSH_KEY=/workspace/vela/artifacts/base/ssh/id_ed25519
fi
QA_SSH_USER=${QA_SSH_USER:-vela}
mkdir -p "$QA_OUT"
QA_RESULTS=$QA_OUT/results.tsv

record() { # record PASS|FAIL|SKIP|WARN ID message [evidence]
  printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "${4:-}" >> "$QA_RESULTS"
  printf '%-5s %-12s %s %s\n' "$1" "$2" "$3" "${4:+[$4]}"
}

vmtool() { "$QA_REPO/tools/vm.sh" "$@"; }

QA_VNCDO=${QA_VNCDO:-/workspace/vela/artifacts/qa/.venv/bin/vncdo}   # fallback when QMP is busy (QMP is single-client)
_vmname=$([ "$QA_TARGET" = iso ] && echo iso || echo dev)
# vm.sh may shift the VNC display (e.g. :12 when :2 is taken by the box desktop) — read the real one
QA_VNC=${QA_VNC:-127.0.0.1::$((5900 + $(cat /workspace/vela/run/$_vmname/vnc 2>/dev/null || echo $([ "$_vmname" = iso ] && echo 1 || echo 2))))}
shot() { # shot name  -> $QA_OUT/name.png ; echoes path  (QMP, falls back to VNC capture)
  local p="$QA_OUT/$1.png"
  { [ -n "${QA_QMP:-}" ] && timeout 12 python3 "$QA_LIB/qmp.py" "$QA_QMP" screenshot "$p"; } >/dev/null 2>&1 \
    || timeout 20 "$QA_VNCDO" -s "$QA_VNC" capture "$p" >/dev/null 2>&1
  echo "$p"
}
key()   { if [ -n "${QA_QMP:-}" ]; then python3 "$QA_LIB/qmp.py" "$QA_QMP" key "$1"; else vmtool key "$1"; fi; }
typ()   { if [ -n "${QA_QMP:-}" ]; then python3 "$QA_LIB/qmp.py" "$QA_QMP" type "$1"; else vmtool type "$1"; fi; }
click() { python3 "$QA_LIB/qmp.py" "${QA_QMP:?QA_QMP needed for mouse}" click "$@"; }
dblclick() { python3 "$QA_LIB/qmp.py" "${QA_QMP:?QA_QMP needed for mouse}" dblclick "$@"; }
drag()  { python3 "$QA_LIB/qmp.py" "${QA_QMP:?QA_QMP needed for mouse}" drag "$@"; }
mmove() { python3 "$QA_LIB/qmp.py" "${QA_QMP:?QA_QMP needed for mouse}" move "$@"; }

_sshopts="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ConnectTimeout=8 -p $QA_SSH_PORT"
gssh() { # run a POSIX sh command in the guest (login shell may be fish: we always feed `sh -s` via stdin)
  local c="$*"
  if [ -n "${QA_SSH_KEY:-}" ]; then printf '%s\n' "$c" | ssh $_sshopts -i "$QA_SSH_KEY" "$QA_SSH_USER@127.0.0.1" 'sh -s'
  else printf '%s\n' "$c" | sshpass -p "$QA_SSH_PASS" ssh $_sshopts "$QA_SSH_USER@127.0.0.1" 'sh -s'; fi
}
gsess() { # run a command inside the user's graphical session environment (Wayland/X11 + session D-Bus)
  gssh 'U=$(id -u); export XDG_RUNTIME_DIR=/run/user/$U DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$U/bus;
        W=$(ls /run/user/$U 2>/dev/null | grep -m1 "^wayland-[0-9]$"); [ -n "$W" ] && export WAYLAND_DISPLAY=$W QT_QPA_PLATFORM=wayland;
        export DISPLAY=${DISPLAY:-:0}; '"$1"
}
gwait_ssh() { local t=${1:-180}; for i in $(seq "$t"); do gssh true 2>/dev/null && return 0; sleep 1; done; return 1; }

ocr() { # ocr png -> text (it+en)
  tesseract "$1" - -l ita+eng --psm 11 2>/dev/null
}
# English words that should not appear in an Italian UI (whole word, case-insensitive). Brand names excluded.
QA_EN_RX='\b(Settings|Search|Open|Close|Cancel|Save|Delete|Loading|Welcome|Click|Please|Error|Next|Back|Skip|Enable|Disable|Install|Remove|Update|Apply|Desktop Folder|Wallpaper|Configure|Show|Hide|Add|Shut ?down|Restart|Log ?out|Sleep|Lock|Files|Home|Trash|Applications|Favorites|Recent|Places|Devices|Network|Volume|Brightness|Notifications|Do not disturb|New Folder|Paste|Copy|Cut|Rename|Properties|Untitled|Get Started|Learn more|Try again|Something went wrong|Lorem)\b'
ocr_english() { # ocr_english png -> prints suspicious English words found
  ocr "$1" | grep -oiwE "$QA_EN_RX" | sort | uniq -c | sort -rn
}
ocr_banned() { ocr "$1" | grep -iE -f <(grep -v '^#' "$QA_LIB/banned_phrases.txt" | sed '/^$/d') ; }

summary() {
  local p f s w
  p=$(grep -c '^PASS' "$QA_RESULTS" 2>/dev/null || true); f=$(grep -c '^FAIL' "$QA_RESULTS" 2>/dev/null || true)
  s=$(grep -c '^SKIP' "$QA_RESULTS" 2>/dev/null || true); w=$(grep -c '^WARN' "$QA_RESULTS" 2>/dev/null || true)
  echo "== $QA_BUILD: PASS $p  FAIL $f  SKIP $s  WARN $w  ($QA_RESULTS)"
}

# Slowness factor: TCG (no KVM) guests are ~4-8x slower. Auto-detect from the VM cmdline.
if [ -z "${QA_SLOW:-}" ]; then
  if grep -qs 'accel=tcg\|-accel tcg' "/workspace/vela/run/$([ "$QA_TARGET" = iso ] && echo iso || echo dev)/cmdline"; then QA_SLOW=5; else QA_SLOW=1; fi
fi
nap() { sleep "$(awk -v a="$1" -v b="$QA_SLOW" 'BEGIN{print a*b}')"; }
