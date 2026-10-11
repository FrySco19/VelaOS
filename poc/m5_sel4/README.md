# seL4/Microkit protection-domain scaffold

This scaffold builds five distinct Rust protection domains with seL4 Microkit
2.3.0 for `qemu_virt_aarch64`, packages them as one Microkit loader image, and
starts QEMU. Microkit creates a separate CSpace and VSpace for each declared
protection domain. The Gate E prototype now uses static protected-call channels
to execute a fixed Wasm intent and store result 42 in volatile LFS state.
Dynamic grant routing, durable storage and negative isolation tests remain open.

Domains initialize and enter Microkit's handler loop; the orchestrator submits
the fixed demonstration intent during initialization.
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

This verifies five statically declared Microkit protection domains and a fixed
connected flow on QEMU. It is not the full M5 architecture, durable M1 storage,
SLM integration, dynamic capability routing or hardware qualification.

The bounded CI smoke test is `docker run --rm --entrypoint python3 velaos-gate-a
smoke_boot.py build/loader.img`. It requires the Wasm fuel/memory proof and the
completed intent markers within 60 seconds, and terminates QEMU afterward.
Missing markers, excessive output or detected faults fail the test. The 2 GiB
configured guest capacity is not an idle RAM measurement.

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

## Gate F timing preparation

The fixed flow emits `BEGIN`/`END` serial markers around the protected calls
and Wasm invocation. Run the host-side monotonic-clock observer with:

```powershell
docker build -f poc/m5_sel4/Dockerfile -t velaos-gate-f poc/m5_sel4
docker run --rm --entrypoint make velaos-gate-f benchmark
```

The observer measures when marker lines arrive from QEMU's serial output. Its
intervals include guest debug-print work, QEMU emulation, and host I/O/scheduling;
they are not guest-only IPC timings or target-hardware measurements. The
architectural counter registers are trapped at EL0 by the pinned kernel, so the
guest does not read `CNTVCT_EL0`. One diagnostic run and its caveats are in the
repository's [Gate F/G report](../../GATE_F_G_REPORT.md). Exit QEMU with
`Ctrl+A`, then `X`.
