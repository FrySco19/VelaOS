"""Vela design tokens for the terminal apps (source: /usr/share/vela/theme/tokens.json)."""
from __future__ import annotations

import json
import os

TOKENS_PATHS = ("/usr/share/vela/theme/tokens.json",
                os.path.join(os.path.dirname(__file__), "..", "..", "..", "..", "share", "vela", "theme", "tokens.json"))

FALLBACK = {
    "light": {"window": "#F3F6F7", "view": "#FFFFFF", "viewAlt": "#F6F9FA", "header": "#EAF0F2",
              "text": "#15303A", "textSecondary": "#587279", "border": "#C9D6DA", "separator": "#D5E0E3",
              "accent": "#D4624A", "accentHover": "#DC745D", "accentText": "#FFFFFF", "link": "#1C6F86",
              "positive": "#2E8B57", "neutral": "#B7791F", "negative": "#C93A3A", "field": "#FFFFFF"},
    "dark": {"window": "#15232B", "view": "#1A2A33", "viewAlt": "#1E303A", "header": "#1D2E37",
             "text": "#E3EEF0", "textSecondary": "#93AAB1", "border": "#34505C", "separator": "#2A3E48",
             "accent": "#D4624A", "accentHover": "#E0755E", "accentText": "#FFFFFF", "link": "#6FB7C4",
             "positive": "#5CBF8A", "neutral": "#E0B25C", "negative": "#E66A6A", "field": "#13222A"},
    "radius": 12, "font": "Inter", "fontMono": "JetBrains Mono",
}

# Terminal palettes (ANSI 16) designed for the Vela light/dark tokens; same values as the
# Konsole colour schemes shipped in /usr/share/konsole/Vela*.colorscheme.
TERM = {
    "dark": {"bg": "#13222A", "fg": "#E3EEF0", "cursor": "#E8836B", "selection": "#2C5566",
             "ansi": ["#1E303A", "#E66A6A", "#5CBF8A", "#E0B25C", "#6FB7C4", "#C792EA", "#76B0B3", "#C9D6DA",
                      "#4A6470", "#F08A80", "#7FD4A5", "#F0C97A", "#8FCFDB", "#D9AEF2", "#AFD5D6", "#FFFFFF"]},
    "light": {"bg": "#FBFCFC", "fg": "#15303A", "cursor": "#D4624A", "selection": "#CFE5E8",
              "ansi": ["#15303A", "#C93A3A", "#2E8B57", "#9A6510", "#1C6F86", "#8E44AD", "#2C6872", "#C9D6DA",
                       "#587279", "#D9534F", "#38A169", "#B7791F", "#2B8BA8", "#A35DC4", "#397984", "#FFFFFF"]},
}


def load_tokens() -> dict:
    for p in TOKENS_PATHS:
        try:
            with open(p, encoding="utf-8") as f:
                t = json.load(f)
            if "light" in t and "dark" in t:
                return t
        except (OSError, ValueError):
            continue
    return FALLBACK


def is_dark(app=None) -> bool:
    forced = os.environ.get("VELA_TERMINAL_THEME")
    if forced in ("dark", "light"):
        return forced == "dark"
    try:
        from PySide6.QtGui import QGuiApplication
        from PySide6.QtCore import Qt
        hints = QGuiApplication.styleHints()
        cs = hints.colorScheme()
        if cs == Qt.ColorScheme.Dark:
            return True
        if cs == Qt.ColorScheme.Light:
            return False
        pal = QGuiApplication.palette()
        return pal.window().color().lightness() < 128
    except Exception:
        return False


def colors(dark: bool) -> dict:
    t = load_tokens()
    base = dict(FALLBACK["dark" if dark else "light"])
    base.update(t.get("dark" if dark else "light", {}))
    base["radius"] = t.get("radius", 12)
    base["font"] = t.get("font", "Inter")
    base["fontMono"] = t.get("fontMono", "JetBrains Mono")
    return base
