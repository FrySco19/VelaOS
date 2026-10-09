#!/bin/sh
# Konsole smoke test on a private Xvfb display with an isolated HOME, using the overlay files:
# Vela profile is the default, colour scheme loads, fish + starship prompt, Ctrl+V and
# Ctrl+Shift+V paste, Ctrl+C interrupts (does not copy). Screenshots to the artifacts dir.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
COMP=$(dirname "$HERE")
OV="$COMP/overlay"
ART=${VT_ARTIFACTS:-/workspace/vela/artifacts/terminal}
mkdir -p "$ART"
N=170
while [ -e /tmp/.X11-unix/X$N ] || [ -e /tmp/.X$N-lock ]; do N=$((N+1)); done
Xvfb :$N -screen 0 1280x800x24 -nolisten tcp >/tmp/vt-kxvfb-$N.log 2>&1 &
XPID=$!
T=$(mktemp -d /tmp/vt-konsole.XXXXXX)
KPID=""
cleanup() { [ -n "$KPID" ] && kill "$KPID" 2>/dev/null; kill $XPID 2>/dev/null; }
trap cleanup EXIT
sleep 1
[ -e /tmp/.X11-unix/X$N ] || { echo "Xvfb failed"; exit 1; }
mkdir -p "$T/run" "$T/etc/xdg" "$T/.config/fish/conf.d" && chmod 700 "$T/run"
cp -r "$OV/etc/skel/." "$T/"
VELA_ROOT="$T" sh "$COMP/hook.chroot.sh" >/dev/null 2>&1 || true   # writes $T/etc/xdg/konsolerc
cp "$OV/etc/fish/conf.d/vela.fish" "$T/.config/fish/conf.d/"
export DISPLAY=:$N HOME="$T" XDG_CONFIG_HOME="$T/.config" XDG_DATA_HOME="$T/.local/share" \
    XDG_RUNTIME_DIR="$T/run" XDG_CONFIG_DIRS="$T/etc/xdg" XDG_DATA_DIRS="$OV/usr/share:/usr/share" \
    LANG=it_IT.UTF-8 QT_QPA_PLATFORM=xcb SHELL=/usr/bin/fish STARSHIP_CONFIG="$OV/usr/share/vela/terminal/starship.toml" \
    KDE_FULL_SESSION= XDG_CURRENT_DESKTOP=KDE
FAIL=0
ok() { echo "PASS $1"; }
ko() { echo "FAIL $1"; FAIL=1; }
eval "$(dbus-launch --sh-syntax)"
mkdir -p "$T/work" && cd "$T/work"
timeout 120 prlimit --as=4000000000 konsole --separate --nofork >"$T/konsole.log" 2>&1 &
KPID=$!
SVC=""
for i in $(seq 1 60); do
  SVC=$(qdbus6 2>/dev/null | grep -oE 'org\.kde\.konsole-[0-9]+' | head -1)
  [ -n "$SVC" ] && qdbus6 "$SVC" /Sessions/1 >/dev/null 2>&1 && break; sleep 0.5
done
echo "service: ${SVC:-none}"; [ -n "$SVC" ] || { tail -20 "$T/konsole.log"; }
sleep 3
q() { qdbus6 "$SVC" "$@" 2>/dev/null; }
prof=$(q /Sessions/1 org.kde.konsole.Session.profile)
[ "$prof" = "Vela chiaro" ] && ok "default profile = $prof" || ko "default profile = '$prof'"
KP=${SVC#org.kde.konsole-}; WID=$(xdotool search --pid "$KP" --onlyvisible 2>/dev/null | head -1)
[ -n "$WID" ] && xdotool windowactivate --sync "$WID" 2>/dev/null; xdotool windowsize "$WID" 1000 600 2>/dev/null
sleep 1
txt() { q /Sessions/1 org.kde.konsole.Session.getAllDisplayedText false 2>/dev/null || q /Sessions/1 org.kde.konsole.Session.getAllDisplayedText; }
txt > "$T/screen0.txt"
grep -q "❯" "$T/screen0.txt" && ok "starship prompt shown" || ko "starship prompt not found: $(tr -s '\n' ' ' < "$T/screen0.txt" | cut -c1-200)"
import -window root "$ART/konsole-01-light.png" 2>/dev/null
bgcheck() {  # file r,g,b : the colour most present in the terminal area must be the scheme background
python3 - "$1" "$2" <<'PY'
import sys, collections
from PIL import Image
im = Image.open(sys.argv[1]).convert("RGB")
want = tuple(int(v) for v in sys.argv[2].split(","))
c = collections.Counter(im.getpixel((x, y)) for x in range(20, 760, 7) for y in range(60, 460, 7))
top = c.most_common(1)[0][0]
sys.exit(0 if top == want else (print(top) or 1))
PY
}
bgcheck "$ART/konsole-01-light.png" 251,252,252 && ok "light scheme background #FBFCFC" || ko "light scheme background"
# Ctrl+V paste
printf 'echo INCOLLA_CTRL_V' | xclip -selection clipboard -i
xdotool key --clearmodifiers ctrl+v; sleep 0.5; xdotool key Return; sleep 1
txt > "$T/screen1.txt"
[ "$(grep -c 'INCOLLA_CTRL_V' "$T/screen1.txt")" -ge 2 ] && ok "Ctrl+V pastes" || ko "Ctrl+V paste"
printf 'echo INCOLLA_CTRL_SHIFT_V' | xclip -selection clipboard -i
xdotool key --clearmodifiers ctrl+shift+v; sleep 0.5; xdotool key Return; sleep 1
txt > "$T/screen2.txt"
[ "$(grep -c 'INCOLLA_CTRL_SHIFT_V' "$T/screen2.txt")" -ge 2 ] && ok "Ctrl+Shift+V pastes" || ko "Ctrl+Shift+V paste"
# Ctrl+C interrupts a running command
xdotool type --delay 20 'sleep 30; echo NON_INTERROTTO'; xdotool key Return; sleep 1
xdotool key --clearmodifiers ctrl+c; sleep 1
xdotool type --delay 20 'echo DOPO_CTRL_C'; xdotool key Return; sleep 1.5
txt > "$T/screen3.txt"
if grep -q '^DOPO_CTRL_C' "$T/screen3.txt" && ! grep -q '^NON_INTERROTTO' "$T/screen3.txt"; then ok "Ctrl+C interrupts"; else ko "Ctrl+C interrupt"; fi
# Ctrl+Shift+C copies the selection
q /Sessions/1 org.kde.konsole.Session.sendText "clear" >/dev/null; xdotool key Return; sleep 0.5
printf 'vuoto' | xclip -selection clipboard -i
xdotool type --delay 20 'echo SELEZIONE_DA_COPIARE'; xdotool key Return; sleep 1
# drag-select a large area of the terminal (mouse is absolute in the window)
xdotool mousemove --window "$WID" 30 60 mousedown 1 mousemove --window "$WID" 600 400 sleep 0.2 mousemove --window "$WID" 900 560 mouseup 1
sleep 0.3
[ "$(xclip -selection clipboard -o 2>/dev/null)" = "vuoto" ] && ok "selecting does not overwrite the clipboard" || ko "copy-on-select is on"
[ -n "$(xclip -selection primary -o 2>/dev/null)" ] && ok "selection goes to primary (middle-click paste)" || ko "primary selection empty"
xdotool key --clearmodifiers ctrl+shift+c; sleep 0.5
clip=$(xclip -selection clipboard -o 2>/dev/null)
[ "$clip" != "vuoto" ] && ok "Ctrl+Shift+C copies (clipboard: $(printf %s "$clip" | tr -s '\n' ' ' | cut -c1-40))" || ko "Ctrl+Shift+C copy"
# unknown command message in Italian
xdotool type --delay 20 'comandoinesistente'; xdotool key Return; sleep 1.5
txt > "$T/screen4.txt"
grep -q 'comandoinesistente: comando non trovato' "$T/screen4.txt" && ok "Italian command-not-found" || ko "Italian command-not-found"
# switch to dark live via vela-terminal-theme
PATH="$OV/usr/bin:$PATH" sh "$OV/usr/bin/vela-terminal-theme" dark
sleep 1
prof=$(q /Sessions/1 org.kde.konsole.Session.profile)
[ "$prof" = "Vela scuro" ] && ok "vela-terminal-theme dark switches open session" || ko "theme switch: '$prof'"
def=$(kreadconfig6 --file konsolerc --group "Desktop Entry" --key DefaultProfile)
[ "$def" = "VelaDark.profile" ] && ok "default profile now VelaDark" || ko "default profile '$def'"
xdotool type --delay 20 'ls -la --color=always /etc | head -12'; xdotool key Return; sleep 1.5
import -window root "$ART/konsole-02-dark.png" 2>/dev/null
bgcheck "$ART/konsole-02-dark.png" 19,34,42 && ok "dark scheme background #13222A" || ko "dark scheme background"
kill "$DBUS_SESSION_BUS_PID" 2>/dev/null
cp "$T/konsole.log" "$ART/konsole-smoke.log" 2>/dev/null
rm -rf "$T"
[ $FAIL = 0 ] && echo "ALL PASS" || echo "SOME FAILED"
exit $FAIL
