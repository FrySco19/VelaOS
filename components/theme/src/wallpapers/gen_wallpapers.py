#!/usr/bin/env python3
"""Procedural Vela wallpapers (original artwork, generated from code).

Usage: gen_wallpapers.py OUTDIR [WIDTH HEIGHT]
Writes Plasma wallpaper packages OUTDIR/<Name>/{metadata.json,contents/images/WxH.jpg,
contents/images_dark/WxH.jpg,contents/screenshot.jpg}. Deterministic (fixed seeds).
Needs python3-numpy and python3-pil.
"""
import json, os, sys
import numpy as np
from PIL import Image

W = int(sys.argv[2]) if len(sys.argv) > 2 else 3840
H = int(sys.argv[3]) if len(sys.argv) > 3 else 2160
OUT = sys.argv[1] if len(sys.argv) > 1 else 'out'
F = np.float32
A = W / H
yy, xx = np.mgrid[0:H, 0:W].astype(F)
X = xx / H          # 0..A   (aspect-correct)
Y = yy / H          # 0..1
PX = 1.0 / H        # one pixel in these units

def hexc(h):
    h = h.lstrip('#'); return np.array([int(h[i:i+2], 16) / 255 for i in (0, 2, 4)], F)

def srgb2lin(c): return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4).astype(F)
def lin2srgb(c): return np.where(c <= 0.0031308, c * 12.92, 1.055 * np.power(np.clip(c, 0, None), 1 / 2.4) - 0.055).astype(F)

def L(h): return srgb2lin(hexc(h))   # work in linear light

def smooth(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0, 1); return (t * t * (3 - 2 * t)).astype(F)

def grad(t, stops):
    """t array -> RGB via list of (pos, hex) stops (linear light)."""
    t = np.clip(t, 0, 1)
    out = np.zeros(t.shape + (3,), F)
    ps = [p for p, _ in stops]; cs = [L(c) for _, c in stops]
    for i in range(3):
        out[..., i] = np.interp(t, ps, [c[i] for c in cs]).astype(F)
    return out

def over(base, col, a):
    a = a[..., None]; return base * (1 - a) + col * a

def add_glow(img, cx, cy, r, col, strength, aspect=1.0):
    d2 = ((X - cx) / aspect) ** 2 + (Y - cy) ** 2
    g = np.exp(-d2 / (r * r)).astype(F) * strength
    return img + L(col)[None, None, :] * g[..., None]

def lowfreq_noise(seed, cells=6, octaves=4):
    rng = np.random.default_rng(seed); acc = np.zeros((H, W), F); amp = 1.0; tot = 0
    for o in range(octaves):
        n = cells * (2 ** o)
        g = rng.random((n + 3, int(n * A) + 3)).astype(F)
        im = Image.fromarray((g * 65535).astype(np.uint16).astype(np.int32), 'I').resize((W, H), Image.BICUBIC)
        acc += np.asarray(im, F) / 65535 * amp; tot += amp; amp *= 0.5
    acc /= tot
    return (acc - acc.min()) / (acc.max() - acc.min())

def finish(img, path, seed=1):
    s = lin2srgb(np.clip(img, 0, 1)) * 255
    rng = np.random.default_rng(seed)
    s += (rng.random(s.shape[:2], dtype=np.float32) - rng.random(s.shape[:2], dtype=np.float32))[..., None] * 0.9  # TPDF dither vs banding
    im = Image.fromarray(np.clip(s + 0.5, 0, 255).astype(np.uint8), 'RGB')
    os.makedirs(os.path.dirname(path), exist_ok=True)
    im.save(path, quality=90, subsampling=0, optimize=True, progressive=True)
    return im

# ---------------------------------------------------------------- designs
def sail(img, apex, bl, br, bulge, cols, alpha, rim=None, rimS=0.0, shadow=0.0, belly_s=0.10):
    """Abstract curved sail from apex (x,y) down to the bottom edge between bl..br (x at y=1.12)."""
    xa, ya = apex; yb = 1.12
    t = np.clip((Y - ya) / (yb - ya), 0, 1)
    xl = xa + (bl - xa) * t - bulge * np.sin(np.pi * t * 0.9)
    xr = xa + (br - xa) * t ** 1.25 + 0.25 * bulge * np.sin(np.pi * t)
    wdt = np.maximum(xr - xl, 1e-4)
    u = (X - xl) / wdt
    aa = 1.3 * PX
    inside = smooth(-aa, aa, X - xl) * smooth(-aa, aa, xr - X) * smooth(-aa, aa, Y - ya)
    if shadow:
        sh = smooth(-0.06, 0.0, X - xl - 0.02) * smooth(-0.06, 0.0, xr + 0.02 - X) * smooth(-0.04, 0, Y - ya) * (1 - inside)
        img = img * (1 - shadow * sh)[..., None]
    col = grad(np.clip(u, 0, 1), cols)
    col = col * (1 - 0.22 * t)[..., None]                       # deeper toward the foot
    belly = np.exp(-((u - 0.30) / 0.22) ** 2).astype(F) * belly_s  # billow highlight (tinted)
    col = col * (1 + belly[..., None] * 1.6)
    img = over(img, col, inside * alpha)
    if rim is not None:
        rl = np.exp(-((X - xl) / (0.0022 + 0 * PX)) ** 2).astype(F) * smooth(0, 0.02, Y - ya)
        img = img + L(rim)[None, None, :] * (rl * rimS)[..., None]
    return img

def vela(dark):
    """Default: abstract sails (main + coral jib) on a calm sea-sky gradient."""
    t = (X / A * 0.35 + Y * 0.65)
    if not dark:
        img = grad(t, [(0, '#EAF5F4'), (0.55, '#BADCDC'), (1, '#6FA8B0')])
        img = add_glow(img, 0.86 * A, 0.06, 0.60, '#FFE0D0', 0.30)
        img = sail(img, (0.70 * A, -0.35), 0.40 * A, 1.10 * A, 0.16, [(0, '#6FB0B7'), (0.45, '#2F7584'), (1, '#174C5D')], 0.97, '#EFFFFC', 0.40, 0.12)
        img = sail(img, (0.97 * A, -0.30), 0.80 * A, 1.40 * A, 0.10, [(0, '#F6C3B2'), (0.5, '#E68B75'), (1, '#C25E48')], 0.95, '#FFF4EE', 0.35, 0.16)
    else:
        img = grad(t, [(0, '#173F4C'), (0.55, '#0C2836'), (1, '#04121A')])
        img = add_glow(img, 0.86 * A, 0.06, 0.60, '#2F7C86', 0.25)
        img = sail(img, (0.70 * A, -0.35), 0.40 * A, 1.10 * A, 0.16, [(0, '#22606F'), (0.45, '#123E4D'), (1, '#0A2532')], 0.97, '#8FDCD2', 0.28, 0.30)
        img = sail(img, (0.97 * A, -0.30), 0.80 * A, 1.40 * A, 0.10, [(0, '#C98572'), (0.5, '#A65A47'), (1, '#6E352A')], 0.93, '#FFC2B0', 0.22, 0.30)
    vign = 1 - 0.20 * smooth(0.45, 1.3, np.sqrt(((X - 0.5 * A) / A) ** 2 * 1.6 + (Y - 0.45) ** 2))
    return img * vign[..., None]

def maestrale(dark):
    """Layered wind-waves, each a sine band darker with depth."""
    if not dark:
        img = grad(Y, [(0, '#F3E6DD'), (0.35, '#D8ECEC'), (1, '#9CCBCF')])
        img = add_glow(img, 0.25 * A, 0.16, 0.40, '#FFD2BE', 0.16)
        cols = ['#BFE0DF', '#97CACC', '#6EAFB6', '#4C939D', '#2F7584', '#1A586A']
        rim = '#FFFFFF'
    else:
        img = grad(Y, [(0, '#0B1C27'), (0.4, '#10303E'), (1, '#1A4A57')])
        img = add_glow(img, 0.25 * A, 0.14, 0.40, '#2F7C86', 0.10)
        cols = ['#173F4D', '#133846', '#0F303D', '#0B2733', '#081F29', '#05161E']
        rim = '#7FD0C8'
    n = len(cols)
    for i, c in enumerate(cols):
        base = 0.42 + i * 0.10
        amp = 0.018 + 0.006 * i
        yc = base + amp * np.sin(X * (2.1 - 0.15 * i) + i * 1.7) + 0.5 * amp * np.sin(X * (4.3 + 0.3 * i) + i)
        d = Y - yc
        m = smooth(-1.2 * PX, 1.2 * PX, d)
        lc = grad(np.clip(d * 3, 0, 1), [(0, c), (1, '#000000' if dark else c)]) if dark else L(c)[None, None, :] * (1 - 0.10 * smooth(0, 0.25, d))[..., None]
        img = over(img, lc, m)
        img = img + L(rim)[None, None, :] * (np.exp(-(d / 0.0018) ** 2) * (0.14 if not dark else 0.07))[..., None]
        img = img * (1 - (np.exp(-np.clip(-d, 0, None) / 0.02) * (1 - m) * 0.08))[..., None]
    return img

def fondale(dark):
    """Deep water: smooth caustic-like noise + light shafts from the surface."""
    nz = lowfreq_noise(7, cells=3, octaves=4)
    if not dark:
        img = grad(Y * 0.92 + nz * 0.12, [(0, '#D6F0EE'), (0.4, '#7FBFC4'), (0.75, '#2F7887'), (1, '#174E60')])
        ray = '#FFFFFF'; rs = 0.16
    else:
        img = grad(Y * 0.92 + nz * 0.12, [(0, '#1F5866'), (0.35, '#0F3443'), (0.7, '#071E2A'), (1, '#030D13')])
        ray = '#7FD0C8'; rs = 0.08
    for i, (x0, w, s) in enumerate([(0.62, 0.05, 1.0), (0.75, 0.03, 0.7), (0.50, 0.025, 0.6), (0.88, 0.06, 0.5), (0.40, 0.04, 0.35)]):
        u = (X - x0 * A) - (Y * 0.35)               # slanted shafts
        shaft = np.exp(-(u / w) ** 2).astype(F) * np.exp(-Y / 0.55).astype(F) * s
        img = img + L(ray)[None, None, :] * (shaft * rs)[..., None]
    caust = smooth(0.55, 0.9, lowfreq_noise(11, cells=10, octaves=2)) * np.exp(-Y / 0.35)
    img = img + L(ray)[None, None, :] * (caust * rs * 0.25)[..., None]
    return img

def orizzonte(dark):
    """Minimal horizon: sky gradient, sun glow, calm sea with a soft reflection."""
    hz = 0.62
    if not dark:
        sky = grad(Y / hz, [(0, '#BFDDE3'), (0.7, '#F1E4DA'), (1, '#F7D3C2')])
        sea = grad((Y - hz) / (1 - hz), [(0, '#8CC0C6'), (1, '#2E6E7D')])
        sun = '#FFC2A6'; s = 0.55
    else:
        sky = grad(Y / hz, [(0, '#07141C'), (0.75, '#123342'), (1, '#2A5560')])
        sea = grad((Y - hz) / (1 - hz), [(0, '#163E4B'), (1, '#040E14')])
        sun = '#E68B75'; s = 0.30
    m = smooth(hz - PX, hz + PX, Y)
    img = over(sky, sea, m)
    img = add_glow(img, 0.66 * A, hz - 0.02, 0.24, sun, s * 0.55)
    img = add_glow(img, 0.66 * A, hz - 0.01, 0.08, sun, s * 0.22)
    disk = smooth(0.047, 0.043, np.sqrt((X - 0.66 * A) ** 2 + (Y - hz - 0.012) ** 2)) * (1 - m)
    img = over(img, L('#FDE6D6' if not dark else '#F2A48F')[None, None, :] * np.ones_like(img), disk * (0.85 if not dark else 0.80))
    refl = np.exp(-((X - 0.66 * A) / 0.07) ** 2).astype(F) * np.exp(-(Y - hz) / 0.25).astype(F) * m
    ripple = 0.6 + 0.4 * np.sin(Y * 900 / (1 + (Y - hz) * 6))
    img = img + L(sun)[None, None, :] * (refl * ripple * s * 0.25)[..., None]
    return img

DESIGNS = [
    ('Vela', 'Vela', vela),
    ('VelaMaestrale', 'Maestrale', maestrale),
    ('VelaFondale', 'Fondale', fondale),
    ('VelaOrizzonte', 'Orizzonte', orizzonte),
]

def write_pkg(pkg, label, fn):
    base = os.path.join(OUT, pkg)
    for dark, sub in ((False, 'images'), (True, 'images_dark')):
        im = finish(fn(dark), os.path.join(base, 'contents', sub, '%dx%d.jpg' % (W, H)), seed=3 + dark)
        if not dark:
            im.resize((440, 248), Image.LANCZOS).save(os.path.join(base, 'contents', 'screenshot.jpg'), quality=88)
        print(pkg, sub, 'ok', flush=True)
    meta = {"KPlugin": {"Authors": [{"Name": "Vela"}], "Id": pkg, "License": "CC-BY-SA-4.0",
                         "Name": label, "Name[it]": label, "Description": "Sfondo Vela (chiaro e scuro)"}}
    json.dump(meta, open(os.path.join(base, 'metadata.json'), 'w'), indent=2, ensure_ascii=False)

if __name__ == '__main__':
    only = os.environ.get('ONLY')
    for pkg, label, fn in DESIGNS:
        if only and pkg != only: continue
        write_pkg(pkg, label, fn)
