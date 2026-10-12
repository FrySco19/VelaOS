# VelaOS Design Tokens v0 — QUALITY v1

Executable sheet for Shell chrome and the host Glass demo. Geometry is ISO VB §7 (read-only: `os/x86_64/README.md` @ `60a5b8b` on `vela/vbox-iso`). Materials and type are upgraded here. This file lives at `docs/art/TOKENS_v0.md` on `art/tokens-v0`. It does not edit `os/x86_64/`.

Host demo minimum: the utterance `ciao` is **body** copy. It stays readable on frost in Light and Deep.

## Typography (real)

Family: **Inter**, then `"Segoe UI"`, `system-ui`, `sans-serif`.

```css
--font-family: Inter, "Segoe UI", system-ui, sans-serif;
```

Chrome and Glass use this stack. Kernel bitmap and debug fonts stay off the top bar, dock, windows, and Glass.

| Role | Size | Weight | Line-height | Tracking | Token prefix |
|------|-----:|-------:|------------:|----------|--------------|
| display | 28px | 600 | 1.2 | -0.02em | `--type-display` |
| title | 20px | 600 | 1.25 | -0.01em | `--type-title` |
| body | 15px | 400 | 1.45 | 0 | `--type-body` |
| body-strong | 15px | 600 | 1.45 | 0 | `--type-body-strong` |
| label | 13px | 500 | 1.35 | 0.01em | `--type-label` |
| caption | 12px | 400 | 1.35 | 0.02em | `--type-caption` |
| micro | 11px | 500 | 1.3 | 0.04em | `--type-micro` |

Each role sets `--type-<role>-size`, `--type-<role>-weight`, `--type-<role>-line`, `--type-<role>-tracking` to the four numbers above. Micro is tooltips only.

**Contrast AA**

| Pair | Foreground | Ground |
|------|------------|--------|
| Deep body | `#E8EEF5` | Deep frost `rgba(20,28,44,0.70)` over `#0B1220` |
| Light body | `#1A2332` | Light frost `rgba(255,255,255,0.78)` or `#FFFFFF` |
| Dawn body | `#F2EDE4` | Dawn frost `rgba(42,52,68,0.72)` over `#1C2430` |

Primary copy (Objective, Proposal titles, Gesture labels, Outcome, the field that holds `ciao`) uses the body pair for the active theme. Muted (`#5B6B7C` / `#B7B0A4` / `#8B97A8`) is caption and audit metadata only. Teal `#2EC4B6` is accent only: the active dock underline and the one primary gesture. Teal is not used for long body text.

**Locale.** Default `en`. Alternate `it` (Welcome card and Settings, same as ISO §7). Strings stay one line: `white-space: nowrap`.

| String | Role | Min content width |
|--------|------|------------------:|
| Conferma | body-strong 15px/600 | 88px |
| Impostazioni | label 13px/500 | 108px |
| Rivedi piano | body 15px/400 | 112px |

`ciao` uses body 15px/400, line-height 1.45, theme body color. It is not micro, not muted, and not teal.

## Frost materials (credible)

Permanent chrome (top bar, dock, windows) and ephemeral Glass share one stack. Ephemeral changes blur and fill opacity only.

| Token | Permanent | Ephemeral Glass |
|-------|-----------|-----------------|
| blur | `24px` `--glass-frost-blur` | `20px` `--glass-ephemeral-blur` |
| saturate | `140%` `--glass-frost-saturate` | `140%` (same) |
| opacity | `0.72` `--glass-frost-opacity` | `0.88` `--glass-ephemeral-opacity` |
| frost-tint | per theme (below) | same tint |
| border | `rgba(255,255,255,0.18)` `--glass-frost-border` | same |
| highlight | `rgba(255,255,255,0.28)` `--glass-frost-highlight` | same |
| shadow | `0 8px 28px rgba(0,0,0,0.28)` `--glass-frost-shadow` | same |

```css
backdrop-filter: blur(var(--glass-frost-blur)) saturate(var(--glass-frost-saturate));
background: var(--theme-frost-tint); /* opacity token multiplies the frost layer */
```

Ephemeral panels set blur to `20px` and layer opacity to `0.88`.

| Theme | `--theme-*-frost-tint` | bg | surface | text | text-muted |
|-------|------------------------|----|---------|------|------------|
| Light | `rgba(255,255,255,0.78)` | `#F4F7F8` | `#FFFFFF` | `#1A2332` | `#5B6B7C` |
| Dawn | `rgba(42,52,68,0.72)` | `#1C2430` | `#2A3444` | `#F2EDE4` | `#B7B0A4` |
| Deep | `rgba(20,28,44,0.70)` | `#0B1220` | `#141C2C` | `#E8EEF5` | `#8B97A8` |

Accent on every theme: `#2EC4B6` (`--accent-teal`). Borders: Light `rgba(26,35,50,0.12)`, Dawn `rgba(242,237,228,0.14)`, Deep `rgba(232,238,245,0.12)`.

**Beauty.** The night-sea wallpaper and sail stay readable through frost. Blur is wallpaper-derived (ISO v0.2 limit: windows behind the bar and dock are not blurred). Fill opacity stays at 0.72 on permanent chrome so the sea is still visible; ephemeral Glass at 0.88 is for short-lived reading, then it closes.

## Toolbar familiar (Shell)

ISO §7 geometry. Do not resize or reorder.

**Top bar.** Height `40px` (`--topbar-height`). Frost stack above (opacity 0.72). Left to right: sail (`18px`) + **VelaOS** · app menus · Spaces dots (`6px`, gap `6px`) · search / network / volume / battery (`16px`) · live date and time (weight 500). Item gap `12px`. Hidden only in Focus (`--topbar-hiding: focus-only`).

**Dock.** Floating bottom-centre. Always visible except Focus. Frost pill, radius `20px`, outer height `64px`, padding `8px` `12px`, margin-bottom `16px`. Six monochrome glyphs, fixed order:

| Index | Id | Label |
|------:|----|-------|
| 1 | `vela` | Vela |
| 2 | `find` | Find |
| 3 | `spaces` | Spaces |
| 4 | `notes` | Notes |
| 5 | `media` | Media |
| 6 | `settings` | Settings |

| Token | Value |
|-------|-------|
| `--dock-cell-size` | `44px` |
| `--dock-glyph-size` | `28px` |
| `--dock-gap` | `8px` |
| `--dock-glyph-opacity-idle` | `0.72` |
| `--dock-glyph-opacity-active` | `1` |
| `--dock-glyph-color` | `currentColor` |
| `--dock-tile-frost-opacity` | `0.90` |
| `--dock-tile-radius` | `14px` |
| `--accent-teal` | `#2EC4B6` |
| `--accent-teal-underline-height` | `2px` |
| `--dock-underline-width` | `20px` |
| `--accent-teal-underline-radius` | `1px` |

**Active** = frost tile + teal underline `2px` high × `20px` wide. That is the only active accent. No second accent, no glow.

**Windows.** Controls `–` `□` `×` on the right (`12px` glyphs, gap `10px`). **Focus** (F11 / View / `focus`) hides top bar and dock. **Notifications** anchor top-right, inset `12px`, max width `320px`. Spaces are virtual desktops.

## Side panel (always present)

A lateral panel is always present in the shell chrome. It is not ephemeral Glass. Three authorization/visibility levels, user-changeable anytime:

| Level | Token | Behaviour |
|-------|-------|-----------|
| Full | `--sidepanel-level: full` | Full width `--sidepanel-width-full: 320px`; Objective + Proposal + Gesture + Rationale + Audit visible |
| Reduced | `--sidepanel-level: reduced` | Width `--sidepanel-width-reduced: 72px`; icon rail / collapsed labels; primary gesture still reachable |
| Hidden | `--sidepanel-level: hidden` | Width `--sidepanel-width-hidden: 0` / off-canvas; panel slot reserved (always present in layout); invoke via gesture or shortcut to restore |

Tokens:

- `--sidepanel-present: always`
- `--sidepanel-edge: right` (default; left only if Francesco says)
- `--sidepanel-frost: permanent` — permanent shell frost stack (`--glass-frost-blur` 24px / saturate 140% / opacity 0.72), matching Shell SHELL_SPEC_v0. Shell owns chrome z-order. The panel is Shell chrome, not a Glass overlay. Denied-list exception: this is shell chrome.
- `--sidepanel-transition: 180ms` ease for level changes anytime

Ephemeral Glass overlays stay 20px/0.88 and are NOT the side panel.

The panel is not a chat — no bubbles/thread; Objective→Proposal→Gesture→Audit; same frost language as toolbar.

Levels do not require a reboot. The preference persists for the session.

Ephemeral Glass still closes when its phase ends. The side-panel slot stays in the shell layout at the chosen level. Focus hides the top bar and the dock; it does not delete the side-panel slot.

## Ephemeral Glass (context-driven — NOT empty demo windows)

Dock-free. Shown only while the host Shell phase is `glass`, `result`, or `error`. `Chiudi` / `Annulla` remove the surface. The verified result stays.

Catalog names, purpose, and required content:

| Name | Purpose | Required content |
|------|---------|------------------|
| Frost | Ephemeral material | blur `20px`, saturate `140%`, opacity `0.88`, theme frost-tint, border, highlight, shadow |
| Theme | Light / Dawn / Deep | one theme; Deep is the night-sea home |
| Objective | User intent | the utterance (`ciao`); body color; one line |
| Proposal | Candidates | title, version, score for each candidate; no authority |
| Gesture | One primary teal action | **Avvia** in teal; Rivedi piano, Annulla, Chiudi are neutral |
| Rationale | Planner note, no effect | the note text; caption color is allowed |
| Outcome | Verified result | request id and value, or reviewed-not-executed; remains after close |
| Audit | What was checked | lane, selections emitted, grants, frames signed, trace |
| Loopback | Verified channel | `127.0.0.1`, Host pinned to the bound port, per-run token, `application/json` |

Gesture labels use body or body-strong, `nowrap`, and the locale min widths above. The host page sends only an utterance or `{action, candidate index}`.

## Zero-App fluidity

Deep night-sea and sail are the emotional home. Glass borrows that wallpaper through frost, then leaves. Only the blocks the current phase needs are mounted, and only until `Chiudi` or `Annulla`. The dock stays on the Shell desktop and stays off ephemeral frames. Italian chrome is allowed; token names stay English. Guiding line: only what is needed, for as long as it is needed. *Il risultato resta. Glass si chiude.*

## No-conflict

Aligned to **ISO VB §7** closed decisions, read-only, commit `60a5b8b` (`os/x86_64/README.md` on `vela/vbox-iso`): top system bar + floating bottom-centre dock, six monochrome glyphs, active = frosted tile + teal underline, windows with right-aligned – □ ×, Focus hides bar and dock, notifications top-right, Spaces = virtual desktops, themes Light / Dawn / Deep, English default with Italian on the Welcome card.

1. §7 geometry in this sheet is unchanged: top bar `40px`; dock bottom-centre; cell `44px`; glyph `28px`; gap `8px`; underline `2×20` in `#2EC4B6`; Focus hides bar and dock.
2. Active dock state is frost tile + that underline. No second accent.
3. Glyph order is Vela, Find, Spaces, Notes, Media, Settings. Rename only with Francesco’s OK.
4. Themes are Light, Dawn, and Deep only.
5. Edits stay on `art/*` and `docs/art/`. `os/x86_64/` is out of scope.
6. Do not push this sheet onto `vela/vbox-iso` or `main` without Francesco’s OK.
7. The side panel is additive shell chrome (`--sidepanel-present: always`, edge `right`). It does not change §7 bar, dock, glyph, or teal geometry.
