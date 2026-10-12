#!/usr/bin/env python3
"""Boot the OS-0 guest headless, drive it through the QEMU monitor and capture
screendumps as PNG evidence (brief section 3: QEMU screendump).

  python3 tools/qemu-shots.py [--out build/shots]
"""
import argparse, os, socket, subprocess, sys, time
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
G = os.path.join(ROOT, "build", "guest")

def hmp(sock, cmd, wait=0.2):
    sock.sendall((cmd + "\n").encode()); time.sleep(wait)
    try:
        return sock.recv(65536).decode(errors="replace")
    except BlockingIOError:
        return ""

def shot(sock, out, name):
    ppm = os.path.join(out, name + ".ppm")
    hmp(sock, f"screendump {ppm}", 1.5)
    png = os.path.join(out, name + ".png")
    Image.open(ppm).save(png); os.remove(ppm)
    print("screendump", png); return png

def move_to(sock, x, y, cur):
    # PS/2 mouse is relative: move in small steps (guest clamps to the screen)
    dx, dy = x - cur[0], y - cur[1]
    steps = max(1, int(max(abs(dx), abs(dy)) / 60))
    for i in range(steps):
        hmp(sock, f"mouse_move {int(dx/steps)} {int(dy/steps)}", 0.05)
    cur[0], cur[1] = x, y

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--out", default=os.path.join(ROOT, "build", "shots"))
    ap.add_argument("--boot-timeout", type=int, default=240)
    a = ap.parse_args(); os.makedirs(a.out, exist_ok=True)
    for f in ("serial.log", "monitor.sock"):
        try: os.remove(os.path.join(G, f))
        except FileNotFoundError: pass
    q = subprocess.Popen([os.path.join(ROOT, "guest", "run-qemu.sh"), "--headless"])
    try:
        t0 = time.time(); log = ""
        while time.time() - t0 < a.boot_timeout:
            time.sleep(1)
            try: log = open(os.path.join(G, "serial.log"), errors="replace").read()
            except FileNotFoundError: continue
            if "first frame drawn" in log: break
        print(log[-1500:])
        if "first frame drawn" not in log: print("guest did not draw a frame", file=sys.stderr); return 1
        print(f"first frame after {time.time()-t0:.1f}s")
        s = socket.socket(socket.AF_UNIX); s.connect(os.path.join(G, "monitor.sock")); s.setblocking(False)
        time.sleep(1); hmp(s, "", 0.3)
        shot(s, a.out, "01-boot-desktop")
        # guest cursor starts at screen centre (960,540); move it to the dock's Notes glyph
        cur = [960, 540]
        move_to(s, 960, 1020, cur); time.sleep(2); shot(s, a.out, "02-pointer-over-dock")
        # click the visible left part of the Find window -> focus + raise
        move_to(s, 400, 500, cur); hmp(s, "mouse_button 1", .3); hmp(s, "mouse_button 0", .3); time.sleep(2)
        shot(s, a.out, "03-click-raises-find")
        hmp(s, "sendkey f11", .3); time.sleep(2); shot(s, a.out, "04-focus-mode")
        hmp(s, "sendkey esc", .3); hmp(s, "sendkey ctrl-alt-1", .3); time.sleep(4); shot(s, a.out, "05-theme-light")
        hmp(s, "sendkey ctrl-alt-2", .3); time.sleep(4); shot(s, a.out, "06-theme-dawn")
        hmp(s, "sendkey ctrl-alt-3", .3); hmp(s, "sendkey ctrl-alt-right", .3); time.sleep(4); shot(s, a.out, "07-space-2")
        log = open(os.path.join(G, "serial.log"), errors="replace").read()
        open(os.path.join(a.out, "serial.log"), "w").write(log)
        hmp(s, "quit", .5)
    finally:
        try: q.wait(timeout=10)
        except subprocess.TimeoutExpired: q.kill()
    return 0

if __name__ == "__main__":
    sys.exit(main())
