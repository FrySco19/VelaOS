# seL4/Microkit protection-domain scaffold

This scaffold builds five distinct Rust protection domains with seL4 Microkit
2.3.0 for `qemu_virt_aarch64`, packages them as one Microkit loader image, and
starts QEMU. Microkit creates a separate CSpace and VSpace for each declared
protection domain. Gate B currently demonstrates static domain separation and
boot; it does not yet configure service IPC channels, dynamic capability
delegation, or negative isolation tests.

Each domain prints an online marker and then waits in Microkit's handler loop.
Because the debug output is shared, concurrent startup messages may interleave.

## Pinned toolchain

- Ubuntu 22.04 container base.
- seL4 Microkit SDK 2.3.0, Linux x86-64 archive, verified against its published
  SHA-256 digest.
- `rust-sel4` commit `d441a880938209f661201704dfbee7b91c5aa905` (tag v5.0.0).
- Rust nightly `2025-10-20`, matching the `rust-sel4` v5.0.0 toolchain file.
- `aarch64-sel4-microkit` custom target, including the IPC buffer linker symbol
  expected by Microkit. `aarch64-unknown-none` is installed in the image too,
  but is not used as the application target.
- QEMU is supplied by Ubuntu 22.04's `qemu-system-arm` package, which provides
  the `qemu-system-aarch64` executable.

The custom target is included locally so the project does not use a moving
branch to select target configuration. The Rust crate itself is fetched at the
immutable v5.0.0 commit. The app and a small `rustc` wrapper enable the
still-unstable `asm_cfg` feature required by generated assembly and the seL4
crates on this pinned nightly.

## Build and boot from the repository root

```powershell
docker build -f poc/m5_sel4/Dockerfile -t velaos-gate-a poc/m5_sel4
docker run --rm -it velaos-gate-a
```

The Docker build compiles all five protection-domain ELFs and creates
`build/loader.img` inside the image. `docker run` starts QEMU interactively.
Use `Ctrl+A`, then `X` to exit QEMU.

## Scope

This verifies five statically declared Microkit protection domains on the
pinned QEMU virtual platform. It is not the full M5 architecture, a connected
service graph, dynamic capability routing, or hardware qualification.
