#!/usr/bin/env python3
"""Vela Plasma style ("vela"): glass panels/popups with 12-16 px radius, soft shadows.
Only the frames that define the look are shipped; everything else falls back to Breeze
("default") automatically. Colors follow the active color scheme (ColorScheme-* classes).
"""
import base64, gzip, io, json, os, subprocess, tempfile
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', '..', 'overlay', 'usr', 'share', 'plasma', 'desktoptheme', 'vela')
SS = 2
STYLE = ('<style type="text/css" id="current-color-scheme">'
         '.ColorScheme-Text{color:#15303A;stop-color:#15303A;}'
         '.ColorScheme-Background{color:#F3F6F7;stop-color:#F3F6F7;}'
         '.ColorScheme-Highlight{color:#D4624A;}</style>')

def shadow_tiles(r, S, dy, layers):
    """Blurred rounded rect; returns dict of 8 PIL tiles (outside S + inside r) at SSx."""
    WW = HH = 4 * r + 40
    St, Sb = S - dy, S + dy
    cw, ch = (S + WW + S) * SS, (St + HH + Sb) * SS
    acc = np.zeros((ch, cw), np.float32)
    for sigma, ody, alpha in layers:
        m = Image.new('L', (cw, ch), 0)
        x0, y0 = S * SS, (St + ody) * SS
        ImageDraw.Draw(m).rounded_rectangle([x0, y0, x0 + WW * SS - 1, y0 + HH * SS - 1], radius=r * SS, fill=255)
        a = np.asarray(m.filter(ImageFilter.GaussianBlur(sigma * SS)), np.float32) / 255 * alpha
        acc = 1 - (1 - acc) * (1 - a)
    # punch out the frame itself (shadow only outside the shape)
    hole = Image.new('L', (cw, ch), 0)
    ImageDraw.Draw(hole).rounded_rectangle([S * SS, St * SS, (S + WW) * SS - 1, (St + HH) * SS - 1], radius=r * SS, fill=255)
    acc *= 1 - np.asarray(hole, np.float32) / 255
    arr = np.zeros((ch, cw, 4), np.uint8); arr[..., 3] = np.clip(acc * 255 + .5, 0, 255).astype(np.uint8)
    im = Image.fromarray(arr, 'RGBA')
    X0, X1, Y0, Y1 = S, S + WW, St, St + HH     # frame bounds in canvas units
    mid_x, mid_y = S + WW // 2, St + HH // 2
    boxes = {
        'topleft': (0, 0, X0 + r, Y0 + r), 'top': (mid_x, 0, mid_x + 4, Y0 + r), 'topright': (X1 - r, 0, X1 + S, Y0 + r),
        'left': (0, mid_y, X0 + r, mid_y + 4), 'right': (X1 - r, mid_y, X1 + S, mid_y + 4),
        'bottomleft': (0, Y1 - r, X0 + r, Y1 + Sb), 'bottom': (mid_x, Y1 - r, mid_x + 4, Y1 + Sb),
        'bottomright': (X1 - r, Y1 - r, X1 + S, Y1 + Sb)}
    return {k: (im.crop(tuple(v * SS for v in b)), (b[2] - b[0], b[3] - b[1])) for k, b in boxes.items()}, (St, Sb, S)

def b64(im):
    buf = io.BytesIO(); im.save(buf, 'PNG', optimize=True); return base64.b64encode(buf.getvalue()).decode()

def frame_elements(prefix, ox, oy, r, op, border_op, edge=4, cls='ColorScheme-Background', square=False):
    """9 frame elements; corner r x r; a 1px inner stroke in ColorScheme-Text at border_op."""
    p = (prefix + '-') if prefix else ''
    E = []
    rr = 0 if square else r
    x, y = ox, oy                          # topleft at (x,y); layout: corners r, sides 'edge' long
    cx, cy = x + r, y + r                  # center block origin
    fill = f'class="{cls}" fill="currentColor" opacity="{op}"'
    line = f'class="ColorScheme-Text" fill="currentColor" opacity="{border_op}"'
    def corner(name, X, Y, d_fill, d_line):
        E.append(f'<g id="{p}{name}"><rect x="{X}" y="{Y}" width="{r}" height="{r}" fill="none"/>'
                 f'<path d="{d_fill}" {fill}/>' + (f'<path d="{d_line}" {line}/>' if border_op > 0 else '') + '</g>')
    # rounded corner fill paths (quarter disc to the inner corner) and 1px ring
    k = 0.5523
    def qd(X, Y, sx, sy, rad):
        # quarter disc anchored at inner corner (X+sx*r, Y+sy*r) ... build generic via arc
        return None
    tl_f = f'M{x} {y+r} A{r} {r} 0 0 1 {x+r} {y} V{y+r} Z' if not square else f'M{x} {y} H{x+r} V{y+r} H{x} Z'
    tl_l = (f'M{x} {y+r} A{r} {r} 0 0 1 {x+r} {y} V{y+1} A{r-1} {r-1} 0 0 0 {x+1} {y+r} Z') if not square else f'M{x} {y} H{x+r} V{y+1} H{x+1} V{y+r} H{x} Z'
    corner('topleft', x, y, tl_f, tl_l)
    X = x + r + edge
    tr_f = f'M{X} {y} A{r} {r} 0 0 1 {X+r} {y+r} H{X} Z' if not square else f'M{X} {y} H{X+r} V{y+r} H{X} Z'
    tr_l = (f'M{X} {y} A{r} {r} 0 0 1 {X+r} {y+r} H{X+r-1} A{r-1} {r-1} 0 0 0 {X} {y+1} Z') if not square else f'M{X} {y} H{X+r} V{y+r} H{X+r-1} V{y+1} H{X} Z'
    corner('topright', X, y, tr_f, tr_l)
    Y = y + r + edge
    bl_f = f'M{x} {Y} H{x+r} V{Y+r} A{r} {r} 0 0 1 {x} {Y} Z' if not square else f'M{x} {Y} H{x+r} V{Y+r} H{x} Z'
    bl_l = (f'M{x} {Y} H{x+1} A{r-1} {r-1} 0 0 0 {x+r} {Y+r-1} V{Y+r} A{r} {r} 0 0 1 {x} {Y} Z') if not square else f'M{x} {Y} H{x+1} V{Y+r-1} H{x+r} V{Y+r} H{x} Z'
    corner('bottomleft', x, Y, bl_f, bl_l)
    br_f = f'M{X} {Y} H{X+r} A{r} {r} 0 0 1 {X} {Y+r} Z' if not square else f'M{X} {Y} H{X+r} V{Y+r} H{X} Z'
    br_l = (f'M{X+r} {Y} A{r} {r} 0 0 1 {X} {Y+r} V{Y+r-1} A{r-1} {r-1} 0 0 0 {X+r-1} {Y} Z') if not square else f'M{X+r-1} {Y} H{X+r} V{Y+r} H{X} V{Y+r-1} H{X+r-1} Z'
    corner('bottomright', X, Y, br_f, br_l)
    def side(name, X, Y, w, h, lx, ly, lw, lh):
        E.append(f'<g id="{p}{name}"><rect x="{X}" y="{Y}" width="{w}" height="{h}" {fill}/>'
                 + (f'<rect x="{lx}" y="{ly}" width="{lw}" height="{lh}" {line}/>' if border_op > 0 else '') + '</g>')
    side('top', x + r, y, edge, r, x + r, y, edge, 1)
    side('bottom', x + r, Y, edge, r, x + r, Y + r - 1, edge, 1)
    side('left', x, y + r, r, edge, x, y + r, 1, edge)
    side('right', X, y + r, r, edge, X + r - 1, y + r, 1, edge)
    E.append(f'<rect id="{p}center" x="{x+r}" y="{y+r}" width="{edge}" height="{edge}" {fill}/>')
    return E, (2 * r + edge, 2 * r + edge)

def mask_elements(ox, oy, r, edge=4, square=False):
    E, _ = frame_elements('mask', ox, oy, r, 1, 0, edge, cls='', square=square)
    return [e.replace('class="" ', '').replace('fill="currentColor"', 'fill="#000"') for e in E]

def hints(prefix, ox, oy, m):
    p = (prefix + '-') if prefix else ''
    tiny = '0.00000001'
    return [f'<rect id="{p}hint-top-margin" x="{ox}" y="{oy}" width="4" height="{m}" fill="#f0f"/>',
            f'<rect id="{p}hint-bottom-margin" x="{ox+6}" y="{oy}" width="4" height="{m}" fill="#f0f"/>',
            f'<rect id="{p}hint-left-margin" x="{ox+12}" y="{oy}" width="{m}" height="4" fill="#f0f"/>',
            f'<rect id="{p}hint-right-margin" x="{ox+12+m+2}" y="{oy}" width="{m}" height="4" fill="#f0f"/>'] + (
           [f'<rect id="hint-top-inset" x="{ox}" y="{oy+20}" width="4" height="{tiny}" fill="#0f0"/>',
            f'<rect id="hint-bottom-inset" x="{ox+6}" y="{oy+20}" width="4" height="{tiny}" fill="#0f0"/>',
            f'<rect id="hint-left-inset" x="{ox+12}" y="{oy+20}" width="{tiny}" height="4" fill="#0f0"/>',
            f'<rect id="hint-right-inset" x="{ox+16}" y="{oy+20}" width="{tiny}" height="4" fill="#0f0"/>'] if not prefix else [])

def shadow_elements(ox, oy, tiles, St, Sb, S):
    E = []
    x = ox
    for name, (im, (w, h)) in tiles.items():
        E.append(f'<image id="shadow-{name}" x="{x}" y="{oy}" width="{w}" height="{h}" preserveAspectRatio="none" '
                 f'xlink:href="data:image/png;base64,{b64(im)}"/>')
        x += w + 4
    E.append(f'<rect id="shadow-center" x="{x}" y="{oy}" width="4" height="4" fill="#000" opacity="0.001"/>')
    x += 8
    E += [f'<rect id="shadow-hint-top-margin" x="{x}" y="{oy}" width="2" height="{St}" fill="#f0f"/>',
          f'<rect id="shadow-hint-bottom-margin" x="{x+4}" y="{oy}" width="2" height="{Sb}" fill="#f0f"/>',
          f'<rect id="shadow-hint-left-margin" x="{x+8}" y="{oy}" width="{S}" height="2" fill="#f0f"/>',
          f'<rect id="shadow-hint-right-margin" x="{x+8}" y="{oy+4}" width="{S}" height="2" fill="#f0f"/>']
    return E

def write_svgz(rel, body, w, h):
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}" height="{h}" '
           f'viewBox="0 0 {w} {h}"><defs>{STYLE}</defs>\n' + '\n'.join(body) + '\n</svg>\n')
    path = os.path.join(OUT, rel + '.svgz'); os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'wb') as raw, gzip.GzipFile(fileobj=raw, mode='wb', mtime=0, filename='') as f: f.write(svg.encode())
    with open('/tmp/plasma-preview-' + rel.replace('/', '_') + '.svg', 'w') as f: f.write(svg)

def make(rel, r, op, border_op, margin, S=None, dy=0, layers=None, thick=False, square=False):
    body = []
    E, (fw, fh) = frame_elements('', 0, 0, r, op, border_op, square=square); body += E
    body += mask_elements(fw + 10, 0, r, square=square)
    body += hints('', 2 * fw + 20, 0, margin)
    if thick:
        body.append(f'<rect id="thick-center" x="{2*fw+60}" y="0" width="32" height="32" class="ColorScheme-Background" fill="currentColor" opacity="{op}"/>')
        body += hints('thick', 2 * fw + 100, 0, margin + 4)
    h = max(fh, 40) + 10
    if S:
        tiles, (St, Sb, SS_) = shadow_tiles(r, S, dy, layers)
        body += shadow_elements(0, h, tiles, St, Sb, SS_)
        h += max(t[1][1] for t in tiles.values()) + 10
    write_svgz(rel, body, 2 * fw + 200 + 400, h)

def build():
    # base = compositing without blur; translucent = with blur; solid/opaque = no transparency
    pan_layers = [(10, 3, 0.22), (1.5, 0, 0.10)]
    dlg_layers = [(14, 6, 0.28), (2, 1, 0.12)]
    tip_layers = [(8, 3, 0.22), (1.5, 0, 0.10)]
    for sub, opp, opd, opt in (('', 0.95, 0.97, 0.97), ('translucent/', 0.72, 0.80, 0.86), ('solid/', 1, 1, 1)):
        make(sub + 'widgets/panel-background', 16, opp, 0.10, 4, 18, 4, pan_layers, thick=True)
        make(sub + 'dialogs/background', 12, opd, 0.12, 6, 26, 6, dlg_layers)
        make(sub + 'widgets/tooltip', 8, opt, 0.12, 4, 14, 3, tip_layers)
    for rel, m in (('opaque/widgets/panel-background', 4), ('opaque/dialogs/background', 6), ('opaque/widgets/tooltip', 4)):
        make(rel, 4, 1, 0.18, m, square=True)
    meta = {"KPlugin": {"Authors": [{"Name": "Vela"}], "Id": "vela", "License": "LGPL-2.1-or-later",
                         "Name": "Vela", "Name[it]": "Vela", "Description": "Pannelli in vetro, angoli arrotondati",
                         "Description[it]": "Pannelli in vetro, angoli arrotondati", "Version": "2.0", "EnabledByDefault": True},
            "X-Plasma-API": "5.0"}
    json.dump(meta, open(os.path.join(OUT, 'metadata.json'), 'w'), indent=2, ensure_ascii=False)
    open(os.path.join(OUT, 'plasmarc'), 'w').write(
        "[Wallpaper]\ndefaultWallpaperTheme=Vela\ndefaultFileSuffix=.jpg\ndefaultWidth=3840\ndefaultHeight=2160\n\n"
        "[ContrastEffect]\nenabled=true\ncontrast=0.25\nintensity=1.0\nsaturation=1.6\n\n[AdaptiveTransparency]\nenabled=true\n\n"
        "[BlurBehindEffect]\nenabled=true\n")
    print('wrote', OUT)

build()
