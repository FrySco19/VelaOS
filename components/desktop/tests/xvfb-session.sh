#!/bin/sh
# Local smoke test of the desktop component on the build box (no VM):
# runs kwin_x11 + plasmashell in Xvfb with a throw-away HOME that has the
# component overlay "installed" under ~/.local and ~/.config.
# Usage: xvfb-session.sh start|shot FILE|stop|eval JS
set -e
COMP=$(cd "$(dirname "$0")/.." && pwd)
T=${VELA_XTEST:-/tmp/vela-desktop-xtest}
DISP=${VELA_XDISP:-:91}
case "$1" in
start)
  rm -rf "$T"; mkdir -p "$T/home/.config" "$T/home/.local/share/plasma/look-and-feel/org.vela.test/contents/layouts" "$T/home/Scrivania"
  cp -r "$COMP/overlay/usr/share/plasma/layout-templates" "$T/home/.local/share/plasma/"
  [ -d "$COMP/overlay/usr/share/plasma/plasmoids" ] && cp -r "$COMP/overlay/usr/share/plasma/plasmoids" "$T/home/.local/share/plasma/"
  printf '{"KPlugin":{"Id":"org.vela.test","Name":"Vela test"},"KPackageStructure":"Plasma/LookAndFeel"}\n' > "$T/home/.local/share/plasma/look-and-feel/org.vela.test/metadata.json"
  echo 'loadTemplate("org.vela.desktop.defaultPanels")' > "$T/home/.local/share/plasma/look-and-feel/org.vela.test/contents/layouts/org.kde.plasma.desktop-layout.js"
  printf '[KDE]\nLookAndFeelPackage=org.vela.test\n' > "$T/home/.config/kdeglobals"
  cp -r "$COMP/overlay/etc/skel/." "$T/home/"
  printf 'XDG_DESKTOP_DIR="$HOME/Scrivania"\n' > "$T/home/.config/user-dirs.dirs"
  Xvfb $DISP -screen 0 1440x900x24 -nolisten tcp >"$T/xvfb.log" 2>&1 & echo $! > "$T/xvfb.pid"
  sleep 1
  env -i PATH=/usr/bin:/bin HOME="$T/home" DISPLAY=$DISP LANG=it_IT.UTF-8 XDG_RUNTIME_DIR="$T/run" \
    XDG_CURRENT_DESKTOP=KDE XDG_MENU_PREFIX=plasma- KDE_SESSION_VERSION=6 KDE_FULL_SESSION=true QT_QUICK_BACKEND=software KWIN_COMPOSE=N \
    sh -c 'mkdir -p -m700 $XDG_RUNTIME_DIR; exec dbus-run-session -- sh -c "echo \$DBUS_SESSION_BUS_ADDRESS > '"$T"'/bus; kwin_x11 >'"$T"'/kwin.log 2>&1 & sleep 2; plasmashell >'"$T"'/plasma.log 2>&1 & exec sleep infinity"' &
  echo $! > "$T/session.pid"
  sleep 12; echo started;;
shot) DISPLAY=$DISP import -window root "$2" && echo "saved $2";;
eval) env HOME="$T/home" DBUS_SESSION_BUS_ADDRESS=$(cat "$T/bus") qdbus6 org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript "$2";;
run) shift  # run an app inside the session
  env -i PATH=/usr/bin:/bin HOME="$T/home" DISPLAY=$DISP LANG=it_IT.UTF-8 XDG_RUNTIME_DIR="$T/run" DBUS_SESSION_BUS_ADDRESS="$(cat "$T/bus")" \
    XDG_CURRENT_DESKTOP=KDE XDG_MENU_PREFIX=plasma- KDE_SESSION_VERSION=6 KDE_FULL_SESSION=true "$@" >>"$T/apps.log" 2>&1 &
  echo "started $1";;
restart-plasma) # simulates the next login (re-reads all config)
  B=$(cat "$T/bus")
  env HOME="$T/home" DBUS_SESSION_BUS_ADDRESS=$B qdbus6 org.kde.plasmashell /MainApplication quit || true
  sleep 3
  env -i PATH=/usr/bin:/bin HOME="$T/home" DISPLAY=$DISP LANG=it_IT.UTF-8 XDG_RUNTIME_DIR="$T/run" DBUS_SESSION_BUS_ADDRESS=$B \
    XDG_CURRENT_DESKTOP=KDE XDG_MENU_PREFIX=plasma- KDE_SESSION_VERSION=6 KDE_FULL_SESSION=true QT_QUICK_BACKEND=software \
    plasmashell >"$T/plasma2.log" 2>&1 &
  sleep 12; echo restarted;;
stop)
  for f in session xvfb; do [ -f "$T/$f.pid" ] && kill "$(cat "$T/$f.pid")" 2>/dev/null || true; done
  [ -f "$T/bus" ] && env DBUS_SESSION_BUS_ADDRESS="$(cat "$T/bus")" qdbus6 org.kde.plasmashell /MainApplication quit >/dev/null 2>&1 || true
  sleep 1; echo stopped;;
esac
