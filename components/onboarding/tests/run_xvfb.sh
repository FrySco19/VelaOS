#!/bin/sh
# Standalone tests for Benvenuto / Guida / Suggerimenti on the box (no VM): Xvfb + llvmpipe, dry-run mode
# (no system command is executed; every intended command is logged). Evidence -> $OUT.
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
APP=$HERE/../overlay/usr/lib/vela/welcome
OUT=${OUT:-/workspace/vela/artifacts/onboarding/xvfb-$(date +%Y%m%d-%H%M)}
T=$(mktemp -d)
mkdir -p "$OUT"
export HOME=$T/home XDG_CONFIG_HOME=$T/home/.config XDG_CACHE_HOME=$T/home/.cache
export VELA_WELCOME_DRYRUN=1 VELA_WELCOME_LIVE=1 VELA_WELCOME_LOG=$OUT/actions.log
export VELA_TOKENS=${VELA_TOKENS:-$HERE/../../theme/tokens.json}
export VELA_WELCOME_FAKE_WIFI='[{"ssid":"Casa","signal":80,"secure":true,"active":true},{"ssid":"TIM-28471","signal":55,"secure":true,"active":false},{"ssid":"Ospiti","signal":30,"secure":false,"active":false}]'
mkdir -p "$HOME"
X="xvfb-run -a -s -screen\ 0\ 1280x800x24"
run() { timeout 150 xvfb-run -a -s "-screen 0 1280x800x24" prlimit --as=2500000000 python3 "$APP/vela_welcome.py" "$@"; }

echo "1/6 data files"
python3 - "$APP" <<'P'
import json, re, sys, os
app = sys.argv[1]
s = json.load(open(os.path.join(app, "strings.json")))
assert set(s["it"]) == set(s["en"]), set(s["it"]) ^ set(s["en"])
banned = [re.compile(l.strip(), re.I) for l in open(os.path.join(app, "../../../../../../qa/lib/banned_phrases.txt"))
          if l.strip() and not l.startswith("#")]
bad = [(k, v) for lang in s for k, v in s[lang].items() for b in banned if b.search(v)]
assert not bad, bad
for f in ("shortcuts.json", "tips.json"):
    json.load(open(os.path.join(app, f)))
print("   strings:", len(s["it"]), "keys, it/en parity ok, no banned phrases")
P
echo "2/6 wizard: every page, it/en, light/dark, a11y"
run --walk "$OUT/wizard" >"$OUT/wizard.log" 2>&1
echo "3/6 guide"
run --guida --walk "$OUT/guide" >"$OUT/guide.log" 2>&1
echo "4/6 keyboard-only run (Tab/Return)"
rm -rf "$XDG_CONFIG_HOME/vela"
run --keytest "$OUT/keys" >"$OUT/keys.log" 2>&1
python3 - "$OUT/keys/keyboard-test.json" "$XDG_CONFIG_HOME/vela/welcome.json" <<'P'
import json, sys
d = json.load(open(sys.argv[1])); st = json.load(open(sys.argv[2]))
assert d["visited_pages"][-1] == 9 and sorted(set(d["visited_pages"])) == list(range(10)), d["visited_pages"]
print("   reached all 10 pages with keyboard only; Benvenuto marked done:", st.get("done"))
P
echo "5/6 first-run gate: second start must exit immediately"
start=$(date +%s%N); run --first-run; end=$(date +%s%N)
echo "   --first-run after completion exited in $(( (end-start)/1000000 )) ms"
echo "6/6 tips"
VELA_WELCOME_LIVE=0 python3 "$APP/vela_tips.py" --dry-run | tee "$OUT/tips-dry-run.json"
for f in "$OUT"/wizard/qml-warnings.txt "$OUT"/guide/qml-warnings.txt; do
    if [ -s "$f" ] && grep -q . "$f"; then echo "QML WARNINGS in $f:"; cat "$f"; fi
done
echo "screens: $(ls "$OUT"/wizard/*.png "$OUT"/guide/*.png | wc -l) png in $OUT"
rm -rf "$T"
