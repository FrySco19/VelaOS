#!/bin/bash
# Regression tests for reported complaints R2 R3 R4 R5 R6 R7 (+ perf R1 latency). Needs QA_QMP + guest SSH.
. "$(dirname "$0")/../lib/qa.sh"
: "${QA_QMP:?set QA_QMP to the VM QMP socket}"
gwait_ssh 120 || { record FAIL ssh "guest SSH unreachable"; exit 1; }
read W H < <(python3 -c "
import sys; sys.path.insert(0,'$QA_LIB'); import qmp; s,f=qmp.connect('$QA_QMP'); print(*qmp.size(f))")
export QMP_W=$W QMP_H=$H
echo "screen ${W}x${H}"
judge_ocr() { # judge_ocr png ID label
  local en b; en=$(ocr_english "$1" | tr '\n' ' '); b=$(ocr_banned "$1" | head -3 | tr '\n' ' ')
  [ -n "$b" ] && record FAIL R7 "$3: banned/marketing copy on screen: $b" "$1"
  [ -n "$en" ] && record WARN P-IT-1 "$3: English words seen by OCR: $en" "$1" || true
}
closeall() { gsess 'for a in kwrite kate dolphin systemsettings plasma-discover konsole kcalc; do pkill -u $(id -u) -x $a 2>/dev/null; done' ; nap 1; }

# --- R3: right-click on empty desktop
closeall; key meta-d 2>/dev/null; nap 1
p0=$(shot r3-00-desktop)
click $((W/2)) $((H/2)) right; nap 0.8
p=$(shot r3-01-desktop-rightclick)
txt=$(ocr "$p")
if python3 - "$p0" "$p" <<'PY'
import sys
from PIL import Image, ImageChops
a,b=[Image.open(x).convert('L') for x in sys.argv[1:3]]
h=ImageChops.difference(a,b).histogram(); sys.exit(0 if sum(h[20:])/(a.size[0]*a.size[1])>0.005 else 1)
PY
then
  if echo "$txt" | grep -qiE 'nuov|incolla|sfondo|terminale|configura|impostazioni'; then record PASS R3 "desktop right-click shows Italian menu" "$p"
  else record FAIL R3 "desktop right-click changed screen but no Italian menu items recognised (OCR)" "$p"; fi
else record FAIL R3 "desktop right-click: nothing appears" "$p"; fi
judge_ocr "$p" R3 "desktop menu"; key esc

# --- R4a: Ctrl+C/Ctrl+V in a text editor (guest-internal, via real key events)
nonce="vela$RANDOM$RANDOM"
gsess "printf 'copiami $nonce\n' > /tmp/qa-src.txt; : > /tmp/qa-dst.txt; (setsid kwrite /tmp/qa-src.txt >/dev/null 2>&1 &)"; nap 4
shot r4-00-kwrite >/dev/null
key ctrl-a; nap 0.3; key ctrl-c; nap 0.5
clip=$(gsess 'timeout 3 wl-paste -n 2>/dev/null || timeout 3 xclip -o -sel clip 2>/dev/null')
echo "$clip" | grep -q "$nonce" && record PASS R4 "Ctrl+C in KWrite reaches system clipboard" || record FAIL R4 "Ctrl+C in KWrite: clipboard has '${clip:0:40}'" "$QA_OUT/r4-00-kwrite.png"
gsess "(setsid kwrite /tmp/qa-dst.txt >/dev/null 2>&1 &)"; nap 4
key ctrl-v; nap 0.5; key ctrl-s; nap 1.5
p=$(shot r4-01-kwrite-pasted)
gsess 'grep -q "'"$nonce"'" /tmp/qa-dst.txt' && record PASS R4 "Ctrl+V pastes into another app (KWrite→KWrite)" "$p" || record FAIL R4 "Ctrl+V into second KWrite did not paste/save" "$p"
closeall

# --- R4b: file copy/paste with Ctrl+C / Ctrl+V in the file manager
gsess 'rm -rf ~/qa-src ~/qa-dst; mkdir -p ~/qa-src ~/qa-dst; echo x > ~/qa-src/documento-qa.txt; (setsid dolphin ~/qa-src >/dev/null 2>&1 &)'; nap 4
key ctrl-a; nap 0.3; key ctrl-c; nap 0.3; gh=$(gssh 'echo $HOME'); key ctrl-l; nap 0.3; typ "$gh/qa-dst"; key ret; nap 1.5; key ctrl-v; nap 2
p=$(shot r4-02-dolphin-paste)
gsess 'test -f ~/qa-dst/documento-qa.txt' && record PASS R4 "file Ctrl+C/Ctrl+V in Dolphin" "$p" || record FAIL R4 "file Ctrl+C/Ctrl+V in Dolphin failed" "$p"
judge_ocr "$p" P-FM-1 "Dolphin"
closeall; gsess 'rm -rf ~/qa-src ~/qa-dst'

# --- R4c: terminal copy/paste
gsess "(setsid konsole >/dev/null 2>&1 &)"; nap 3
typ "echo $nonce-term"; key ret; nap 0.5
gsess 'printf "incollato-dal-sistema" | wl-copy 2>/dev/null || printf "incollato-dal-sistema" | xclip -sel clip'
key ctrl-shift-v; nap 0.5
p=$(shot r4-03-terminal-paste)
ocr "$p" | grep -q 'incollato' && record PASS R4 "Ctrl+Shift+V pastes in terminal" "$p" || record FAIL R4 "terminal paste not visible (OCR)" "$p"
key ctrl-c; closeall

# --- R5: settings are full
gsess 'kcmshell6 --list 2>/dev/null' > "$QA_OUT/kcm-list.txt"
n=$(grep -c . "$QA_OUT/kcm-list.txt")
miss=""
for k in kcm_networkmanagement kcm_bluetooth kcm_pulseaudio kcm_kscreen kcm_powerdevilprofilesconfig kcm_users kcm_clock kcm_regionandlang kcm_keyboard kcm_mouse kcm_touchpad kcm_notifications kcm_colors kcm_lookandfeel kcm_wallpaper kcm_access kcm_componentchooser kcm_about-distro kcm_printer_manager kcm_kwin_virtualdesktops kcm_keys kcm_fonts kcm_firewall; do
  grep -q "^$k\b" "$QA_OUT/kcm-list.txt" || miss="$miss $k"; done
[ -z "$miss" ] && record PASS R5 "$n settings modules, all required present" "$QA_OUT/kcm-list.txt" || record FAIL R5 "$n modules; missing:$miss" "$QA_OUT/kcm-list.txt"
grep -qiE 'vela' "$QA_OUT/kcm-list.txt" && record PASS R5 "Vela settings module(s) present" || record FAIL R5 "no Vela (IA/privacy) settings module in kcmshell6 --list" "$QA_OUT/kcm-list.txt"
gsess "(setsid systemsettings >/dev/null 2>&1 &)"
r=$(python3 "$QA_LIB/settle.py" "$QA_QMP" $((15*QA_SLOW)) 800); echo "settings latency $r" >> "$QA_OUT/latency.log"
p=$(shot r5-00-settings); judge_ocr "$p" R5 "Impostazioni"
record INFO R1 "systemsettings open: $r" "$p"; closeall

# --- R6 + R2: store and installed app visibility
gsess "(setsid plasma-discover >/dev/null 2>&1 &)"
r=$(python3 "$QA_LIB/settle.py" "$QA_QMP" $((20*QA_SLOW)) 1500); echo "discover latency $r" >> "$QA_OUT/latency.log"
nap 3; p=$(shot r6-00-store); judge_ocr "$p" R6 "Store"
ocr "$p" | grep -qiE 'install|applicazion|aggiorn|cerca' && record PASS R6 "store window shows content" "$p" || record FAIL R6 "store window empty/not recognised" "$p"
closeall
# install a small Debian GUI app non-interactively and check it shows in the launcher search without re-login
app_pkg=${QA_APP_PKG:-kcalc}; app_q=${QA_APP_QUERY:-kcalc}
gssh "sudo -n DEBIAN_FRONTEND=noninteractive apt-get install -y -q $app_pkg" > "$QA_OUT/r2-install.log" 2>&1 \
  && record PASS R2 "installed $app_pkg via apt (as Discover would)" "$QA_OUT/r2-install.log" || record SKIP R2 "could not install $app_pkg (network?)" "$QA_OUT/r2-install.log"
nap 5; key meta; nap 1.2; typ "$app_q"; nap 2
p=$(shot r2-01-launcher-search)
ocr "$p" | grep -qiE 'kcalc|calcolatric' && record PASS R2 "newly installed app found in launcher search" "$p" || record FAIL R2 "newly installed app NOT found in launcher search" "$p"
judge_ocr "$p" R2 "launcher"; key esc
# --- R7: whole-screen OCR of every screenshot taken
for f in "$QA_OUT"/*.png; do b=$(ocr_banned "$f" | head -2 | tr '\n' ' '); [ -n "$b" ] && record FAIL R7 "banned copy in $(basename "$f"): $b" "$f"; done
summary
