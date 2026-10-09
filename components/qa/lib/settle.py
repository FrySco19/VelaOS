#!/usr/bin/env python3
"""Measure UI latency from the host: poll QMP screendumps until the screen changes, then until it is stable.
usage: settle.py SOCK [timeout_s=15] [stable_ms=600]  -> prints JSON {"changed_after_ms":..,"settled_after_ms":..}
Start it, then trigger the action (or run it right after the trigger: the baseline is the first frame)."""
import json, os, sys, time, tempfile
sys.path.insert(0, os.path.dirname(__file__))
import qmp
from PIL import Image, ImageChops

sock = sys.argv[1]; timeout = float(sys.argv[2]) if len(sys.argv) > 2 else 15; stable = float(sys.argv[3]) / 1000 if len(sys.argv) > 3 else 0.6
s, f = qmp.connect(sock)
tmp = tempfile.mktemp(suffix='.png', dir='/tmp')
def grab():
    qmp.screendump(f, tmp); return Image.open(tmp).convert('L').resize((320, 200))
def diff(a, b):
    h = ImageChops.difference(a, b).histogram(); return sum(h[12:]) / (320 * 200)
t0 = time.time(); base = grab(); prev = base; changed = None; last_change = None
while time.time() - t0 < timeout:
    cur = grab(); now = time.time()
    if changed is None and diff(base, cur) > 0.01: changed = last_change = now
    elif changed is not None:
        if diff(prev, cur) > 0.002: last_change = now
        elif now - last_change >= stable: break
    prev = cur
os.path.exists(tmp) and os.unlink(tmp)
print(json.dumps({'changed_after_ms': None if changed is None else int((changed - t0) * 1000),
                  'settled_after_ms': None if last_change is None else int((last_change - t0) * 1000),
                  'timeout': time.time() - t0 >= timeout}))
