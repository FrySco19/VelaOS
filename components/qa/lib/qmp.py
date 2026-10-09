#!/usr/bin/env python3
"""Minimal QMP client for QA (host side; works on live ISO and dev VM, no guest tools needed).

usage: qmp.py SOCK screenshot out.png
       qmp.py SOCK key ctrl-alt-t            (QEMU qcodes joined by '-': ctrl, alt, shift, meta_l, ret, esc, tab, spc, f2, print, ...)
       qmp.py SOCK type "testo"              (guest layout QA_LAYOUT=it (default) or us)
       qmp.py SOCK click X Y [left|right|middle] [W H]   (absolute; needs usb-tablet/virtio-tablet; X,Y in screen pixels)
       qmp.py SOCK move X Y [W H]
       qmp.py SOCK dblclick X Y [W H]
       qmp.py SOCK drag X1 Y1 X2 Y2 [W H]
       qmp.py SOCK raw '{"execute":"query-status"}'
Screen size W H for absolute coords defaults to the size of the last screenshot (env QMP_W/QMP_H or probed via screendump).
"""
import json, os, socket, sys, tempfile, time

def connect(path):
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.connect(path); f = s.makefile('rw')
    json.loads(f.readline())
    cmd(f, 'qmp_capabilities'); return s, f

def cmd(f, name, **args):
    f.write(json.dumps({'execute': name, **({'arguments': args} if args else {})}) + '\n'); f.flush()
    while True:
        r = json.loads(f.readline())
        if 'return' in r or 'error' in r:
            if 'error' in r: raise SystemExit(f'QMP error {name}: {r["error"]}')
            return r['return']

def screendump(f, out):
    tmp = tempfile.mktemp(suffix='.ppm', dir='/tmp')
    try:
        cmd(f, 'screendump', filename=tmp, format='png'); os.replace(tmp, out); return
    except SystemExit:
        pass
    cmd(f, 'screendump', filename=tmp)
    from PIL import Image
    Image.open(tmp).save(out); os.unlink(tmp)

def size(f):
    if os.environ.get('QMP_W'): return int(os.environ['QMP_W']), int(os.environ['QMP_H'])
    tmp = tempfile.mktemp(suffix='.png', dir='/tmp'); screendump(f, tmp)
    from PIL import Image
    w, h = Image.open(tmp).size; os.unlink(tmp); return w, h

def keys(f, combo, hold=0.05):
    ks = [{'type': 'qcode', 'data': k} for k in combo.split('-') if k]
    cmd(f, 'send-key', keys=ks, **{'hold-time': int(hold * 1000) or 50})

CHAR = {' ': 'spc', '\n': 'ret', '\t': 'tab', '-': 'minus', '=': 'equal', '.': 'dot', ',': 'comma', '/': 'slash',
        ';': 'semicolon', "'": 'apostrophe', '[': 'bracket_left', ']': 'bracket_right', '\\': 'backslash', '`': 'grave_accent'}
SHIFT = {'_': 'minus', '+': 'equal', ':': 'semicolon', '"': 'apostrophe', '?': 'slash', '>': 'dot', '<': 'comma',
         '!': '1', '@': '2', '#': '3', '$': '4', '%': '5', '^': '6', '&': '7', '*': '8', '(': '9', ')': '0',
         '{': 'bracket_left', '}': 'bracket_right', '|': 'backslash', '~': 'grave_accent'}
IT = {' ': 'spc', '\n': 'ret', '\t': 'tab', '/': 'shift-7', '-': 'slash', '_': 'shift-slash', '.': 'dot', ',': 'comma',
      ':': 'shift-dot', ';': 'shift-comma', "'": 'minus', '?': 'shift-minus', '"': 'shift-2', '!': 'shift-1', '=': 'shift-0',
      '+': 'bracket_right', '*': 'shift-bracket_right', '(': 'shift-8', ')': 'shift-9', '&': 'shift-6', '%': 'shift-5',
      '$': 'shift-4', '<': 'less', '>': 'shift-less', '|': 'shift-backslash', '\\': 'grave_accent', '@': 'alt_r-semicolon',
      '#': 'alt_r-apostrophe', '[': 'alt_r-bracket_left', ']': 'alt_r-bracket_right', '~': None,
      'è': 'bracket_left', 'é': 'shift-bracket_left', 'ò': 'semicolon', 'à': 'apostrophe', 'ù': 'backslash', 'ì': 'equal'}
def type_text(f, text):
    """Type text. Guest layout from env QA_LAYOUT (default 'it', Vela default; 'us' also supported)."""
    it = os.environ.get('QA_LAYOUT', 'it') == 'it'
    for ch in text:
        if ch.isalpha() and ch.isascii():
            keys(f, ('shift-' if ch.isupper() else '') + ch.lower())
        elif ch.isdigit(): keys(f, ch)
        elif it and ch in IT and IT[ch]: keys(f, IT[ch])
        elif not it and ch in CHAR: keys(f, CHAR[ch])
        elif not it and ch in SHIFT: keys(f, 'shift-' + SHIFT[ch])
        else: print(f'skip unsupported char {ch!r}', file=sys.stderr)
        time.sleep(0.02)

def absmove(f, x, y, w, h):
    cmd(f, 'input-send-event', events=[
        {'type': 'abs', 'data': {'axis': 'x', 'value': int(x * 32767 / max(w - 1, 1))}},
        {'type': 'abs', 'data': {'axis': 'y', 'value': int(y * 32767 / max(h - 1, 1))}}])

def button(f, b, down):
    cmd(f, 'input-send-event', events=[{'type': 'btn', 'data': {'down': down, 'button': b}}])

def main():
    sock, op, *a = sys.argv[1:]
    s, f = connect(sock)
    if op == 'screenshot': screendump(f, a[0])
    elif op == 'key': keys(f, a[0])
    elif op == 'type': type_text(f, a[0])
    elif op in ('click', 'move', 'dblclick'):
        x, y = int(a[0]), int(a[1])
        rest = a[2:]
        b = rest.pop(0) if rest and not rest[0].isdigit() else 'left'
        w, h = (int(rest[0]), int(rest[1])) if len(rest) >= 2 else size(f)
        absmove(f, x, y, w, h); time.sleep(0.15)
        if op != 'move':
            for _ in range(2 if op == 'dblclick' else 1):
                button(f, b, True); time.sleep(0.06); button(f, b, False); time.sleep(0.08)
    elif op == 'drag':
        x1, y1, x2, y2 = map(int, a[:4])
        w, h = (int(a[4]), int(a[5])) if len(a) >= 6 else size(f)
        absmove(f, x1, y1, w, h); time.sleep(0.15); button(f, 'left', True); time.sleep(0.2)
        for i in range(1, 11):
            absmove(f, x1 + (x2 - x1) * i // 10, y1 + (y2 - y1) * i // 10, w, h); time.sleep(0.04)
        time.sleep(0.2); button(f, 'left', False)
    elif op == 'raw':
        j = json.loads(a[0]); print(json.dumps(cmd(f, j['execute'], **j.get('arguments', {})), indent=1))
    else: raise SystemExit(__doc__)
    s.close()

if __name__ == '__main__': main()
