# VelaOS Design Tokens v0

English visual-design tokens for Glass / Zero-App chrome (top bar, dock, ephemeral surfaces).
Aligned to **ISO VB section 7** closed decisions (read-only reference: native shell at `vela/vbox-iso` / `os/x86_64/README.md` @ `60a5b8b`). This sheet does **not** redesign that shell and does **not** modify `os/x86_64/`.

Branch base: `main` @ `6f05ff0`. Branch: `art/tokens-v0`.

## No-conflict rules (ISO VB §7)

1. Do not change shell geometry or chrome layout described in §7 (top bar + floating bottom-centre dock; Focus hides both).
2. Do not invent a second accent for “active”: active dock state is always **frosted tile + teal underline**.
3. Dock glyph set and order are fixed: **Vela · Find · Spaces · Notes · Media · Settings** (six monochrome glyphs). Rename only with Francesco’s OK.
4. Themes remain exactly **Light / Dawn / Deep** — no extra theme names in v0.
5. Art tokens live under `docs/art/` (and future `art/*` branches). Component specs belong to Vela Glass UI / Vela Shell; kernel/shell pixels under `os/x86_64/` are out of scope for this branch.
6. Never push these tokens onto `vela/vbox-iso` or `main` without Francesco’s OK.

## Frost materials (Glass)

| Token | Value | Notes |
|-------|-------|--------|
| `--glass-frost-blur` | `24px` | Backdrop blur for bar, dock, ephemeral Glass |
| `--glass-frost-saturate` | `140%` | Slight saturation lift over wallpaper |
| `--glass-frost-opacity` | `0.72` | Fill opacity of frost layer |
| `--glass-frost-border` | `rgba(255,255,255,0.18)` | Hairline highlight |
| `--glass-frost-highlight` | `rgba(255,255,255,0.28)` | Top-edge sheen |
| `--glass-frost-shadow` | `0 8px 28px rgba(0,0,0,0.28)` | Soft depth under floating chrome |
| `--glass-ephemeral-opacity` | `0.88` | Short-lived Glass surfaces (higher than bar/dock) |
| `--glass-ephemeral-blur` | `20px` | Slightly tighter blur for ephemeral panels |

Wallpaper-derived blur only (v0.2 limit): windows behind bar/dock are not blurred.

## Teal underline (active)

| Token | Value | Notes |
|-------|-------|--------|
| `--accent-teal` | `#2EC4B6` | Primary accent; splash progress + active underline |
| `--accent-teal-underline-height` | `2px` | Dock active underline thickness |
| `--accent-teal-underline-opacity` | `1` | Solid underline (no glow in v0) |
| `--accent-teal-underline-radius` | `1px` | Soften ends slightly |
| `--accent-teal-muted` | `rgba(46,196,182,0.35)` | Inactive/hover wash |

## Themes

Shared roles: `bg`, `surface`, `text`, `text-muted`, `accent`, `border`, `shadow`.
`accent` maps to `--accent-teal` in all themes for v0.

### Light

| Token | Value |
|-------|--------|
| `--theme-light-bg` | `#F4F7F8` |
| `--theme-light-surface` | `#FFFFFF` |
| `--theme-light-text` | `#1A2332` |
| `--theme-light-text-muted` | `#5B6B7C` |
| `--theme-light-accent` | `var(--accent-teal)` |
| `--theme-light-border` | `rgba(26,35,50,0.12)` |
| `--theme-light-shadow` | `0 8px 24px rgba(26,35,50,0.10)` |
| `--theme-light-frost-tint` | `rgba(255,255,255,0.78)` |

### Dawn

| Token | Value |
|-------|--------|
| `--theme-dawn-bg` | `#1C2430` |
| `--theme-dawn-surface` | `#2A3444` |
| `--theme-dawn-text` | `#F2EDE4` |
| `--theme-dawn-text-muted` | `#B7B0A4` |
| `--theme-dawn-accent` | `var(--accent-teal)` |
| `--theme-dawn-border` | `rgba(242,237,228,0.14)` |
| `--theme-dawn-shadow` | `0 10px 28px rgba(0,0,0,0.35)` |
| `--theme-dawn-frost-tint` | `rgba(42,52,68,0.72)` |

### Deep

| Token | Value |
|-------|--------|
| `--theme-deep-bg` | `#0B1220` |
| `--theme-deep-surface` | `#141C2C` |
| `--theme-deep-text` | `#E8EEF5` |
| `--theme-deep-text-muted` | `#8B97A8` |
| `--theme-deep-accent` | `var(--accent-teal)` |
| `--theme-deep-border` | `rgba(232,238,245,0.12)` |
| `--theme-deep-shadow` | `0 12px 32px rgba(0,0,0,0.45)` |
| `--theme-deep-frost-tint` | `rgba(20,28,44,0.70)` |

Default first-boot theme: **Light** (Welcome card); Deep is the night-sea desktop reference.

## Dock (familiar toolbar)

Geometry (ISO §7 — do not redesign):

- Floating **bottom-centre**, frosted glass, **always visible** (`hiding: none`) except Focus mode.
- Exactly **6** monochrome glyphs, fixed order:

| Index | Glyph id | Role |
|------:|----------|------|
| 1 | `vela` | Primary / Vela console |
| 2 | `find` | Find / search |
| 3 | `spaces` | Spaces (virtual desktops) |
| 4 | `notes` | Notes |
| 5 | `media` | Media |
| 6 | `settings` | Settings |

| Token | Value | Notes |
|-------|-------|--------|
| `--dock-height` | `64px` | Outer hit target height |
| `--dock-padding-y` | `8px` | Vertical padding inside frost |
| `--dock-padding-x` | `12px` | Horizontal padding |
| `--dock-gap` | `8px` | Space between glyph cells |
| `--dock-glyph-size` | `28px` | Monochrome icon optical size |
| `--dock-cell-size` | `44px` | Square cell |
| `--dock-radius` | `20px` | Frost pill radius |
| `--dock-glyph-color` | `currentColor` | Monochrome; inherit theme text |
| `--dock-glyph-opacity-idle` | `0.72` | Idle glyphs |
| `--dock-glyph-opacity-active` | `1` | Active glyph |
| `--dock-tile-frost-opacity` | `0.90` | Active frosted tile fill |
| `--dock-tile-radius` | `14px` | Active tile corner |
| `--dock-underline-width` | `20px` | Teal underline length under active tile |
| `--dock-margin-bottom` | `16px` | Lift above screen edge |

Active state = `--dock-tile-frost-*` + `--accent-teal` underline (`--accent-teal-underline-height`). No other active accent.

## Top bar

| Token | Value | Notes |
|-------|-------|--------|
| `--topbar-height` | `40px` | Familiar top system bar |
| `--topbar-frost-opacity` | `0.70` | Frost fill |
| `--topbar-item-gap` | `12px` | Cluster spacing |
| `--topbar-icon-size` | `16px` | Status icons (search, network, volume, battery) |
| `--topbar-clock-weight` | `500` | Live date + time |
| `--topbar-sail-size` | `18px` | Sail glyph beside VelaOS wordmark |
| `--topbar-spaces-dot-size` | `6px` | Spaces dots |
| `--topbar-spaces-dot-gap` | `6px` | Dot spacing |
| `--topbar-hiding` | `focus-only` | Hidden only in Focus (F11) |

Chrome order (left → right): sail + **VelaOS** · app menus · Spaces dots · search / network / volume / battery · live date+time.

## Windows & Focus

| Token | Value | Notes |
|-------|-------|--------|
| `--window-controls` | `close-max-min-right` | Right-aligned `–` `□` `×` |
| `--window-control-size` | `12px` | Control glyph size |
| `--window-control-gap` | `10px` | Gap between controls |
| `--focus-hides` | `topbar,dock` | Focus mode (F11 / View / `focus`) |
| `--notification-anchor` | `top-right` | Notifications |
| `--notification-gap` | `12px` | Inset from top/right edges |
| `--notification-max-width` | `320px` | Card max width |
| `--spaces-model` | `virtual-desktops` | Spaces = virtual desktops |

## Ephemeral Glass surfaces

Use frost tokens with `--glass-ephemeral-*` for short-lived panels (menus, popovers, Welcome card chrome). Keep teal as the only accent. Prefer Light/Dawn/Deep theme roles for text and borders; do not introduce a fourth theme.

## Implementation notes

- Prefer CSS custom properties as named above; map 1:1 into Glass/Shell component specs.
- Monochrome dock glyphs: single-color paths; tint via `--dock-glyph-color` / theme text.
- Values are v0 starting points for implementers; pixel-perfect capture from VirtualBox screenshots may refine blur/opacity later without changing glyph order or active-state rules.

## Zero-App concept alignment

Fluidity and beauty for Zero-App moments, read from the night-sea / sail wallpaper and ephemeral Glass. This note does not add tokens, themes, or a second accent.

- **Wallpaper.** Deep night-sea and a sail; a moonlight path on the water; a cool blue palette. Theme **Deep** is the emotional home of Zero-App moments (still one of Light / Dawn / Deep — no extra theme name).
- **Ephemeral Glass over wallpaper.** Frost blur and opacity (`--glass-frost-*`, `--glass-ephemeral-*`) keep the sea and sail alive behind the UI. Beauty is transparency, not solid cards.
- **Motion.** Notifications may ghost and fade. Glass appears and closes. The result stays; the chrome leaves — *Il risultato resta. Glass si chiude.*
- **Accent discipline.** Teal (`--accent-teal`) only for primary actions (Conferma, Sì, play/pause, checks, Wi-Fi). Never a second accent that fights the moonlit teal water. Dock active state stays frosted tile + teal underline.
- **Shell vs Zero-App.** Dock and top bar stay for the familiar shell. Zero-App moments are dock-free floating Glass. Do not force the dock into ephemeral frames.
- **Language.** Italian chrome is fine. Token names stay English and language-agnostic.
- **Guiding line.** Only what is needed, for as long as it is needed — fluidity over permanence.
