#!/usr/bin/env python3
"""Original Vela app icons (SVG). Rounded-square tile, marine gradient, white line glyphs, coral accent."""
import os
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', '..', 'overlay', 'usr', 'share', 'icons', 'hicolor', 'scalable', 'apps')
os.makedirs(OUT, exist_ok=True)
CORAL = '#E68B75'

def tile(gid, top, bottom):
    return f'''<defs>
  <linearGradient id="{gid}" x1="0" y1="0" x2="0" y2="1">
   <stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/>
  </linearGradient>
  <linearGradient id="{gid}h" x1="0" y1="0" x2="0" y2="1">
   <stop offset="0" stop-color="#fff" stop-opacity=".28"/><stop offset=".5" stop-color="#fff" stop-opacity="0"/>
  </linearGradient>
 </defs>
 <rect x="6" y="8" width="116" height="116" rx="28" fill="#071C27" opacity=".18"/>
 <rect x="6" y="6" width="116" height="116" rx="28" fill="url(#{gid})"/>
 <rect x="6.5" y="6.5" width="115" height="115" rx="27.5" fill="url(#{gid}h)" stroke="#fff" stroke-opacity=".18"/>'''

def svg(body, size=128):
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}">\n {body}\n</svg>\n'

S = 'fill="none" stroke="#fff" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"'
SPARK = lambda cx, cy, r, col=CORAL: (f'<path d="M{cx} {cy-r} C{cx+r*0.18} {cy-r*0.18} {cx+r*0.18} {cy-r*0.18} {cx+r} {cy} '
                                      f'C{cx+r*0.18} {cy+r*0.18} {cx+r*0.18} {cy+r*0.18} {cx} {cy+r} C{cx-r*0.18} {cy+r*0.18} {cx-r*0.18} {cy+r*0.18} {cx-r} {cy} '
                                      f'C{cx-r*0.18} {cy-r*0.18} {cx-r*0.18} {cy-r*0.18} {cx} {cy-r}Z" fill="{col}"/>')
# sail mark: main sail (curved luff) + jib, in a 0..100 box
def sail_mark(x, y, s, main='#fff', jib=CORAL, op=1):
    k = s / 100
    def P(px, py): return f'{x+px*k:.2f} {y+py*k:.2f}'
    m = f'<path d="M{P(46,4)} C{P(24,30)} {P(14,62)} {P(12,90)} L{P(46,90)} Z" fill="{main}" opacity="{op}"/>'
    j = f'<path d="M{P(54,16)} C{P(70,38)} {P(82,64)} {P(88,90)} L{P(54,90)} Z" fill="{jib}"/>'
    hull = f'<path d="M{P(8,94)} L{P(92,94)} C{P(86,99)} {P(78,100)} {P(70,100)} L{P(30,100)} C{P(22,100)} {P(14,99)} {P(8,94)} Z" fill="{main}" opacity="{op}"/>'
    return m + j + hull

icons = {}
icons['vela-logo'] = svg(f'''<defs><linearGradient id="m" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#2F7584"/><stop offset="1" stop-color="#123B4D"/></linearGradient></defs>
 {sail_mark(14, 10, 100, main="url(#m)")}''')
icons['vela-logo-symbolic'] = '''<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16">
 <style id="current-color-scheme" type="text/css">.ColorScheme-Text { color:#15303A; }</style>
 <path class="ColorScheme-Text" fill="currentColor" d="M7.4 0.8C4.3 4.6 2.9 8.9 2.7 13H7.4Z"/>
 <path class="ColorScheme-Text" fill="currentColor" opacity=".72" d="M8.6 2.6C10.9 5.6 12.5 9 13.3 13H8.6Z"/>
 <path class="ColorScheme-Text" fill="currentColor" d="M1.6 13.8H14.4C13.8 14.7 13 15.2 12 15.2H4C3 15.2 2.2 14.7 1.6 13.8Z"/>
</svg>
'''
icons['vela-welcome'] = svg(tile('g', '#F2FAF9', '#BFE0E0') + sail_mark(26, 22, 76, main='#1C5468'))
icons['vela-assistant'] = svg(tile('g', '#2F7584', '#0F3345') + SPARK(64, 64, 34, '#fff') + SPARK(94, 34, 12))
icons['vela-terminal-ai'] = svg(tile('g', '#1D3F4E', '#0A1E28') +
    f'<path {S} d="M30 46 L48 62 L30 78"/><path {S} d="M56 82 H80"/>' + SPARK(92, 40, 14))
icons['vela-browser-ai'] = svg(tile('g', '#3D8B98', '#164A5B') +
    f'<circle cx="64" cy="64" r="34" {S}/><path d="M78 50 L70 70 L50 78 L58 58 Z" fill="#fff"/><circle cx="64" cy="64" r="4" fill="#164A5B"/>' + SPARK(96, 32, 13))
icons['vela-spazi'] = svg(tile('g', '#4E9AA6', '#1C5468') +
    '<rect x="26" y="34" width="50" height="38" rx="8" fill="#fff" opacity=".45"/>'
    '<rect x="40" y="46" width="50" height="38" rx="8" fill="#fff" opacity=".7"/>'
    f'<rect x="54" y="58" width="50" height="38" rx="8" fill="{CORAL}"/>')
icons['vela-memoria'] = svg(tile('g', '#2F7584', '#123B4D') +
    '<rect x="30" y="30" width="56" height="16" rx="6" fill="#fff" opacity=".5"/>'
    '<rect x="30" y="52" width="56" height="16" rx="6" fill="#fff" opacity=".75"/>'
    '<rect x="30" y="74" width="40" height="16" rx="6" fill="#fff"/>'
    f'<circle cx="84" cy="84" r="13" fill="none" stroke="{CORAL}" stroke-width="7"/><path d="M93 93 L102 102" stroke="{CORAL}" stroke-width="7" stroke-linecap="round"/>')
icons['vela-privacy'] = svg(tile('g', '#2A6B79', '#0E3040') +
    f'<path d="M64 26 L94 38 V60 C94 80 81 95 64 102 C47 95 34 80 34 60 V38 Z" fill="#fff"/>'
    f'<circle cx="64" cy="60" r="9" fill="{CORAL}"/><path d="M64 64 V80" stroke="{CORAL}" stroke-width="8" stroke-linecap="round"/>')
gear = []
import math
icons['vela-help'] = svg(tile('g', '#3D8B98', '#164A5B') +
    '<circle cx="64" cy="64" r="36" fill="none" stroke="#fff" stroke-width="7"/>'
    '<path d="M52 53 C52 45 57.5 40 64.5 40 C71.5 40 77 45 77 51.5 C77 57.5 73 60.5 69 63 C66 65 64.5 67 64.5 71" fill="none" stroke="#fff" stroke-width="7.5" stroke-linecap="round"/>'
    f'<circle cx="64.5" cy="85" r="5" fill="{CORAL}"/>')

for i in range(8):
    a = i * math.pi / 4
    gear.append(f'<rect x="58" y="22" width="12" height="18" rx="4" fill="#fff" transform="rotate({i*45} 64 64)"/>')
icons['vela-settings-ai'] = svg(tile('g', '#5A6E78', '#26363F') + ''.join(gear) +
    '<circle cx="64" cy="64" r="28" fill="#fff"/><circle cx="64" cy="64" r="16" fill="#26363F"/>' + SPARK(64, 64, 10))
icons['vela-snapshots'] = svg(tile('g', '#3D8B98', '#164A5B') +
    f'<path {S} d="M38 64 A26 26 0 1 0 46 45"/><path d="M34 34 L48 46 L32 52 Z" fill="#fff"/>'
    f'<path d="M64 50 V66 L76 74" fill="none" stroke="{CORAL}" stroke-width="7" stroke-linecap="round" stroke-linejoin="round"/>')

for name, data in icons.items():
    open(os.path.join(OUT, name + '.svg'), 'w').write(data)
print('wrote', len(icons), 'icons to', OUT)

# tray/monochrome variant of the assistant (recolored by Plasma via ColorScheme-Text)
open(os.path.join(OUT, 'vela-assistant-symbolic.svg'), 'w').write('''<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16">
 <style id="current-color-scheme" type="text/css">.ColorScheme-Text { color:#15303A; }</style>
 <path class="ColorScheme-Text" fill="currentColor" d="M7 1.5C7.4 4.6 8.4 5.6 11.5 6 8.4 6.4 7.4 7.4 7 10.5 6.6 7.4 5.6 6.4 2.5 6 5.6 5.6 6.6 4.6 7 1.5Z"/>
 <path class="ColorScheme-Text" fill="currentColor" opacity=".7" d="M12 9.5C12.2 11 12.9 11.8 14.5 12 12.9 12.2 12.2 13 12 14.5 11.8 13 11.1 12.2 9.5 12 11.1 11.8 11.8 11 12 9.5Z"/>
</svg>
''')
