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

## Gate C root-task boot proof

Microkit's static channel model does not dynamically pass capabilities between
protection domains. The separate `pcb_root` package uses the pinned
`sel4-root-task` support to boot a root-task PCB proof. It creates a live child
with its own CSpace and VSpace, mints a badged send-only endpoint capability
into the child's CSpace, receives and verifies the child's badge, replies, then
deletes the grant and suspends the child. The child uses a configured MCS
scheduling context. The pinned `sel4-kernel-loader-add-payload` currently needs
two small source compatibility fixes; `patch-payload-helper.sh` applies them
only to the exact pinned checkout and is safe to rerun.

Build and boot the custom root task separately from the default Microkit image:

```powershell
docker build -f poc/m5_sel4/Dockerfile -t velaos-gate-c poc/m5_sel4
docker run --rm --entrypoint make velaos-gate-c run-gate-c
```

Expected serial markers include `Gate C: minted badged send capability into
live recipient CSpace`, `Gate C: recipient used the grant; badge verified`, and
`Gate C: revoked the recipient endpoint capability`.

This remains a proof of the kernel capability-derivation and IPC path, not a
complete PCB policy implementation. The root-task PCB uses a fixed badge in
place of Biscuit validation; configurable grant profiles, token validation,
timed lease expiry, attempts to use revoked caps, and authorization deny tests
are not implemented.

## Gate D bounded Wasm proof

The `executor` protection domain uses Wasmi 2.0.0 with default features
disabled, no `std`, no WASI, and an empty linker (no host imports). The embedded
probe runs with a 10-million-unit fuel budget, a 64 KiB linear-memory limit, a
16 KiB value-stack limit, and a recursion-depth limit of 64. It also exercises
an infinite loop with a small fuel budget and checks that an attempted
second-page memory growth is denied.

Build and boot the QEMU image:

```powershell
docker build -f poc/m5_sel4/Dockerfile -t velaos-gate-d poc/m5_sel4
docker run --rm -it velaos-gate-d
```

The executor prints `VelaOS Gate D: fuel exhaustion and 64 KiB memory cap
verified` after both bounds pass. This is not a task-ingress or full Gate D
implementation: epoch deadlines, per-task IPC payload limits, concurrency,
cancellation, fresh-store cleanup, and bus/LFS integration remain.

## Gate E protected-call flow

The five-domain image wires protected-call channels from Orchestrator to PCB,
PCB to Bus, Bus to Executor, and Bus to LFS. The fixed intent is authorized by
a demonstration label check, the executor runs the bounded Wasmi sample and
returns `42`, and the LFS PD records that value in its volatile in-memory state.
The LFS stub does not persist to disk; this flow is not Biscuit validation or
the dynamically delegated Gate C capability path.

```powershell
docker build -f poc/m5_sel4/Dockerfile -t velaos-gate-e poc/m5_sel4
docker run --rm -it velaos-gate-e
```

QEMU serial output should include `VelaOS Gate E: PCB authorized fixed
intent`, `VelaOS Gate E: executor returned Wasm result 42`,
`VelaOS Gate E: LFS recorded volatile result 42`, and `VelaOS Gate E: intent
completed with result 42`. The labels and single-flow route are fixture-only;
deny cases, untrusted input, durable object lookup, and restore/rollback are
not covered.
