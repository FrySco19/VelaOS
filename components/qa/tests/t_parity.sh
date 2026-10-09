#!/bin/bash
# macOS/Windows parity checks that need visual judgment: produces screenshots + automatic hints. Needs QA_QMP + SSH.
. "$(dirname "$0")/../lib/qa.sh"
: "${QA_QMP:?set QA_QMP}"
gwait_ssh 120 || { record FAIL ssh "guest SSH unreachable"; exit 1; }
read W H < <(python3 -c "
import sys; sys.path.insert(0,'$QA_LIB'); import qmp; s,f=qmp.connect('$QA_QMP'); print(*qmp.size(f))"); export QMP_W=$W QMP_H=$H
closeall() { gsess 'for a in kwrite dolphin systemsettings plasma-discover konsole spectacle; do pkill -u $(id -u) -x $a 2>/dev/null; done'; nap 1; }
seen() { ocr "$1" | grep -qiE "$2"; }
step() { # step ID name "keys..." regex_expected   (keys separated by space; 'sleep:N' allowed)
  local id=$1 name=$2 seq=$3 rx=$4 k
  for k in $seq; do case $k in sleep:*) nap "${k#sleep:}";; *) key "$k"; nap 0.4;; esac; done
  nap 1; local p; p=$(shot "$name")
  if [ -n "$rx" ]; then seen "$p" "$rx" && record PASS "$id" "$name: expected UI text found" "$p" || record FAIL "$id" "$name: expected /$rx/ not seen (OCR) — check screenshot" "$p"
  else record INFO "$id" "$name: screenshot for visual review" "$p"; fi
  local en; en=$(ocr_english "$p" | tr '\n' ' '); [ -n "$en" ] && record WARN P-IT-1 "$name: English words by OCR: $en" "$p"
}
closeall; key meta-d; nap 1
step V-1      p-00-desktop        ""                 ""
step P-DESK-3 p-01-launcher       "meta sleep:1"      "cerca|applicazion|preferit|tutte"
key esc
step P-SEARCH-1 p-02-search-calc  "alt-spc sleep:1"   ""
typ "12*7"; nap 1.5; p=$(shot p-02b-search-calc); seen "$p" '84' && record PASS P-SEARCH-1 "search calculator 12*7=84" "$p" || record FAIL P-SEARCH-1 "search calculator result not shown" "$p"
key esc; nap 0.3
key alt-spc; nap 1; typ "impostazioni"; nap 1.5; p=$(shot p-02c-search-settings); seen "$p" 'impostazion' && record PASS P-SEARCH-1 "search finds 'Impostazioni'" "$p" || record FAIL P-SEARCH-1 "search 'impostazioni' finds nothing" "$p"; key esc
gsess 'notify-send -a "Vela QA" "Prova notifica" "Questo è un test automatico"'; nap 1.5
p=$(shot p-03-notification); seen "$p" 'prova notifica' && record PASS P-NOTIF-1 "notification popup visible" "$p" || record FAIL P-NOTIF-1 "notification popup not seen" "$p"
gsess "(setsid dolphin >/dev/null 2>&1 &)"; nap 3; gsess "(setsid konsole >/dev/null 2>&1 &)"; nap 3
step P-MT-3   p-04-alttab        "" ""
python3 "$QA_LIB/qmp.py" "$QA_QMP" raw '{"execute":"input-send-event","arguments":{"events":[{"type":"key","data":{"down":true,"key":{"type":"qcode","data":"alt"}}},{"type":"key","data":{"down":true,"key":{"type":"qcode","data":"tab"}}}]}}' >/dev/null
nap 1; p=$(shot p-04b-alttab-held); record INFO P-MT-3 "Alt+Tab held: review switcher" "$p"
python3 "$QA_LIB/qmp.py" "$QA_QMP" raw '{"execute":"input-send-event","arguments":{"events":[{"type":"key","data":{"down":false,"key":{"type":"qcode","data":"tab"}}},{"type":"key","data":{"down":false,"key":{"type":"qcode","data":"alt"}}}]}}' >/dev/null
step P-MT-2   p-05-overview      "meta-w sleep:1.5" ""
key esc
step P-MT-1   p-06-snap-left     "meta-left sleep:1" ""
step P-MT-1   p-07-maximize      "meta-up sleep:1" ""
closeall
step P-SHOT-1 p-08-screenshot-tool "meta-shift-s sleep:2" ""
key esc; nap 0.5
gsess 'ls -t ~/Immagini/Schermate ~/Immagini ~/Pictures 2>/dev/null | head -3' > "$QA_OUT/shots-dir.txt"
step P-TERM-1 p-09-terminal      "ctrl-alt-t sleep:2" ""
gsess 'pgrep -u $(id -u) -x konsole >/dev/null || pgrep -u $(id -u) -f vela-terminal >/dev/null' && record PASS P-TERM-1 "Ctrl+Alt+T opens a terminal" || record FAIL P-TERM-1 "Ctrl+Alt+T did not start a terminal"
closeall
step P-FM-1   p-10-files         "meta-e sleep:2.5" "document|scaricat|immagin|musica|video|cestino"
closeall
gsess "(setsid systemsettings >/dev/null 2>&1 &)"; nap 4; p=$(shot p-11-settings-home); record INFO R5 "settings home for review" "$p"; closeall
step P-LOGIN-2 p-12-lockscreen   "meta-l sleep:2.5" ""
typ "$QA_SSH_PASS"; key ret; nap 2; p=$(shot p-12b-unlocked); record INFO P-LOGIN-2 "after unlock (review)" "$p"
# Power menu: open only, never confirm here
step P-POWER-1 p-13-power-menu   "" ""
summary
