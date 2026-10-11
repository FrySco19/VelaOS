# seL4 PCB–Executor bridge: Phase 2 and Phase 3

Two AArch64 Microkit domains exchange authenticated Wasm tasks through a 4 KiB page and protected IPC calls. The Executor uses Wasmi 2.0.0 as a no_std interpreter, with no WASI or host imports. Ed25519 verification uses ed25519-compact 2.6.0 with default features disabled.

## Protocol and isolation

PCB maps the shared page read/write; Executor maps it read-only. Neither mapping is executable. Before verification, Executor captures an owned 4 KiB snapshot. Both signature verification and Wasm execution use that snapshot, preventing producer modifications from changing the authenticated executable bytes.

The v2 frame contains a 20-byte header, 32-byte scope/budget claims, 64-byte signature and bounded payload. The domain-separated signature covers the complete header, claims and exact payload. Claims bind the Executor audience, execute operation, request ID, payload length, fuel and memory quota. Invalid bounds, signatures, scope and replay are rejected before instantiation. Valid request IDs are consumed before execution, including failed executions.

Channel 0 is a Microkit protected call: Executor priority 101, PCB priority 100. Replies use label 0x702 and four message registers: status, result as u32 bits, request ID, execution-attempt counter. Status codes: 0 success, 1 malformed frame, 2 authentication failure, 3 scope failure, 4 replay, 5 Wasm rejection/trap, 6 fuel exhaustion. The request label is 0x701 with no message registers; task bytes travel through the shared page.

## Runtime bounds

- Signed fuel quota: 1 through 10,000,000.
- Signed linear-memory quota: 1 through 65,536 bytes; one memory, one instance, no tables.
- Interpreter stack: 16 KiB; recursion depth: 64.
- Native domain stacks: 64 KiB each; Executor heap: 2 MiB.
- Empty linker; modules declaring any import are rejected before instantiation.
- ELF checks reject writable executable PT_LOAD segments for both binaries.

The private snapshot is an intentional bounded copy, not end-to-end zero-copy. Fuel meters guest execution, not all parsing work or scheduler latency. These limits are not measurements of whole-system RAM or hard real-time guarantees.

## Reproduce

From the repository root, with Rust and Docker available:

```powershell
New-Item -ItemType Directory -Force .local | Out-Null
cargo run --locked --manifest-path poc/m5_executor/keygen/Cargo.toml -- .local/m5-executor.seed poc/m5_executor/app/src/trusted_key.rs
icacls .local/m5-executor.seed /inheritance:r /grant:r "$($env:USERNAME):(F)"
cargo test --locked --manifest-path poc/m5_executor/keygen/Cargo.toml
docker build --secret id=pcb_seed,src=.local/m5-executor.seed -f poc/m5_executor/Dockerfile -t velaos-executor-phase23 poc/m5_executor
docker run --rm --entrypoint timeout velaos-executor-phase23 20s make run
```

The seed is generated with operating-system entropy, retained only in the ignored local directory, and supplied to compilation through a BuildKit secret mount. Only the public trust anchor is committed. Regenerating an anchor requires rebuilding both domains. The build pins Microkit 2.3.0, rust-sel4 and nightly-2025-10-20; its output is build/loader.img inside the image.

A timeout exit alone is not a pass: require the final ALL CHECKS PASSED marker and no panic or guest fault. See [verification report](PHASE_2_3_REPORT.md) and [captured serial output](phase-2-3-boot.log).

## Remaining production work

This is a scoped Ed25519 token proof, not a Biscuit/Datalog verifier. Replay tracking lasts only for the current boot; persistent replay state, clock-based TTL, revocation, cancellation and production key provisioning remain future work. The local test PCB ELF embeds its signing seed: do not distribute that binary as a production secret-bearing image. BuildKit protects the raw seed mount, not secret material intentionally compiled into PCB.

QEMU execution verifies this bridge and its rejection cases, not a completed OS, physical hardware qualification or a general proof against every runtime vulnerability. No LLM participates in byte transport or authorization.

## Automated bridge verification

The Rust host runner rebuilds the image, captures complete QEMU output, requires each expected reply exactly once, rejects faults and validates the idle timeout only after completion. After generating the local key, run from the repository root:

```powershell
cargo run --locked --manifest-path poc/m5_executor/keygen/Cargo.toml --bin verify-bridge -- poc/m5_executor .local/m5-executor.seed .local/m5-bridge-repro.log
```

On Windows with Docker accessed through WSL, provide Docker-visible absolute context/seed names and append `--wsl`; the log destination remains a host filename. Run `cargo test --locked --manifest-path poc/m5_executor/keygen/Cargo.toml` separately for descriptor and cryptographic host tests.

See [ownership, acknowledgement and remaining gates](BRIDGE_CONTRACT.md). The single-flight protected call deliberately replaces unreliable notification-only delivery.
