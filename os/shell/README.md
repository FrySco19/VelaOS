# VelaOS shell — OS-0 scaffold

Native desktop shell for the VelaOS Linux guest (brief rev 2, section 7 #6).
It is a small software compositor written in C: it owns the framebuffer
(`/dev/fb0`) and the input devices (`/dev/input/event*`) and draws every pixel
itself. There is no browser, no web view, no HTTP and nothing on `:7878`.
The Glass host page (`poc/product/glass`) is **not** the OS shell.

## What OS-0 contains

| Area | Section 7 decision | In this scaffold |
|------|--------------------|------------------|
| Layout | Top bar + floating bottom-centre dock only | Edge-to-edge 32 px top bar; floating dock 16 px above the bottom edge |
| Dock | 6 monochrome line glyphs on dark frosted glass; active = frosted light tile + 2 px teal underline | Vela, Find, Spaces, Notes, Media, Settings; frosted tile + teal underline on the focused app, small dot for running apps, hover label |
| Windows | – □ × on the right only, 16 px radius | Integrated title area, app glyph + title, – □ × right-aligned, muted ink when inactive, soft shadow (stronger when focused) |
| Focus mode | First-class, hides bar + dock | F11 toggles, Esc leaves; focused window goes full screen; minimise/□ also leave Focus |
| Notifications | Top-right, outside windows | Frosted card under the bar, persists across focus changes, × dismisses |
| Themes | Light / Dawn / Deep | Token sets in `src/theme.c`; runtime switch (Settings window swatches, top-bar moon/sun, Ctrl+Alt+1/2/3) recolours bar, dock, windows, wallpaper |
| Materials | Glass | Real backdrop blur of whatever is underneath (wallpaper *and* windows), tint, frost, 1 px light stroke |
| Spaces | Virtual desktops | 3 Spaces; Ctrl+Alt+←/→ or the dock Spaces glyph; top-bar indicator; each Space keeps its own windows |
| Locale | EN default, IT selectable at Welcome | All UI strings English. Welcome card (with Italian) is OS-6, not built yet |

Wallpapers are generated procedurally (night sea + glowing sail for Deep,
golden hour for Dawn, clear morning for Light), so the guest needs no image files.
Window contents are fixtures only (Home card, Find, Notes, Media, Settings).

## Layout

```
os/shell/
  include/vs.h            shared types and API
  src/gfx.c               AA SDF shapes, soft shadows, box-blur, backdrop blur, clip
  src/text.c              stb_truetype text with glyph cache (Inter + DejaVu Serif)
  src/theme.c             Light / Dawn / Deep tokens
  src/wallpaper.c         procedural signature wallpapers
  src/glyphs.c            monochrome line glyphs (dock, status, captions, cursor)
  src/shell.c             scene + input: bar, dock, windows, Focus, Spaces, notifications
  src/main_headless.c     PNG preview renderer + behavioural self-tests
  src/main_fbdev.c        Linux guest backend: /dev/fb0 + evdev
  guest/init              initramfs PID 1: mounts, drivers, then velashell
  guest/build-initramfs.sh  Debian kernel + busybox + velashell -> initramfs
  guest/run-qemu.sh       boot the guest in QEMU (std VGA, 1920x1080)
  tools/qemu-shots.py     headless boot + QEMU monitor screendumps (PNG)
  assets/fonts/           Inter (OFL) + DejaVu Serif, with licences
  docs/preview/           rendered preview frames
```

## Preview

On any Linux/WSL box with a C compiler:

```sh
cd os/shell
make test       # behavioural self-test at 1920x1080 and 1280x800
make preview    # writes build/preview/*.png (same renderer the guest uses)
```

Committed frames: `docs/preview/os0-deep-desktop.png`, `os0-deep-empty.png`,
`os0-light-settings.png`, `os0-dawn-home.png`, `os0-deep-focus-mode.png`,
`os0-deep-space2.png`.

Guest (QEMU, Debian/Ubuntu build host):

```sh
make guest                     # static velashell + initramfs (fetches kernel + busybox debs)
./guest/run-qemu.sh            # window; or:
python3 tools/qemu-shots.py    # headless boot + screendumps in build/shots/
```

Keys in the guest: F11 Focus mode, Esc leave Focus, Ctrl+Alt+←/→ Spaces,
Ctrl+Alt+1/2/3 Light/Dawn/Deep, Ctrl+Alt+T next theme, Ctrl+Alt+N test notification,
Ctrl+Alt+= larger text, Ctrl+Alt+M reduce motion.

The VirtualBox ISO is built separately on `vela/vbox-iso`; this shell is meant to
be dropped into that image once OS-0 is accepted.

## Acceptance status (brief section 3)

| ID | Status | Notes |
|----|--------|-------|
| A1 no browser/URL/:7878 | pass (by construction) | Native framebuffer compositor; no HTTP anywhere |
| A2 Welcome card EN/IT, no account | deferred (OS-6) | |
| A3 full-bleed wallpaper | pass | Procedural night sea + glowing sail default |
| A4 top bar + live clock | partial | Bar, status glyphs, minute clock; menus are static labels |
| A5 floating dock launch/focus/active | pass | Self-test + previews |
| A6 two overlapping windows, z-order, click-to-focus | pass | Self-test |
| A7 drag, –, □, ×, inactive dimmed | pass | Minimise restores from the dock |
| A8 translucent surfaces blur what is behind | pass | Backdrop blur of the composed frame |
| A9 top-right notification, persists, × | pass | |
| A10 permission sheet | deferred (OS-7) | |
| A11 runtime Light/Dawn/Deep, Larger text, Reduce motion | partial | Themes + larger text live; no animations exist yet, so Reduce motion is only a stored flag |
| A12 idle RAM < 1.5 GB | pending measurement | Guest logs memory in use on the first frame |
| A13 no telemetry, EN default | pass | |

## Next packages (not in OS-0)

OS-1 compositor hardening (damage tracking, double buffering, DRM/KMS page flip) ·
OS-2 real menus and status backends · OS-3 resize, snapping, child dialogs ·
OS-4 dock launch of real apps, app grid · OS-5 tokens file shared with apps, motion system ·
OS-6 Welcome (Appearance / Language EN+IT / Privacy / Accessibility), persisted ·
OS-7 permission sheet (system-modal, dim + blur) · OS-8 Home + Find ·
OS-9 Vela / Tasks / Activity · OS-10 RAM budget + A1–A13 evidence pack.

## Limits

- Software rendering, full redraw per event; fine for a VM, not tuned.
- PS/2 relative mouse or absolute tablets via evdev; no touch, no multi-monitor.
- Theme/Space state is not persisted yet (no storage layer in the guest).
