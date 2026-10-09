#!/usr/bin/env python3
"""Generate the Vela Aurorae window decorations (light 'Vela' and dark 'VelaScuro').

Title bar 34 px, top corners 12 px, macOS-class soft shadow (pre-rendered, embedded as PNG
inside the 9-slice frame), hairline outline, original circular buttons on the left
(close coral, minimize sand, maximize sea-green; glyph on hover; grey when inactive).
"""
import base64, io, json, os
import numpy as np
from PIL import Image, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
T = json.load(open(os.path.join(HERE, '..', '..', 'tokens.json')))
OUTROOT = os.path.join(HERE, '..', '..', 'overlay', 'usr', 'share', 'aurorae', 'themes')

PL = PR = 40; PT = 26; PB = 52        # shadow padding
TH = 34                               # title bar height (6 + 22 + 6)
R = 12                                # corner radius
SS = 2                                # raster supersampling for the shadow
WW, WH = 360, 240                     # reference window for slicing

def rgba(h, a=1.0):
    h = h.lstrip('#'); return 'rgba(%d,%d,%d,%.3f)' % (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)

def shadow_canvas(layers):
    """layers: list of (sigma, dy, alpha). Returns RGBA uint8 array (2x) of the shadow around window."""
    cw, ch = (PL + WW + PR) * SS, (PT + WH + PB) * SS
    acc = np.zeros((ch, cw), np.float32)
    for sigma, dy, alpha in layers:
        m = Image.new('L', (cw, ch), 0)
        from PIL import ImageDraw
        d = ImageDraw.Draw(m)
        x0, y0 = PL * SS, (PT + dy) * SS
        d.rounded_rectangle([x0, y0, x0 + WW * SS - 1, y0 + WH * SS - 1], radius=R * SS, fill=255)
        m = m.filter(ImageFilter.GaussianBlur(sigma * SS))
        a = np.asarray(m, np.float32) / 255 * alpha
        acc = 1 - (1 - acc) * (1 - a)
    return acc

def png_b64(alpha, color):
    h = color.lstrip('#'); r, g, b = (int(h[i:i+2], 16) for i in (0, 2, 4))
    arr = np.zeros(alpha.shape + (4,), np.uint8)
    arr[..., 0], arr[..., 1], arr[..., 2] = r, g, b
    arr[..., 3] = np.clip(alpha * 255 + 0.5, 0, 255).astype(np.uint8)
    buf = io.BytesIO(); Image.fromarray(arr, 'RGBA').save(buf, 'PNG', optimize=True)
    return base64.b64encode(buf.getvalue()).decode()

def compose_canvas(sh, scol, bar, outline, highlight, center_fill):
    """Full reference window (shadow + title bar + outline) rasterised at SSx by rsvg-convert."""
    import subprocess, tempfile
    cw, ch = PL + WW + PR, PT + WH + PB
    x0, y0 = PL, PT
    tb = (f'M{x0} {y0+TH} V{y0+R} A{R} {R} 0 0 1 {x0+R} {y0} H{x0+WW-R} A{R} {R} 0 0 1 {x0+WW} {y0+R} V{y0+TH} Z')
    hl = (f'M{x0+0.5} {y0+R+0.5} A{R-0.5} {R-0.5} 0 0 1 {x0+R} {y0+0.5} H{x0+WW-R} A{R-0.5} {R-0.5} 0 0 1 {x0+WW-0.5} {y0+R+0.5}')
    ol = (f'M{x0-0.5} {y0+WH+0.5} V{y0+R} A{R+0.5} {R+0.5} 0 0 1 {x0+R} {y0-0.5} H{x0+WW-R} '
          f'A{R+0.5} {R+0.5} 0 0 1 {x0+WW+0.5} {y0+R} V{y0+WH+0.5} Z')
    svg = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{cw}" height="{ch}" viewBox="0 0 {cw} {ch}">'
           f'<image x="0" y="0" width="{cw}" height="{ch}" preserveAspectRatio="none" href="data:image/png;base64,{png_b64(sh, scol)}"/>'
           f'<rect x="{x0}" y="{y0+TH}" width="{WW}" height="{WH-TH}" fill="{center_fill}"/>'
           f'<path d="{tb}" fill="{bar}"/>'
           f'<path d="{hl}" fill="none" stroke="{highlight}" stroke-width="1"/>'
           f'<path d="{ol}" fill="none" stroke="{outline}" stroke-width="1"/></svg>')
    with tempfile.TemporaryDirectory() as td:
        open(os.path.join(td, 'c.svg'), 'w').write(svg)
        subprocess.run(['rsvg-convert', '-z', str(SS), os.path.join(td, 'c.svg'), '-o', os.path.join(td, 'c.png')], check=True)
        return Image.open(os.path.join(td, 'c.png')).convert('RGBA')

def png_rgba_b64(im):
    buf = io.BytesIO(); im.save(buf, 'PNG', optimize=True); return base64.b64encode(buf.getvalue()).decode()

def frame(prefix, ox, oy, canvas):
    """Slice the canvas into the 9 FrameSvg elements, laid out without overlap at offset (ox, oy)."""
    tiles = {  # name: (x, y, w, h) in canvas units (1x)
        'topleft': (0, 0, PL + R, PT + TH),
        'top': (PL + R + 40, 0, 4, PT + TH),
        'topright': (PL + WW - R, 0, PR + R, PT + TH),
        'left': (0, PT + TH + 40, PL + R, 4),
        'center': (PL + R + 40, PT + TH + 40, 4, 4),
        'right': (PL + WW - R, PT + TH + 40, PR + R, 4),
        'bottomleft': (0, PT + WH, PL + R, PB),
        'bottom': (PL + R + 40, PT + WH, 4, PB),
        'bottomright': (PL + WW - R, PT + WH, PR + R, PB),
    }
    col_x = {'left': 0, 'mid': PL + R + 10, 'right': PL + R + 10 + 14}
    row_y = {'top': 0, 'mid': PT + TH + 10, 'bot': PT + TH + 10 + 14}
    place = {'topleft': ('left', 'top'), 'top': ('mid', 'top'), 'topright': ('right', 'top'),
             'left': ('left', 'mid'), 'center': ('mid', 'mid'), 'right': ('right', 'mid'),
             'bottomleft': ('left', 'bot'), 'bottom': ('mid', 'bot'), 'bottomright': ('right', 'bot')}
    out = []
    for name, (x, y, w, h) in tiles.items():
        px = ox + col_x[place[name][0]]; py = oy + row_y[place[name][1]]
        crop = canvas.crop((x * SS, y * SS, (x + w) * SS, (y + h) * SS))
        out.append(f'<image id="{prefix}-{name}" x="{px}" y="{py}" width="{w}" height="{h}" '
                   f'preserveAspectRatio="none" xlink:href="data:image/png;base64,{png_rgba_b64(crop)}"/>')
    return '\n'.join(out), (col_x['right'] + PR + R, row_y['bot'] + PB)

def decoration_svg(c, dark):
    act = shadow_canvas([(20, 14, 0.30 if not dark else 0.55), (3, 1, 0.16 if not dark else 0.35)])
    ina = shadow_canvas([(12, 8, 0.16 if not dark else 0.40), (2, 1, 0.10 if not dark else 0.30)])
    scol = T['brand']['marine900'] if not dark else '#000000'
    outline = rgba('#071C27', 0.13) if not dark else rgba('#FFFFFF', 0.10)
    outline_i = rgba('#071C27', 0.09) if not dark else rgba('#FFFFFF', 0.07)
    hl = rgba('#FFFFFF', 0.65) if not dark else rgba('#FFFFFF', 0.07)
    a, size = frame('decoration', 0, 0, compose_canvas(act, scol, c['header'], outline, hl, c['window']))
    i, _ = frame('decoration-inactive', 0, size[1] + 20, compose_canvas(ina, scol, c['headerInactive'], outline_i, hl, c['window']))
    my = 2 * (size[1] + 20)
    maxi = (f'<rect id="decoration-maximized-center" x="0" y="{my}" width="4" height="{TH}" fill="{c["header"]}"/>\n'
            f'<rect id="decoration-maximized-inactive-center" x="10" y="{my}" width="4" height="{TH}" fill="{c["headerInactive"]}"/>')
    W = size[0] + 10; H = my + TH + 10
    return (f'<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
            f'width="{W}" height="{H}" viewBox="0 0 {W} {H}">\n<defs/>\n{a}\n{i}\n{maxi}\n</svg>\n')

BTN = 20      # button box (theme units); circle 12 => 8 px gap between circles
def button_svg(kind, c, dark):
    wb = T['windowButtons']
    base = {'close': wb['close'], 'minimize': wb['minimize'], 'maximize': wb['maximize'], 'restore': wb['maximize']}[kind]
    inactive = wb['inactive'] if not dark else wb['inactiveDark']
    glyph_col = {'close': '#6E2214', 'minimize': '#6A4A10', 'maximize': '#1E4B37', 'restore': '#1E4B37'}[kind]
    ring = rgba('#000000', 0.16 if not dark else 0.30)
    cx = cy = BTN / 2; r = 6
    def glyph(col, op):
        s = f'stroke="{col}" stroke-opacity="{op}" stroke-width="1.4" stroke-linecap="round" fill="none"'
        if kind == 'close':
            return f'<path d="M{cx-2.6} {cy-2.6} L{cx+2.6} {cy+2.6} M{cx+2.6} {cy-2.6} L{cx-2.6} {cy+2.6}" {s}/>'
        if kind == 'minimize':
            return f'<path d="M{cx-3.2} {cy} H{cx+3.2}" {s}/>'
        if kind == 'maximize':   # two small corner triangles = enlarge
            return (f'<path d="M{cx-3.3} {cy+0.9} V{cy-3.3} H{cx+0.9} Z M{cx+3.3} {cy-0.9} V{cy+3.3} H{cx-0.9} Z" '
                    f'fill="{col}" fill-opacity="{op}"/>')
        return (f'<path d="M{cx-0.6} {cy-3.6} V{cy-0.6} H{cx-3.6} Z M{cx+0.6} {cy+3.6} V{cy+0.6} H{cx+3.6} Z" '
                f'fill="{col}" fill-opacity="{op}"/>')
    states = []
    def state(name, x, fill, show_glyph=False, gop=0.75, ring_col=ring, extra=''):
        el = [f'<g id="{name}-center">', f'<rect x="{x}" y="0" width="{BTN}" height="{BTN}" fill="none"/>',
              f'<g transform="translate({x} 0)">',
              f'<circle cx="{cx}" cy="{cy}" r="{r}" fill="{fill}"/>',
              f'<circle cx="{cx}" cy="{cy}" r="{r-0.35}" fill="none" stroke="{ring_col}" stroke-width="0.7"/>', extra]
        if show_glyph: el.append(glyph(glyph_col, gop))
        el += ['</g>', '</g>']
        states.append(''.join(el))
    def darker(h, f):
        h = h.lstrip('#'); return '#' + ''.join('%02X' % int(int(h[i:i+2], 16) * f) for i in (0, 2, 4))
    state('active', 0, base)
    state('hover', 30, base, True)
    state('pressed', 60, darker(base, 0.82), True, 0.9)
    state('inactive', 90, inactive, ring_col=rgba('#000000', 0.08 if not dark else 0.25))
    state('hover-inactive', 120, base, True)
    state('pressed-inactive', 150, darker(base, 0.82), True, 0.9)
    state('deactivated', 180, 'none', ring_col=rgba('#000000', 0.25) if not dark else rgba('#FFFFFF', 0.25))
    state('deactivated-inactive', 210, 'none', ring_col=rgba('#000000', 0.15) if not dark else rgba('#FFFFFF', 0.15))
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{240}" height="{BTN}" viewBox="0 0 240 {BTN}">\n'
            + '\n'.join(states) + '\n</svg>\n')

def rgbtxt(h, a=255):
    h = h.lstrip('#'); return '%d,%d,%d,%d' % (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)

def write_theme(name, label_it, label_en, dark):
    c = T['dark' if dark else 'light']
    d = os.path.join(OUTROOT, name); os.makedirs(d, exist_ok=True)
    open(os.path.join(d, 'decoration.svg'), 'w').write(decoration_svg(c, dark))
    for k in ('close', 'minimize', 'maximize', 'restore'):
        open(os.path.join(d, k + '.svg'), 'w').write(button_svg(k, c, dark))
    rc = f"""[General]
TitleAlignment=Center
TitleVerticalAlignment=Center
Animation=120
ActiveTextColor={rgbtxt(c['text'])}
InactiveTextColor={rgbtxt(c['textSecondary'])}
UseTextShadow=false
HaloActive=false
HaloInactive=false
LeftButtons=XIA
RightButtons=
Shadow=true
DecorationPosition=0

[Layout]
BorderLeft=0
BorderRight=0
BorderBottom=0
BorderTop=0
BorderLeftMaximized=0
BorderRightMaximized=0
BorderBottomMaximized=0
TitleEdgeTop=6
TitleEdgeBottom=6
TitleEdgeLeft=10
TitleEdgeRight=10
TitleEdgeTopMaximized=6
TitleEdgeBottomMaximized=6
TitleEdgeLeftMaximized=10
TitleEdgeRightMaximized=10
TitleBorderLeft=12
TitleBorderRight=12
TitleHeight=22
ButtonWidth={BTN}
ButtonHeight={BTN}
ButtonSpacing=0
ButtonMarginTop=1
ExplicitButtonSpacer=8
PaddingTop={PT}
PaddingBottom={PB}
PaddingLeft={PL}
PaddingRight={PR}
"""
    open(os.path.join(d, name + 'rc'), 'w').write(rc)
    md = f"""[Desktop Entry]
Name={label_en}
Name[it]={label_it}
Comment=Vela window decoration
Comment[it]=Bordi finestra Vela
X-KDE-PluginInfo-Name={name}
X-KDE-PluginInfo-Author=Vela
X-KDE-PluginInfo-Version=2.0
X-KDE-PluginInfo-License=GPL-2.0-or-later
X-KDE-PluginInfo-Website=https://vela.invalid/
"""
    open(os.path.join(d, 'metadata.desktop'), 'w').write(md)
    print('wrote', d)

write_theme('VelaLight', 'Vela chiaro', 'Vela Light', False)
write_theme('VelaDark', 'Vela scuro', 'Vela Dark', True)
