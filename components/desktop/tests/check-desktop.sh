#!/bin/sh
# Vela desktop regression check (component "desktop"). Run INSIDE the guest as the
# logged-in desktop user, e.g. from the box:
#   ssh -p 2222 vela@127.0.0.1 'sh -s' < components/desktop/tests/check-desktop.sh
# Prints one line per check: PASS|FAIL|SKIP <ID> <message>; exit code = number of FAILs.
# Side effects: creates and removes a test app ~/.local/share/applications/vela-qa-*.desktop
# and puts a test string on the clipboard.
UID_=$(id -u)
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$UID_}
export DBUS_SESSION_BUS_ADDRESS=${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}
[ -z "$WAYLAND_DISPLAY" ] && [ -S "$XDG_RUNTIME_DIR/wayland-0" ] && export WAYLAND_DISPLAY=wayland-0
[ -z "$DISPLAY" ] && export DISPLAY=:0
export XDG_MENU_PREFIX=plasma- XDG_CURRENT_DESKTOP=KDE
FAILS=0
ok()   { echo "PASS $1 $2"; }
ko()   { echo "FAIL $1 $2"; FAILS=$((FAILS+1)); }
skip() { echo "SKIP $1 $2"; }
chk()  { id=$1; msg=$2; shift 2; if "$@" >/dev/null 2>&1; then ok "$id" "$msg"; else ko "$id" "$msg"; fi; }
rd()   { f=$1; shift; kreadconfig6 --file "$f" "$@" 2>/dev/null; }
APPLETS="$HOME/.config/plasma-org.kde.plasma.desktop-appletsrc"
DD=$(xdg-user-dir DESKTOP 2>/dev/null || echo "$HOME/Desktop")

# ---- D1 layout (top bar + dock) ----
if [ -f "$APPLETS" ]; then
  for w in org.kde.plasma.kickoff org.vela.appname org.kde.plasma.appmenu org.kde.plasma.systemtray org.kde.plasma.digitalclock org.kde.plasma.icontasks org.kde.plasma.trash org.kde.plasma.folder; do
    chk D1-$w "widget $w present in layout" grep -q "plugin=$w\$" "$APPLETS"
  done
  python3 - "$APPLETS" <<'PY'
import re, sys
txt = open(sys.argv[1]).read()
cont = {}
for m in re.finditer(r'^\[Containments\]\[(\d+)\]\n((?:[^\[\n].*\n?)*)', txt, re.M):
    cont[m.group(1)] = dict(l.split('=', 1) for l in m.group(2).splitlines() if '=' in l)
top = [c for c, v in cont.items() if v.get('plugin') == 'org.kde.panel' and v.get('location') == '3']
bot = [c for c, v in cont.items() if v.get('plugin') == 'org.kde.panel' and v.get('location') == '4']
def applets(c):
    return re.findall(r'^\[Containments\]\[%s\]\[Applets\]\[\d+\]\n(?:[^\[\n].*\n)*?plugin=(.*)$' % c, txt, re.M)
print(("PASS" if top else "FAIL") + " D1-top top bar exists")
print(("PASS" if bot else "FAIL") + " D1-dock dock exists at the bottom")
if top:
    a = applets(top[0]); print(("PASS" if 'org.kde.plasma.appmenu' in a and 'org.kde.plasma.digitalclock' in a else "FAIL") + " D1-topitems top bar has global menu + clock: " + ",".join(a))
if bot:
    a = applets(bot[0]); print(("PASS" if a and a[-1] == 'org.kde.plasma.trash' and 'org.kde.plasma.icontasks' in a else "FAIL") + " D1-dockitems dock = apps ... Cestino: " + ",".join(a))
m = re.search(r'^launchers=(.*)$', txt, re.M)
print(("PASS" if m and m.group(1) else "FAIL") + " D1-pinned dock pinned apps: " + (m.group(1) if m else "none"))
PY
else
  ko D1 "no $APPLETS (Plasma never started for this user?)"
fi

# ---- D2 right-click on desktop ----
if [ -f "$APPLETS" ]; then
  chk D2-terminal "desktop menu has 'Apri terminale'" grep -q '^_open_terminal=true' "$APPLETS"
  chk D2-display "desktop menu has display settings" grep -q '^_display_settings=true' "$APPLETS"
  chk D2-rmb "right button bound to context menu" grep -q '^RightButton;NoModifier=org.kde.contextmenu' "$APPLETS"
fi

# ---- D3 desktop icons + new-app watcher ----
chk D3-units "watchers active (apps, drives)" systemctl --user is-active -q vela-desktop-apps.path vela-desktop-drives.path
chk D3-home "Home icon on desktop" test -f "$DD/vela-home.desktop"
chk D3-trash "Cestino icon on desktop" test -f "$DD/vela-trash.desktop"
N=vela-qa-$$; F="$HOME/.local/share/applications/$N.desktop"
mkdir -p "$HOME/.local/share/applications"
printf '[Desktop Entry]\nType=Application\nName=Prova QA\nExec=true\nIcon=applications-other\n' > "$F"
i=0; while [ $i -lt 20 ] && [ ! -f "$DD/$N.desktop" ]; do sleep 1; i=$((i+1)); done
APPEARED=0
if [ -f "$DD/$N.desktop" ]; then APPEARED=1; ok D3-newapp "new app got a desktop icon in ${i}s"; else
  if [ "$(vela-desktop-settings --get | grep AddToDesktop)" = "NewApps.AddToDesktop=false" ]; then skip D3-newapp "disabled in settings"; else ko D3-newapp "no desktop icon after 20s"; fi; fi
rm -f "$F"
i=0; while [ $i -lt 20 ] && [ -f "$DD/$N.desktop" ]; do sleep 1; i=$((i+1)); done
if [ $APPEARED = 1 ]; then chk D3-uninstall "icon removed when app is uninstalled" test ! -f "$DD/$N.desktop"; else skip D3-uninstall "no icon was created"; fi

# ---- D4 clipboard ----
NONCE="vela-clip-$$-$(date +%s)"
if [ -n "$WAYLAND_DISPLAY" ] && command -v wl-copy >/dev/null; then
  printf %s "$NONCE" | wl-copy >/dev/null 2>&1; sleep 1; got=$(wl-paste -n 2>/dev/null)
elif command -v xclip >/dev/null; then
  printf %s "$NONCE" | xclip -selection clipboard >/dev/null 2>&1; sleep 1; got=$(xclip -o -selection clipboard 2>/dev/null)
fi
[ "$got" = "$NONCE" ] && ok D4-roundtrip "clipboard copy/paste round trip" || ko D4-roundtrip "clipboard round trip got '$got'"
hist=$(qdbus6 org.kde.klipper /klipper org.kde.klipper.klipper.getClipboardHistoryItem 0 2>/dev/null)
[ "$hist" = "$NONCE" ] && ok D4-history "Klipper history (Meta+V) has the copied text" || ko D4-history "Klipper history item 0 = '$hist'"

# ---- D5 shortcuts / KWin behaviour (effective config incl. /etc/xdg) ----
[ "$(rd kwinrc --group Effect-overview --key BorderActivate)" = 7 ] && ok D5-hotcorner "top-left hot corner = Panoramica" || ko D5-hotcorner "hot corner not set"
rd kwinrc --group ModifierOnlyShortcuts --key Meta | grep -q Overview && ok D5-meta "Meta = Panoramica" || ko D5-meta "Meta not bound to Overview"
[ "$(rd kwinrc --group TabBox --key LayoutName)" = thumbnail_grid ] && ok D5-alttab "Alt+Tab with thumbnails" || ko D5-alttab "Alt+Tab layout"
[ "$(rd kwinrc --group Windows --key ElectricBorderTiling)" = true ] && ok D5-snap "drag-to-edge snapping" || ko D5-snap "snapping"
rd kglobalshortcutsrc --group services --group org.kde.krunner.desktop --key _launch | grep -q 'Meta+Space' && ok D5-search "Meta+Space opens search" || ko D5-search "Meta+Space"
rd kglobalshortcutsrc --group services --group org.kde.spectacle.desktop --key RectangularRegionScreenShot | grep -q 'Meta+Shift+S' && ok D5-shot "Meta+Shift+S = area screenshot" || ko D5-shot "Meta+Shift+S"
n=$(qdbus6 org.kde.KWin /VirtualDesktopManager org.freedesktop.DBus.Properties.Get org.kde.KWin.VirtualDesktopManager count 2>/dev/null)
[ -n "$n" ] && [ "$n" -ge 1 ] && ok D5-spaces "virtual desktops (Spazi): $n" || skip D5-spaces "KWin D-Bus not reachable"
if qdbus6 org.kde.kglobalaccel >/dev/null 2>&1; then
  qdbus6 org.kde.kglobalaccel /component/org_kde_krunner_desktop org.kde.kglobalaccel.Component.shortcutNames >/dev/null 2>&1 && ok D5-kglobalaccel "kglobalaccel knows KRunner" || skip D5-kglobalaccel "krunner component not registered yet"
fi

# ---- D6 Dolphin like Finder ----
grep -qs 'show_information_panel" shortcut="Space' "$HOME/.local/share/kxmlgui5/dolphin/dolphinui.rc" && ok D6-space "Space = quick preview in File" || ko D6-space "Space shortcut missing"
[ "$(rd kdeglobals --group KDE --key SingleClick)" = false ] && ok D6-click "double-click opens" || ko D6-click "SingleClick not false"
[ "$(rd dolphinrc --group General --key ShowFullPath)" = false ] && ok D6-breadcrumb "breadcrumb path bar" || ko D6-breadcrumb "breadcrumb"
chk D6-ark "compress/extract menu (Ark)" test -f /usr/lib/x86_64-linux-gnu/qt6/plugins/kf6/kfileitemaction/compressfileitemaction.so

# ---- D7 default apps ----
for pair in inode/directory=org.kde.dolphin.desktop x-scheme-handler/https=firefox-esr.desktop text/plain=org.kde.kwrite.desktop \
  image/png=org.kde.gwenview.desktop application/pdf=org.kde.okular.desktop video/mp4=org.kde.haruna.desktop \
  audio/mpeg=org.kde.elisa.desktop application/zip=org.kde.ark.desktop \
  application/vnd.openxmlformats-officedocument.wordprocessingml.document=libreoffice-writer.desktop; do
  mt=${pair%%=*}; want=${pair#*=}
  got=$(xdg-mime query default "$mt" 2>/dev/null)
  if [ ! -f "/usr/share/applications/$want" ]; then ko D7-$mt "default app $want not installed"
  elif [ "$got" = "$want" ]; then ok D7-$mt "opens with $want"
  else ko D7-$mt "xdg-mime says '$got', expected $want"; fi
done

echo "SUMMARY desktop checks: $FAILS failed"
exit $FAILS
