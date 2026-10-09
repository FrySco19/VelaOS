#!/bin/sh
# Runs the GUI smoke test on a private Xvfb display with an isolated HOME.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
N=150
while [ -e /tmp/.X11-unix/X$N ] || [ -e /tmp/.X$N-lock ]; do N=$((N+1)); done
Xvfb :$N -screen 0 1400x900x24 -nolisten tcp >/tmp/vt-xvfb-$N.log 2>&1 &
XPID=$!
trap 'kill $XPID 2>/dev/null' EXIT
sleep 1
[ -e /tmp/.X11-unix/X$N ] || { echo "Xvfb failed"; exit 1; }
T=$(mktemp -d /tmp/vt-gui.XXXXXX)
mkdir -p "$T/run" && chmod 700 "$T/run"
env -i PATH=/usr/bin:/bin DISPLAY=:$N HOME="$T" XDG_CONFIG_HOME="$T/.config" XDG_DATA_HOME="$T/.local/share" \
    XDG_RUNTIME_DIR="$T/run" LANG=it_IT.UTF-8 LC_ALL=it_IT.UTF-8 QT_QPA_PLATFORM=xcb USER=vela \
    VT_ARTIFACTS="${VT_ARTIFACTS:-/workspace/vela/artifacts/terminal}" \
    timeout 300 prlimit --as=3000000000 python3 "$HERE/gui_smoke.py"
