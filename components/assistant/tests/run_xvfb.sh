#!/bin/sh
# GUI smoke test on the box: unit tests, then the real QML UI rendered under Xvfb in its main states (light + dark),
# then the D-Bus integration test. Output: /workspace/vela/artifacts/assistant/ (or $1).
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
APP=$(cd "$HERE/../overlay/usr/lib/vela/assistant" && pwd)
OUT=${1:-/workspace/vela/artifacts/assistant}
mkdir -p "$OUT/xvfb"
(cd "$HERE/.." && python3 -m pytest -q tests) | tee "$OUT/unit-tests.txt"
TMPH=$(mktemp -d)
HOME=$TMPH XDG_CONFIG_HOME=$TMPH/.config XDG_DATA_HOME=$TMPH/.local/share \
  xvfb-run -a -s "-screen 0 1280x800x24" python3 "$APP/vela_assistant.py" --shots "$OUT/xvfb" --no-dbus --no-tray
rm -rf "$TMPH"
python3 -c "import json,sys; d=json.load(open(sys.argv[1])); w=d.get('qml_warnings',[]); print('qml warnings:', len(w)); [print(' ', x) for x in w]" "$OUT/xvfb/shots.json"
sh "$HERE/run_dbus_xvfb.sh" "$OUT/dbus"
