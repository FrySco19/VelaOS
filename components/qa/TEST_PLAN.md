# Vela OS 2 — Test plan 

The tester speaks for the initial tester. A test passes only if it was **run in a VM** (QEMU dev VM / ISO VM on the box, or VirtualBox `Vela-2` on the user's PC) and left evidence under
`local development artifacts (not included)<build>/` (PNG, log, JSON). "Should work" is a fail.

Legend — **Auto** = scripted in `components/qa/tests/` · **Semi** = scripted actions + human (qa) visual judgment of screenshots · **PC** = only checkable in VirtualBox on the user's Windows PC.
Environment column: Q = QEMU (box), V = VirtualBox (user PC).

---------------------------------------------------------------------------------------------------
## 0. Regression tests for reported complaints (all S1 if they fail)

| ID | Complaint (his words) | Test | How | Pass criteria | Env |
|----|----|----|----|----|----|
| R1 | "va tutto così lento" | Boot-to-desktop time; app launch latency; idle RAM; compositor actually GPU/llvmpipe-composited, no software-cursor lag | Auto: `t_perf.sh` — `systemd-analyze`, time from VM start to SDDM/autologin desktop (screenshot polling), `time` to first window for Dolphin/Konsole/Settings/Discover via `kdotool search --sync`; `free -m` idle after 60 s | boot→desktop ≤ 25 s Q / ≤ 30 s V (live), Dolphin/Konsole ≤ 1.5 s, Settings ≤ 2.5 s, idle RAM ≤ 1.5 GB; no visible stutter in a 10-frame screenshot burst while dragging a window | Q, V |
| R1b | lento in VirtualBox | VM config: VMSVGA + 3D accel, 2 vCPU, 4 GB, guest additions loaded (`vboxguest`, `vboxvideo`/`vmwgfx`) | PC: `VBoxManage showvminfo Vela-2`, guest `lsmod` | graphics controller VMSVGA, VRAM ≥128 MB, vmwgfx loaded, KWin compositing on (`qdbus org.kde.KWin /Compositor active` = true) | V |
| R2 | app installate non compaiono sul desktop | Install an app from the store → it appears in the app launcher **and** can be put on the desktop / dock; newly installed .desktop shows up without logout | Auto: `t_store.sh` — `flatpak install -y flathub <small app>` (or Discover CLI `plasma-discover --application`), then check `kbuildsycoca`/launcher search (`krunner` query via `qdbus org.kde.krunner`), check `~/Desktop` policy (desktop icons enabled / "Aggiungi al desktop" in right-click menu works); screenshot of launcher + desktop | app found in launcher search within 10 s of install; right-click in launcher shows "Aggiungi al desktop"/"Aggiungi al dock" and doing it shows the icon on the desktop | Q, V |
| R3 | tasto destro sul desktop non fa niente | Right-click on empty desktop | Semi: `t_desktop.sh` — move pointer to desktop centre, right-click (ydotool/QMP mouse), screenshot | context menu appears in ≤ 300 ms, Italian items (Nuova cartella / Nuovo file, Incolla, Cambia sfondo, Impostazioni schermo, Apri terminale qui), no English, no dead entries | Q, V |
| R3b | right-click everywhere | Right-click on a file, in Dolphin, on dock icon, on titlebar, on top bar, on text field | Semi | every surface has a sensible Italian menu | Q |
| R4 | Ctrl+C / Ctrl+V non funzionano | (a) guest-internal copy/paste in text: Kate/text field → other app; (b) file copy/paste in Dolphin and on desktop; (c) Konsole Ctrl+Shift+C/V + Ctrl+C/V hint; (d) host↔guest clipboard | Auto: `t_clipboard.sh` — `wl-copy`/`wl-paste` round-trip, `ydotool key ctrl+c/ctrl+v` in KWrite then read file; Dolphin file copy via keys then `ls`; PC: `VBoxManage guestcontrol` + `controlvm clipboard mode bidirectional`, set host clipboard and paste in guest | all four directions work; host→guest and guest→host text both verified with a nonce string | Q (a–c), V (d) |
| R5 | impostazioni PIENE | Full System Settings: Wi-Fi/rete, Bluetooth, audio, schermi, energia, utenti, data/ora, lingua/regione, tastiera, mouse/touchpad, notifiche, aspetto (tema chiaro/scuro, accento, sfondo), accessibilità, privacy & IA, app predefinite, aggiornamenti, info sistema, stampanti, condivisione, backup/snapshot | Auto+Semi: `t_settings.sh` — `kcmshell6 --list` count + presence of required KCMs; open each in `systemsettings` and screenshot | ≥ all categories listed exist and open without error, all titles in Italian, Vela modules (IA, privacy, memoria) present and functional (toggles persist after reopen) | Q |
| R6 | uno store | Discover opens, Flathub reachable, search, install, launch, remove; Debian packages updates visible | Auto: `t_store.sh` + screenshots of Discover home, search "gimp", app page, install progress, installed list | store opens ≤ 5 s, home not empty, Italian UI, install & remove succeed, removed app disappears from launcher | Q, V |
| R7 | "Meno rumore, più possibilità — ma cos'è sta roba" | No marketing/pretentious copy anywhere | Auto: `t_copy_lint.sh` (static, on overlays + built ISO squashfs) bans taglines (list in `lib/banned_phrases.txt`), Lorem/TODO/FIXME/placeholder, English strings in Vela UI; Semi: qa reads every screenshot | zero banned phrases; every visible string is plain, functional Italian ("Cerca app", not "Un pensiero. Un punto di partenza.") | Q (static + screens) |
| R8 | "installato ma…" (installer) | Install to disk with Calamares, reboot, log in, everything still works, apps installed in live persist after install? (documented) | Semi: `t_install.sh` — QEMU with blank disk, drive Calamares via keys or `calamares` automated config if base provides | installer in Italian, ≤ 7 steps, finishes, reboots to installed system, user created with chosen password | Q, V |

---------------------------------------------------------------------------------------------------
## 1. macOS / Windows parity checklist

Each line = one test ID; severity if failing in brackets.

### 1.1 Boot, login, installer, power
- P-BOOT-1 [S1] ISO boots BIOS and UEFI in QEMU and VirtualBox; branded Vela boot menu in Italian, no Debian/live-build default splash text visible beyond 1 s. (Auto boot, Semi visual)
- P-BOOT-2 [S2] Plymouth splash (Vela) instead of scrolling kernel text.
- P-LOGIN-1 [S1] SDDM Vela theme: user avatar, password field, keyboard layout, power menu, Italian; live session autologin. Wrong password shows a human error.
- P-LOGIN-2 [S2] Lock screen (Meta+L) themed, unlock works.
- P-INST-1 [S1] Calamares: language, keyboard, partition (whole disk, Btrfs default), user, summary, install, reboot. Italian. No crash.
- P-POWER-1 [S1] Spegni / Riavvia / Esci / Blocca / Sospendi from the menu work; shutdown ≤ 10 s; ACPI power button in VirtualBox triggers clean shutdown.

### 1.2 Hardware & system
- P-NET-1 [S1] Network connects automatically in VM (NAT); tray icon shows connected; Wi-Fi panel present (no real Wi-Fi in VM — verify UI only, document).
- P-AUDIO-1 [S2] PipeWire running, output device present (VirtualBox: Intel HDA/AC97), volume slider + mute in tray, test sound plays (`pw-play`), volume OSD appears.
- P-DISP-1 [S1] Resize VirtualBox window → guest resolution follows (guest additions/vmwgfx); Impostazioni Schermo lists resolutions; HiDPI scale 100/125/150/200 works.
- P-DISP-2 [S2] QEMU: `virtio-vga`/`std` resolution change via settings works.
- P-UPD-1 [S2] Updates: Discover shows update page; `apt`/flatpak update notification; no error banner when offline (human message).
- P-NOTIF-1 [S2] Notifications: `notify-send` shows themed Italian popup, Centro notifiche keeps history, Non disturbare toggle works.
- P-SHOT-1 [S2] Screenshots: Print / Meta+Shift+S opens Spectacle (region), saves to ~/Immagini, copies to clipboard, notification with "Apri"/"Mostra nella cartella".

### 1.3 Desktop & multitasking
- P-DESK-1 [S1] Desktop: wallpaper Vela, icons on desktop allowed (Home, Cestino optional), drag icons, right-click menu (R3), new folder on desktop.
- P-DESK-2 [S1] Top bar (global menu / clock / tray) + dock (centered, magnify or hover effect, running indicators, pin/unpin by right-click and drag).
- P-DESK-3 [S2] App launcher (Launchpad-like grid or Kickoff) with search, categories in Italian, all installed apps visible.
- P-MT-1 [S1] Window ops: open, move, resize from edges, maximize (double-click titlebar), minimize to dock and restore, close; snap halves/quarters; animations smooth.
- P-MT-2 [S2] Overview (Meta / Meta+W / hot corner): all windows + virtual desktops; drag window between desktops.
- P-MT-3 [S2] Alt+Tab switcher with thumbnails; Meta+Tab / Alt+` .
- P-SHORT-1 [S1] Shortcuts: Ctrl+C/V/X/Z/A/S/F in apps; Meta (launcher), Meta+E (file), Meta+L (lock), Ctrl+Alt+T (terminal), Alt+F4, Meta+arrow snap, Print, Meta+Space/Alt+Space (search/assistant). Listed in a "Scorciatoie" page.
- P-DND-1 [S2] Drag & drop: file Dolphin→desktop, desktop→Dolphin, file→app (open), text between apps; host↔guest drag&drop in VirtualBox (PC, documented if unsupported on Wayland).

### 1.4 Files & search
- P-FM-1 [S1] Dolphin: create/rename (F2)/copy/cut/paste/delete→Cestino/restore from Cestino/empty, undo (Ctrl+Z), new folder, open with…, properties, compress/extract zip, sidebar (Home, Documenti, Download, Immagini, Musica, Video, Cestino), Italian folder names (xdg-user-dirs it).
- P-FM-2 [S2] Quick preview (space) or preview pane; thumbnails for images/PDF.
- P-FM-3 [S2] USB/shared folder from VirtualBox appears (PC, optional).
- P-SEARCH-1 [S1] Global search (KRunner / Vela search) finds apps, files by name, settings pages, calculator, unit conversion; Italian results; ≤ 300 ms to show.

### 1.5 Apps
- P-STORE-1 [S1] see R6; plus installed app appears on desktop/dock (R2), uninstall removes launcher entry.
- P-TERM-1 [S1] Classic terminal (Konsole or Vela terminal): opens with Ctrl+Alt+T, themed, tabs, copy/paste (Ctrl+Shift+C/V and right-click), font Inter/mono legible, zsh/fish configured.
- P-AITERM-1 [S1] AI terminal: natural language intent → proposed command with explanation → runs only after confirm; dangerous commands (rm -rf, dd, mkfs, sudo) require explicit confirmation; works offline with a clear message if no model.
- P-AI-1 [S1] AI assistant: global shortcut opens it from any app; knows selection/active window (when allowed); answers with local model or configured API; if no model: friendly Italian setup message, not a stack trace. Actions that change files/system show preview + confirm.
- P-BROWSER-1 [S1] Browser opens, loads a page (network), AI panel summarizes page / answers questions, saves to file; Italian UI; default browser set.
- P-ONB-1 [S1] Onboarding on first login: language, theme, AI choice (local / API / off), privacy (memory on/off), finishes in ≤ 2 min, does not reappear, can be reopened from settings.
- P-PRIV-1 [S1] Privacy & IA settings: what AI can see, local vs API, kill switch disables all AI, memory pause/clear, per-app permissions, action log viewable. Toggles persist.
- P-WS-1 [S2] Workspaces per project (if shipped): create, switch, restore.

### 1.6 Copy quality & accessibility
- P-IT-1 [S1] Italian default everywhere (system locale it_IT.UTF-8, keyboard it, timezone Europe/Rome); no English string in any first-party Vela UI; KDE translations installed (`plasma-*-l10n`/`kde-l10n-it` equivalents in trixie).
- P-IT-2 [S2] Microcopy plain and useful: verbs on buttons ("Installa", "Apri"), no marketing, no anglicisms when an Italian word exists, consistent tone (tu), correct accents (è, perché), no truncated labels.
- P-A11Y-1 [S2] Accessibility: text scaling, high contrast/colour, reduced motion, Orca screen reader installed and starts (Meta+Alt+S), keyboard navigation of dock/launcher/settings, sticky keys.

### 1.7 Visual quality (Semi, qa judges every screenshot)
- V-1 [S3] Consistent corner radius (12 px windows/panels/menus), consistent shadows.
- V-2 [S3] One font family (Inter UI + a mono for terminal), no fallback DejaVu in Vela apps; crisp hinting.
- V-3 [S3] Coherent icon theme; no missing-icon placeholders, no blurry raster icons at 100 % scale.
- V-4 [S3] Palette: marine blues + coral accent applied to accent colour, selection, focus rings, wallpaper.
- V-5 [S3] Alignment/spacing on an 4/8 px grid; no clipped text; dark and light modes both readable (contrast ≥ 4.5:1 for text).
- V-6 [S2] No Debian/KDE/"Plasma" branding where Vela branding is expected (About page may credit KDE/Debian).

---------------------------------------------------------------------------------------------------
## 2. Automation

`components/qa/run.sh <build-name> [--iso path|--devvm]` runs everything it can and writes:
`local development artifacts (not included)<build>/{results.json, results.md, *.png, *.log}`.

- `lib/qa.sh` — helpers: `vm` (wraps `tools/vm.sh`), `gssh` (SSH into guest as vela), `shot <name>` (screenshot to evidence dir), `gkey`, `gtype`, `gclick x y [button]` (guest ydotool, falls back to `tools/vm.sh key`), `record PASS|FAIL|SKIP id msg`.
- `tests/t_static_lint.sh` — runs on the box with no VM: banned phrases, English/placeholder detection in overlays, .desktop files have `Name[it]`, packages.list names exist in trixie (`apt-cache policy` when available), shell syntax (`sh -n`) of hooks.
- `tests/t_*.sh` — VM tests listed above; each prints `PASS/FAIL/SKIP <ID> <msg>` lines.
- Visual judgment: after a run qa opens every PNG and records verdicts in `results.md`.

Severity rules for filing: any R* fail = S1. English string or placeholder in main flow = S2. Visual polish = S3.

## 3. Public preview gate
1. Build a fresh ISO without --test and verify its SHA256.
2. Boot it in a dedicated VM with a new user profile.
3. Record first boot, desktop, settings, Store and terminal behavior.
4. Check that development SSH access is absent.
5. Report installer, hardware and model inference separately; mark anything untested.
