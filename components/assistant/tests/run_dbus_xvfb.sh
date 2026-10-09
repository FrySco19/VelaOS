#!/bin/sh
# Integration test on the box: private D-Bus session + Xvfb, D-Bus activation of the KRunner runner and of the
# assistant, forwarding from the launcher script, real window screenshots. No KDE needed.
# Usage: tests/run_dbus_xvfb.sh [OUTDIR]
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
APP=$(cd "$HERE/../overlay/usr/lib/vela/assistant" && pwd)
OUT=${1:-/workspace/vela/artifacts/assistant/dbus}
mkdir -p "$OUT"
TMP=$(mktemp -d)
mkdir -p "$TMP/share/dbus-1/services" "$TMP/home"
cat > "$TMP/share/dbus-1/services/org.vela.Assistant.service" <<EOT
[D-BUS Service]
Name=org.vela.Assistant
Exec=/usr/bin/python3 $APP/vela_assistant.py --background --no-tray
EOT
cat > "$TMP/share/dbus-1/services/org.vela.AssistantRunner.service" <<EOT
[D-BUS Service]
Name=org.vela.AssistantRunner
Exec=/usr/bin/python3 $APP/krunner_runner.py
EOT
export HOME="$TMP/home" XDG_CONFIG_HOME="$TMP/home/.config" XDG_DATA_HOME="$TMP/home/.local/share"
export XDG_DATA_DIRS="$TMP/share:/usr/local/share:/usr/share" LANG=it_IT.UTF-8 VELA_ASSISTANT_NO_BLUR=1
# a mock OpenAI-compatible model so the panel can answer
python3 "$HERE/mock_openai.py" 18771 > "$TMP/mock.log" 2>&1 &
MOCK=$!
sleep 0.5
mkdir -p "$XDG_CONFIG_HOME/vela"
printf '{"version":1,"provider":"ollama","ollama":{"base_url":"http://127.0.0.1:18771","model":"qwen2.5:1.5b"}}\n' > "$XDG_CONFIG_HOME/vela/ai.json"
echo "appunti di prova" > "$HOME/appunti.txt"

cat > "$TMP/inner.sh" <<'EOT'
set -u
pass=0; fail=0
ok()   { echo "PASS $1"; pass=$((pass+1)); }
bad()  { echo "FAIL $1: $2"; fail=$((fail+1)); }
R="busctl --user call org.vela.AssistantRunner /runner org.kde.krunner1"
out=$($R Match s "tema scuro" 2>&1) && echo "$out" | grep -q "Passa al tema scuro" && ok runner-intent || bad runner-intent "$out"
out=$($R Match s "? come si fa uno screenshot" 2>&1) && echo "$out" | grep -q "Chiedi all" && ok runner-ask || bad runner-ask "$out"
out=$($R Match s "firefox" 2>&1) && echo "$out" | grep -q "^a(sssida{sv}) 0" && ok runner-no-noise || bad runner-no-noise "$out"
# Run: activates org.vela.Assistant via D-Bus, opens the panel and asks
$R Run ss "ask:Ciao, presentati" "" >/dev/null 2>&1 && ok runner-run || bad runner-run "call failed"
for i in $(seq 1 40); do busctl --user call org.vela.Assistant /Assistant org.vela.Assistant Ping >/dev/null 2>&1 && break; sleep 0.25; done
busctl --user call org.vela.Assistant /Assistant org.vela.Assistant Ping 2>/dev/null | grep -q ok && ok dbus-activation || bad dbus-activation "no org.vela.Assistant"
sleep 2.5
import -window root "$OUT/01-krunner-run-ask.png" && ok shot1 || bad shot1 import
# launcher wrapper logic: toggle hides the visible+active window, then shows it again
python3 "$APP/vela_assistant.py" --files "$HOME/appunti.txt" && ok forward-files || bad forward-files rc
sleep 1.5
import -window root "$OUT/02-files-attached.png" && ok shot2 || bad shot2 import
$R Run ss 'intent:{"tool": "set_dark_mode", "args": {"enabled": true}}' "" >/dev/null 2>&1 && ok runner-run-intent || bad runner-run-intent call
sleep 1.5
import -window root "$OUT/03-intent-card.png" && ok shot3 || bad shot3 import
# action log written by the assistant (the question above was sent to the local mock model)
grep -q '"app": "assistant"' "$XDG_DATA_HOME/vela/ai-actions.jsonl" && ok action-log || bad action-log "missing"
echo "RESULT pass=$pass fail=$fail"
EOT
export APP OUT
xvfb-run -a -s "-screen 0 1280x800x24" dbus-run-session -- sh "$TMP/inner.sh" 2>&1 | grep -E "^(PASS|FAIL|RESULT)" | tee "$OUT/results.txt"
kill $MOCK 2>/dev/null || true
cp "$XDG_DATA_HOME/vela/ai-actions.jsonl" "$OUT/ai-actions.jsonl" 2>/dev/null || true
rm -rf "$TMP"
