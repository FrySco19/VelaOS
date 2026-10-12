# VelaOS native x86_64 (VirtualBox ISO)

This folder builds **a real bootable operating system image**: a small x86_64 kernel written
for VelaOS, loaded by GRUB, that owns the screen, keyboard and mouse of the virtual machine
and draws the Vela desktop itself (top bar with clock, dock, windows, terminal shell).
There is **no Linux, no browser and no host web page** underneath it.

> **Status: v0.2 shell scaffold, aligned to the OS vision brief section 7 (closed decisions):**
> top system bar + floating bottom-centre dock with six monochrome glyphs on frosted glass
> (active app = frosted tile + teal underline), windows with right-aligned – □ ×, Focus mode
> (hides bar and dock), notifications top-right, Spaces = virtual desktops, themes Light / Dawn / Deep,
> English by default with Italian selectable on the first-boot Welcome card. Everything is drawn by the
> guest itself; nothing is served over HTTP and no host browser is involved.

| File | What it is |
|------|------------|
| `dist/velaos-x86_64.iso` | Bootable ISO (BIOS). Attach it to a VirtualBox VM. |
| `dist/velaos-x86_64.vdi` | Same system as a VirtualBox disk (optional alternative to the ISO). |
| `screenshots/vbox-*.png` | Captured **inside VirtualBox 7.2.20** at 1920×1080: `vbox-1-grub`, `vbox-2-splash`, `vbox-3-welcome`, `vbox-desktop`, `vbox-4-focus`, `vbox-5-deep`, `vbox-6-vdi-desktop` (booted from the VDI). |
| `screenshots/qemu-*.png` | Same ISO in QEMU (headless smoke test, `tools/qemu-shots.sh`). |
| `Start-VelaOS-VirtualBox.ps1` | One command: creates the VM and boots the ISO on Windows. |

![VelaOS desktop in VirtualBox](screenshots/vbox-desktop.png)

## Quick start on Windows (one command)

1. Install VirtualBox 7.x (https://www.virtualbox.org/).
2. Open PowerShell in this folder (`os/x86_64`) and run:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\Start-VelaOS-VirtualBox.ps1
   ```

That creates a VM called `VelaOS-x86_64` and starts it.

## Manual VirtualBox steps (no script)

1. **New VM**: VirtualBox → *Machine* → *New*.
   - Name: `VelaOS`
   - ISO Image: leave empty (attach below) — or pick `dist/velaos-x86_64.iso` and tick *Skip Unattended Installation*.
   - Type: **Other**, Version: **Other/Unknown (64-bit)**.
2. **Hardware**: Memory **512 MB**, 1 CPU. **Do not** enable EFI.
3. **Hard disk**: *Do Not Add a Virtual Hard Disk* (the ISO is a live system).
   (Alternative: *Use an Existing Virtual Hard Disk File* → `dist/velaos-x86_64.vdi`.)
4. **Settings → Display**: Graphics Controller **VBoxVGA**, Video memory **32 MB** or more (1920×1080 needs ≥ 9 MB).
5. **Settings → System → Motherboard**: Pointing Device **PS/2 Mouse**, *Enable EFI* **off**.
6. **Settings → Storage**: click the empty optical drive → disc icon → *Choose a disk file…* → `dist/velaos-x86_64.iso`.
7. **Start**.

### What you should see

1. **GRUB menu** (blue): `VelaOS 0.2 (x86_64)` — boots automatically after 3 seconds at 1920×1080.
   Other entries: `1280x720 (small screens)` and `safe graphics (firmware mode)` if the screen stays black.
2. **Splash**: black screen, white sail glyph, **VelaOS**, a thin teal progress bar
   (`Starting kernel… → Probing devices… → Starting compositor… → Starting desktop…`).
3. **Welcome to VelaOS** card (first boot): Appearance (Light / Dawn / Deep), Language
   (English default / Italiano), Privacy (On this device), Accessibility (Larger text, Reduce motion).
   Click choices with the mouse, then **Make yourself at home →** (or press **Enter**).
4. **Desktop**: night-sea wallpaper with a glowing sail, a frosted top bar (sail + **VelaOS**, app menus,
   Spaces dots, search / network / volume / battery, live date and time), the floating dock, the **Vela**
   window (command console) and **Settings**, plus a *Welcome aboard* notification top-right.
5. Click inside the VM window to use the mouse (VirtualBox captures it; **Right Ctrl** releases it).

### Things to try

| Action | How |
|--------|-----|
| Open / focus / minimise an app | Click its dock icon (Vela, Find, Spaces, Notes, Media, Settings) |
| Move a window | Drag its title row |
| Minimise / maximise / close | – □ × at the top-right of each window |
| Focus mode (hides bar + dock) | **F11**, or top bar **View**, or type `focus`; **Esc** exits |
| Switch Space (virtual desktop) | **Ctrl+Alt+←/→**, click the dots in the top bar, or the Spaces app |
| Change theme | Settings → Appearance, or type `theme light`, `theme dawn`, `theme deep` |
| Italian UI | Welcome card / Settings → Italiano, or type `lang it` |
| Console | type `help`, `neofetch`, `ls`, `cat README.txt`, `notify hello`, `shutdown` |

## Build from source (Linux / WSL)

```sh
sudo apt-get install -y build-essential nasm grub-pc-bin grub-common xorriso mtools python3-pil qemu-system-x86 socat
make            # -> build/velaos-x86_64.iso
make run        # optional: boot it in QEMU
./tools/qemu-shots.sh   # optional: headless QEMU boot with PNG screenshots
```

Make a VDI from the ISO (Windows): `VBoxManage convertfromraw dist\velaos-x86_64.iso dist\velaos-x86_64.vdi --format VDI`
(the ISO is hybrid, so it also boots as a disk).

## Why this path

Classic VirtualBox virtualises x86 only, so the seL4 Microkit AArch64 build used by the
QEMU executor gate cannot be the VirtualBox deliverable. The shortest path to "boots in
VirtualBox and looks like an OS" is a native x86_64 kernel booted by GRUB with a linear
framebuffer. Section 7 of the vision brief suggests a Linux guest as substrate for the full
OS-0…OS-10 slices; this image is the native, dependency-free boot-to-pixels path that proves the
VirtualBox deliverable today, and either substrate can host the same shell design. The seL4 executor gate (`make e2e`) stays as a separate QEMU AArch64 check;
the Windows `launch-glass.bat` demo stays an optional host demo.

## Limits (honest)

- Kernel uses polling, no interrupts, no scheduler, no processes, no storage driver yet:
  files in *Find* / `ls` are built-in samples, Notes and settings are not saved across reboots.
- Media, Find and Spaces content is a UI scaffold (no real codecs or semantic index yet).
- Glass blur is computed from the wallpaper only (windows behind the bar/dock are not blurred).
- Italian covers the shell chrome, Welcome and Settings; console output stays English.
- BIOS boot only (no EFI). GRUB starts at 1024×768; the kernel then switches to 1920×1080 through the Bochs/VirtualBox VBE (DISPI) registers. GRUB menu has a 1280×720 entry and a safe-graphics entry (firmware mode).
- PS/2 keyboard (US layout) and PS/2 mouse only. USB tablet pointing device is not supported.
- The clock shows the VM RTC (VirtualBox: host local time; QEMU: UTC).
- `shutdown` uses the VirtualBox/QEMU ACPI ports; on other hardware close the window.
- Glass and the seL4 executor are not inside this image yet; that is the next step once boot is stable.
