# Gate D — standalone seL4 Executor engine setup

This proof builds one AArch64 Microkit Executor protection domain. It is
deliberately separate from the existing host Wasmtime crate in `poc/m2`: the
seL4 target uses **Wasmi 2.0.0**, a `no_std`-compatible interpreter, not
Wasmtime/Cranelift JIT. There is no runtime code generation and no WASI feature.
The only registered host linker is empty, so modules with imports cannot be
instantiated. The embedded module is a boot-time fixture, not an untrusted task
ingress or a PCB-connected service.

## Build and run

From the repository root with Docker available:

```powershell
docker build -f poc/m5_executor/Dockerfile -t velaos-gate-d poc/m5_executor
docker run --rm -it velaos-gate-d
```

The build uses Ubuntu 22.04, seL4 Microkit SDK 2.3.0 (archive SHA-256 pinned in
the Dockerfile), `rust-sel4` v5.0.0 at its immutable commit, and Rust
`nightly-2025-10-20`. The Microkit `qemu_virt_aarch64` platform builds
`build/loader.img` and boots it in QEMU. The reproducible Docker build and QEMU
boot were verified on 2026-10-11. The build completed for the AArch64 target,
the W^X ELF check passed for all three `PT_LOAD` segments, and QEMU reached
user space. The serial output confirmed the import-free Wasm fixture, WASI
import rejection, fuel exhaustion, and memory-growth denial. QEMU was stopped
after the markers using a 20-second timeout.

The WASI probe declares `wasi_snapshot_preview1::proc_exit` and attempts
instantiation with an empty linker; successful rejection shows this host import
cannot be resolved in this setup. It does not test every WASI import or a
general filesystem/network attack surface.

## Runtime bounds

- Wasmi fuel metering is enabled. The fixture Store receives a 10,000,000 fuel
  budget; a separate 64-fuel infinite-loop probe must trap with `OutOfFuel`.
- The Store limits linear memory to 64 KiB, one memory, one instance, and no
  tables. The fixture verifies that growth beyond the initial page is rejected.
- Wasmi stack height is capped at 16 KiB and recursion depth at 64.
- Fuel bounds guest instruction execution, not scheduler delay, IPC latency, or
  the execution time of future host functions; this is not a hard real-time
  guarantee.
- The executor PD has a 2 MiB Microkit heap. This is a prototype allocation
  bound, not a measured full-system RAM budget or an admission guarantee for
  arbitrary production modules.
  - Phase 1 of the PCB–Executor bridge maps a 4 KiB page read/write in PCB and
    read-only in Executor. PCB publishes a bounded mock descriptor, mock token,
    and payload, then signals Microkit notification channel 0. Executor validates
    descriptor bounds and logs the received lengths. This is IPC scaffolding only:
    no Biscuit cryptographic verification, capability grant, or payload execution
    is performed by the bridge.
  - Bridge build and runtime handshake verified on 2026-10-11: both AArch64 Rust
    binaries compiled, both ELF W^X checks passed, Microkit generated the two-PD
    loader image, and QEMU logged PCB publication followed by Executor receipt of
    request 1 with token length 18 and payload length 17. QEMU was stopped after
    the handshake using a 20-second timeout.
  - A follow-up rebuild after making unsafe writer calls explicit was interrupted
    by a read-only filesystem error in the Rust/BuildKit output paths and a WSL
    daemon disconnect. The explicit unsafe-block change is formatting/safety
    annotation only; a clean rebuild of that final source state remains pending.
  - No WASI, guest host functions, network, filesystem, token verifier, or
    production task ABI is wired yet. The pre-existing embedded Wasmi fixture is
    still run as an independent Executor self-test, not on the shared payload.

## Memory mapping and W^X strategy

Wasmi interprets Wasm bytecode; it does not JIT or request executable pages for
module code. The module and linear memory remain data. Native instruction
execution is confined to the statically linked Executor ELF. The build runs
`check-wx.py` against its ELF program headers and fails if a `PT_LOAD` segment
is both writable and executable. This establishes an ELF-segment check for this
artifact, not a general proof of every seL4 mapping or future allocator path.
No `mprotect`-style permission transition or dynamic executable allocator is
part of this design.

The app heap and Wasm linear memory are separate limits: Store limiter bounds
guest linear memory, while Microkit's PD heap bound limits Wasmi/runtime
allocations. Production shared-memory exchange will require an explicit frame
mapping and typed bounded ABI; it must not expose host pointers or implicit
WASI imports.

## Scope and next gate work

The engine self-test and Phase 1 PCB–Executor handshake are separate. Before
cryptographic or runtime integration, add a bounded production task envelope,
explicit shared-page ownership/lifecycle, Biscuit verification and capability
attenuation, malformed-module and forbidden-import tests over IPC,
cancellation/revocation semantics, and target measurements. QEMU boot does not
qualify physical hardware, whole-system RAM, or a hard real-time latency
guarantee.
