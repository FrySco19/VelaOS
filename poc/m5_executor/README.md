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
- No WASI, host functions, network, filesystem, token verifier, PCB channel, or
  shared-memory page is wired yet. In particular, Biscuit tokens are not passed
  to the guest; task ABI and trusted payload handling remain future work.

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

This is the engine/bootstrap phase only. It does not connect to the PCB or
consume dynamic task modules. Before integration, add a bounded validated task
envelope, explicit shared-page ownership/lifecycle, malformed-module and
forbidden-import tests, fuel exhaustion over IPC, cancellation/revocation
semantics, and measurements on the target. QEMU boot does not qualify physical
hardware, whole-system RAM, or a hard real-time latency guarantee.
