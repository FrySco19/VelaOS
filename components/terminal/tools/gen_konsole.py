#!/usr/bin/env python3
"""Generate the Konsole colour schemes and profiles from vela_terminal.theme.TERM,
so Konsole and the Vela AI Terminal always share the same palette.

    python3 components/terminal/tools/gen_konsole.py      (rewrites overlay/usr/share/konsole/*)
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
COMP = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(COMP, "overlay/usr/lib/vela/terminal"))
from vela_terminal.theme import TERM  # noqa: E402

OUT = os.path.join(COMP, "overlay/usr/share/konsole")
FONT = "JetBrains Mono,10,-1,2,400,0,0,0,0,0,0,0,0,0,0,1"
NAMES = {"light": ("VelaLight", "Vela chiaro", "Vela light"),
         "dark": ("VelaDark", "Vela scuro", "Vela dark")}


def rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))


def mix(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def c(t):
    return ",".join(str(v) for v in t)


def scheme(mode):
    p = TERM[mode]
    bg, fg = rgb(p["bg"]), rgb(p["fg"])
    ansi = [rgb(x) for x in p["ansi"]]
    out = []

    def sec(name, col):
        out.append(f"[{name}]\nColor={c(col)}\n")

    sec("Background", bg)
    sec("BackgroundFaint", bg)
    sec("BackgroundIntense", bg)
    for i in range(8):
        sec(f"Color{i}", ansi[i])
        sec(f"Color{i}Faint", mix(ansi[i], bg, 0.45))
        sec(f"Color{i}Intense", ansi[i + 8])
    sec("Foreground", fg)
    sec("ForegroundFaint", mix(fg, bg, 0.45))
    sec("ForegroundIntense", fg)
    sid, it, en = NAMES[mode]
    out.append("[General]\nBlur=false\nColorRandomization=false\n"
               f"Description={it}\nDescription[it]={it}\nDescription[en]={en}\n"
               "FillStyle=Tile\nOpacity=1\nWallpaper=\nWallpaperFlipType=NoFlip\nWallpaperOpacity=1\n")
    return "\n".join(out)


def profile(mode):
    p = TERM[mode]
    sid, it, _en = NAMES[mode]
    return f"""[Appearance]
ColorScheme={sid}
Font={FONT}
LineSpacing=1
UseFontLineCharacters=false

[Cursor Options]
CursorShape=1
CustomCursorColor={c(rgb(p["cursor"]))}
CustomCursorTextColor={c(rgb(p["bg"]))}
UseCustomCursorColor=true

[General]
DimWhenInactive=false
LocalTabTitleFormat=%d
Name={it}
Parent=FALLBACK/
RemoteTabTitleFormat=%H
SemanticHints=1
SemanticInputClick=true
SemanticUpDown=true
ShowTerminalSizeHint=false
TerminalCenter=false
TerminalColumns=100
TerminalMargin=12
TerminalRows=28

[Interaction Options]
AutoCopySelectedText=false
CopyTextAsHTML=false
MiddleClickPasteMode=0
OpenLinksByDirectClickEnabled=false
TrimLeadingSpacesInSelectedText=false
TrimTrailingSpacesInSelectedText=true
UnderlineFilesEnabled=false
UnderlineLinksEnabled=true

[Keyboard]
KeyBindings=default

[Scrolling]
HighlightScrolledLines=false
HistoryMode=1
HistorySize=10000
ReflowLines=true
ScrollBarPosition=1

[Terminal Features]
BellMode=3
BlinkingCursorEnabled=true
"""


def main():
    os.makedirs(OUT, exist_ok=True)
    for mode in ("light", "dark"):
        sid = NAMES[mode][0]
        with open(os.path.join(OUT, sid + ".colorscheme"), "w") as f:
            f.write(scheme(mode))
        with open(os.path.join(OUT, sid + ".profile"), "w") as f:
            f.write(profile(mode))
    print("wrote", OUT)


if __name__ == "__main__":
    main()
