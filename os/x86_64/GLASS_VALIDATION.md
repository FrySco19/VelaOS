# Native Glass validation — 2026-10-12

The Zero-App visual direction is running in a real native x86_64 guest, rather than a generated screenshot or host HTML page.

## Actual checks

- Cross-compiled the no_std Rust surface model for x86_64-unknown-none with Rust 1.96.1 and linked it with the existing C framebuffer shell.
- Two Rust tests passed: bounded routing/unknown intents and cancellation/retention state transitions.
- Built the GRUB BIOS ISO with GCC, NASM and grub-mkrescue; the multiboot2 ELF check passed.
- ELF W^X checker passed for four load segments. This does not establish page-table NX enforcement or complete runtime isolation.
- A real QEMU boot test passed: keyboard note intent, editing, Ctrl+Enter RAM retention, automatic closure after three seconds, network unavailable surface and Esc cancellation. Serial output records release of temporary buffers; it does not prove that all system memory reaches zero.
- VirtualBox booted VelaOS-Glass at 512 MB with BIOS, Other/Unknown 64-bit and VBoxVGA. Keyboard intent → note entry → Ctrl+Enter retention was exercised. The final ISO was attached and booted; its framebuffer is captured in screenshots/glass-virtualbox.png.

The QEMU captures in screenshots/glass-desktop.png, glass-note.png and glass-network.png are guest framebuffer captures. They are not concept renders. The new photographic wallpaper is an illustration asset, not evidence of task execution.

## Packaged artifacts

- dist/velaos-glass-x86_64.iso — SHA-256 8d158ab03f27583a66d111217cd5a6829a728f5f5b1156cfd1019c49865b2827.
- dist/velaos-glass-x86_64.vdi — SHA-256 729a8f9c20707cb02a3104fcec019f0ca5ecc2d92aae7f8b583cafc4c56ef36b.

The VDI is a conversion of the verified hybrid ISO. A separate VDI boot was not performed in this session. A clean Linux build is covered by the Native x86 Glass workflow; tool/font versions can produce different binary hashes, so source-build reproducibility does not imply bit-identical artifacts.

## Scope still open

No live LLM, networking, audio decoder, secure process scheduler, Wasm task domains, Biscuit/PCB admission or durable LFS writes are connected to this x86 preview. It remains distinct from the seL4 AArch64 proof. Notes are retained only in session RAM, with no cryptographic storage receipt. The interface says so. The wallpaper/UI change is not a whole-system memory or latency qualification.

The old scaffold ISO/VDI remain available. The launcher defaults to the new Glass ISO and a separate VelaOS-Glass VM.
