#!/usr/bin/env python3
"""Minimal QMP client for Vela VM harness (no deps).

usage: qmp.py SOCK screenshot OUT.png
       qmp.py SOCK key ctrl-alt-t [hold_ms]
       qmp.py SOCK type "text" [--layout it|us]
       qmp.py SOCK mouse X Y [click|dclick|rclick|none] [--size WxH]   (absolute, needs usb-tablet)
       qmp.py SOCK cmd '{"execute": "query-status"}'
       qmp.py SOCK powerdown | quit | status
"""
import json, os, socket, subprocess, sys, tempfile, time

class QMP:
    def __init__(self, path):
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.connect(path)
        self.f = self.s.makefile("rw")
        json.loads(self.f.readline())  # greeting
        self.cmd("qmp_capabilities")

    def cmd(self, execute, **args):
        msg = {"execute": execute}
        if args:
            msg["arguments"] = args
        self.f.write(json.dumps(msg) + "\n"); self.f.flush()
        while True:
            r = json.loads(self.f.readline())
            if "event" in r:
                continue
            if "error" in r:
                raise RuntimeError(f"{execute}: {r['error']}")
            return r.get("return")

# qcode names; Italian (it) layout map: char -> (qcode, shift, altgr)
BASE = {c: (c, False, False) for c in "abcdefghijklmnopqrstuvwxyz0123456789"}
BASE.update({c.upper(): (c, True, False) for c in "abcdefghijklmnopqrstuvwxyz"})
BASE.update({" ": ("spc", False, False), "\n": ("ret", False, False), "\t": ("tab", False, False)})
IT = dict(BASE)
IT.update({
    "!": ("1", 1, 0), '"': ("2", 1, 0), "£": ("3", 1, 0), "$": ("4", 1, 0), "%": ("5", 1, 0),
    "&": ("6", 1, 0), "/": ("7", 1, 0), "(": ("8", 1, 0), ")": ("9", 1, 0), "=": ("0", 1, 0),
    "'": ("minus", 0, 0), "?": ("minus", 1, 0), "`": ("minus", 0, 1),
    "ì": ("equal", 0, 0), "^": ("equal", 1, 0), "~": ("equal", 0, 1),
    "è": ("bracket_left", 0, 0), "é": ("bracket_left", 1, 0), "[": ("bracket_left", 0, 1), "{": ("bracket_left", 1, 1),
    "+": ("bracket_right", 0, 0), "*": ("bracket_right", 1, 0), "]": ("bracket_right", 0, 1), "}": ("bracket_right", 1, 1),
    "ò": ("semicolon", 0, 0), "ç": ("semicolon", 1, 0), "@": ("semicolon", 0, 1),
    "à": ("apostrophe", 0, 0), "°": ("apostrophe", 1, 0), "#": ("apostrophe", 0, 1),
    "ù": ("backslash", 0, 0), "§": ("backslash", 1, 0),
    "\\": ("grave_accent", 0, 0), "|": ("grave_accent", 1, 0),
    ",": ("comma", 0, 0), ";": ("comma", 1, 0), ".": ("dot", 0, 0), ":": ("dot", 1, 0),
    "-": ("slash", 0, 0), "_": ("slash", 1, 0), "<": ("less", 0, 0), ">": ("less", 1, 0),
    "€": ("e", 0, 1),
})
US = dict(BASE)
US.update({
    "!": ("1", 1, 0), "@": ("2", 1, 0), "#": ("3", 1, 0), "$": ("4", 1, 0), "%": ("5", 1, 0), "^": ("6", 1, 0),
    "&": ("7", 1, 0), "*": ("8", 1, 0), "(": ("9", 1, 0), ")": ("0", 1, 0), "-": ("minus", 0, 0), "_": ("minus", 1, 0),
    "=": ("equal", 0, 0), "+": ("equal", 1, 0), "[": ("bracket_left", 0, 0), "{": ("bracket_left", 1, 0),
    "]": ("bracket_right", 0, 0), "}": ("bracket_right", 1, 0), ";": ("semicolon", 0, 0), ":": ("semicolon", 1, 0),
    "'": ("apostrophe", 0, 0), '"': ("apostrophe", 1, 0), "`": ("grave_accent", 0, 0), "~": ("grave_accent", 1, 0),
    "\\": ("backslash", 0, 0), "|": ("backslash", 1, 0), ",": ("comma", 0, 0), "<": ("comma", 1, 0),
    ".": ("dot", 0, 0), ">": ("dot", 1, 0), "/": ("slash", 0, 0), "?": ("slash", 1, 0),
})
ALIAS = {"ctrl": "ctrl", "control": "ctrl", "alt": "alt", "altgr": "alt_r", "shift": "shift", "super": "meta_l",
         "meta": "meta_l", "win": "meta_l", "enter": "ret", "return": "ret", "esc": "esc", "escape": "esc",
         "space": "spc", "del": "delete", "backspace": "backspace", "bs": "backspace", "pgup": "pgup", "pgdn": "pgdn"}

def keys(q, combo, hold=80):
    ks = [{"type": "qcode", "data": ALIAS.get(k.lower(), k.lower())} for k in combo.split("-") if k]
    q.cmd("send-key", keys=ks, **{"hold-time": hold})

def type_text(q, text, layout="it", delay=0.02):
    m = IT if layout == "it" else US
    for ch in text:
        if ch not in m:
            raise SystemExit(f"char {ch!r} not mappable on layout {layout}")
        code, sh, ag = m[ch]
        ks = []
        if sh: ks.append({"type": "qcode", "data": "shift"})
        if ag: ks.append({"type": "qcode", "data": "alt_r"})
        ks.append({"type": "qcode", "data": code})
        q.cmd("send-key", keys=ks, **{"hold-time": 30})
        time.sleep(delay)

def screenshot(q, out):
    out = os.path.abspath(out)
    if out.endswith(".png"):
        try:
            q.cmd("screendump", filename=out, format="png"); return
        except RuntimeError:
            pass
    tmp = tempfile.mktemp(suffix=".ppm", dir=os.path.dirname(out) or ".")
    q.cmd("screendump", filename=tmp)
    subprocess.run(["convert", tmp, out], check=True); os.unlink(tmp)

def mouse(q, x, y, action="click", size=None):
    # absolute coords scaled to 0..32767 using the current screen size
    if size:
        w, h = map(int, size.split("x"))
    else:
        tmp = tempfile.mktemp(suffix=".ppm"); q.cmd("screendump", filename=tmp)
        with open(tmp, "rb") as f:
            f.readline(); dims = f.readline().split()
            while dims and dims[0].startswith(b"#"): dims = f.readline().split()
            w, h = int(dims[0]), int(dims[1])
        os.unlink(tmp)
    ax, ay = int(x * 32767 / max(w - 1, 1)), int(y * 32767 / max(h - 1, 1))
    q.cmd("input-send-event", events=[{"type": "abs", "data": {"axis": "x", "value": ax}},
                                      {"type": "abs", "data": {"axis": "y", "value": ay}}])
    btn = {"click": "left", "dclick": "left", "rclick": "right", "mclick": "middle"}.get(action)
    if btn:
        for _ in range(2 if action == "dclick" else 1):
            for down in (True, False):
                q.cmd("input-send-event", events=[{"type": "btn", "data": {"down": down, "button": btn}}])
                time.sleep(0.05)

def main(a):
    if len(a) < 2:
        print(__doc__); sys.exit(2)
    q = QMP(a[0]); op = a[1]
    if op == "screenshot": screenshot(q, a[2])
    elif op == "key": keys(q, a[2], int(a[3]) if len(a) > 3 else 80)
    elif op == "type":
        layout = "it"
        if "--layout" in a: layout = a[a.index("--layout") + 1]
        type_text(q, a[2], layout)
    elif op == "mouse":
        size = a[a.index("--size") + 1] if "--size" in a else None
        act = a[4] if len(a) > 4 and not a[4].startswith("--") else "click"
        mouse(q, int(a[2]), int(a[3]), act, size)
    elif op == "cmd":
        m = json.loads(a[2]); print(json.dumps(q.cmd(m["execute"], **m.get("arguments", {})), indent=1))
    elif op == "powerdown": q.cmd("system_powerdown")
    elif op == "quit": q.cmd("quit")
    elif op == "status": print(q.cmd("query-status")["status"])
    else: print(__doc__); sys.exit(2)

if __name__ == "__main__":
    main(sys.argv[1:])
