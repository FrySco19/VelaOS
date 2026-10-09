# Vela design tokens

Source of truth: `components/theme/tokens.json` (installed as `/usr/share/vela/theme/tokens.json`).
Keys: `light` / `dark` (semantic colors: window, view, viewAlt, header, sidebar, button, text, textSecondary, separator, border, accent, accentText, link, positive/neutral/negative, glass, tooltip + aliases bg, surface, surface2, muted, field, success, danger, navy, shadow), `brand`, `windowButtons`, `radius` (12, scalar), `radii` (window/card/popup/panel/dock/control/small), `spacing`, `font` ("Inter", scalar), `fontMono`, `type` (sizes, weights, hinting), `shadow`, `motion`, `icons`, `cursor`, `kde` (ids of every KDE theme piece).
QML apps: `import org.vela.tokens` → `Tokens.radiusCard`, `Tokens.l`, `Tokens.accent` … (singleton installed in the default Qt 6 QML path, no import path needed).

## Rules for every Vela app (assistant, terminal, onboarding, apps, settings)
1. **Colors: use the system palette / Kirigami.Theme**, never hard-coded hex for surfaces or text. The color schemes `VelaChiaro` and `VelaScuro` already carry the Vela palette, and the user can change the accent in Impostazioni → Colori. Hard-code only brand art (logo, illustrations).
2. **Radius**: windows/cards/popups 12 px, controls (buttons, fields) 8 px, small chips 6 px, dock 20 px, top bar 0 (full width) or 16 px when floating.
3. **Spacing**: 4 px grid; default paddings 12/16, section gaps 24, page margins 24–32.
4. **Type**: Inter 10 pt for UI (Kirigami default font is set system-wide), 9 pt secondary, headings 15 pt DemiBold, monospace JetBrains Mono 10 pt. Sentence case, plain Italian.
5. **Motion**: 120 ms for hover/press, 200 ms for panels and page transitions, 280 ms max; easing `OutCubic` (QML `Easing.OutCubic`) or the spring curve in tokens. Multiply by `Kirigami.Units.longDuration`-style values or check `kdeglobals [KDE] AnimationDurationFactor` — when it is 0 (reduced motion) animate nothing.
6. **Light/dark**: never hard-code a mode. Switch with `vela-theme apply --mode light|dark|auto [--accent #RRGGBB|default]` (state in ~/.config/vela/appearance.json).
7. **Accent** is coral `#D4624A` (selection, primary buttons, focus ring, switches). Brand coral `#E68B75` and the marine blues are for illustrations/logo only.
8. **Glass**: translucent surfaces (`Tokens.glass`) only for floating chrome (popovers, assistant bar); content areas stay opaque.
9. **Icons**: use freedesktop names so the Vela icon theme (Papirus-based) applies; Vela app icons: `vela-logo`, `vela-logo-symbolic`, `vela-assistant`, `vela-terminal-ai`, `vela-welcome`, `vela-spazi`, `vela-memoria`, `vela-privacy`, `vela-settings-ai`, `vela-browser-ai`, `vela-snapshots`, `vela-assistant-symbolic`.

## Palette
| token | light | dark |
|---|---|---|
| window | #F3F6F7 | #15232B |
| view (content) | #FFFFFF | #1A2A33 |
| header / titlebar | #EAF0F2 | #1D2E37 |
| text | #15303A | #E3EEF0 |
| text secondary | #587279 | #93AAB1 |
| separator | #D5E0E3 | #2A3E48 |
| accent (selection, primary) | #D4624A | #D4624A |
| link | #1C6F86 | #6FB7C4 |
| positive / neutral / negative | #2E8B57 / #B7791F / #C93A3A | #5CBF8A / #E0B25C / #E66A6A |

Window buttons (left, close/minimize/maximize, 12 px circles, 24 px hit area, glyph on hover): close #E5806C, minimize #E3C27E, maximize #7FB49E; inactive window: #C4CFD3 (light) / #3C525C (dark).

Contrast (WCAG): text 12.9:1 light / 13.6:1 dark; secondary text 4.4:1 / 6.6:1; white on accent 3.7:1 (used for bold/selected UI, not body copy).
