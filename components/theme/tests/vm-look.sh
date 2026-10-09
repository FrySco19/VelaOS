#!/bin/sh
# components/theme/tests/vm-look.sh — theme visual check in the dev VM (run on the box).
#   vm-look.sh reset            back up + clear user vela's Plasma/KWin config, restart the session (first-login look)
#   vm-look.sh mode light|dark  vela-theme mode in the user session
#   vm-look.sh shots TAG        screenshots: desktop, launcher, desktop menu, Dolphin, System Settings, Konsole → artifacts/theme/vm/TAG-*.png
#   vm-look.sh run CMD...       run a command inside the Plasma session (Wayland env)
# Needs: running dev VM (tools/devvm.sh start), theme deployed (tools/deploy.sh theme).
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
VM="$ROOT/tools/vm.sh -n dev"
OUT=/workspace/vela/artifacts/theme/vm; mkdir -p "$OUT"
ENV='export XDG_RUNTIME_DIR=/run/user/$(id -u) WAYLAND_DISPLAY=wayland-0 QT_QPA_PLATFORM=wayland DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus;'
srun() { $VM ssh "sh -c '$ENV $*'"; }
bg()   { $VM ssh "sh -c '$ENV nohup $* >/dev/null 2>&1 &'"; }
shot() { sleep "${2:-6}"; $VM screenshot "$OUT/$1.png" && echo "shot $OUT/$1.png"; }
case ${1:-} in
  reset)
    srun 'mkdir -p ~/.theme-backup && cd ~/.config && cp -a kdeglobals kwinrc plasmarc plasma-org.kde.plasma.desktop-appletsrc plasmashellrc kcminputrc ksplashrc ~/.theme-backup/ 2>/dev/null; rm -f kdeglobals kwinrc plasmarc plasma-org.kde.plasma.desktop-appletsrc plasmashellrc kcminputrc ksplashrc vela/appearance.json; rm -rf ~/.cache/plasma* ~/.cache/icon-cache.kcache ~/.cache/ksycoca6*; true'
    $VM ssh 'sudo systemctl restart sddm'; echo "session restarting (TCG: give it a few minutes)";;
  mode) srun "vela-theme mode $2";;
  run) shift; srun "$*";;
  shots)
    T=${2:?tag}
    shot "$T-desktop" 2
    bg dolphin; shot "$T-dolphin" 40
    srun 'qdbus6 org.kde.dolphin-$(pgrep -n dolphin) /dolphin/Dolphin_1 org.qtproject.Qt.QWidget.close' 2>/dev/null || srun 'pkill -n -x dolphin' || true
    bg systemsettings; shot "$T-settings" 60
    srun 'pkill -n -x systemsettings' || true
    bg konsole; shot "$T-konsole" 30
    srun 'pkill -n -x konsole' || true
    $VM mouse 960 540 rclick; shot "$T-desktopmenu" 8; $VM key esc
    $VM key meta_l; shot "$T-launcher" 15; $VM key esc;;
  *) sed -n 2,7p "$0"; exit 2;;
esac
