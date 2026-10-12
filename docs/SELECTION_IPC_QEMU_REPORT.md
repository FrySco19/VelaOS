# UserSelection PCB IPC — QEMU evidence, 2026-10-12

Baseline checkpoint: `52f2765155b2664e2006fb88a8eaa798c122f818`.
Execution: Windows host fixture preparation; Docker builds and QEMU executed
through Debian WSL. Pinned Microkit SDK 2.3.0 and nightly-2025-10-20 toolchain.

## Results

- Rebuilt baseline E2E image from scratch: 37 required markers, no panic/fault.
- Built combined six-domain Microkit AArch64 image. Original PCB/Executor/LFS
  remain unchanged; three test PDs add input, selection PCB and authorization sink.
- Nine additional markers verified exactly once; 46 total in the combined boot.
- Shared no_std `ed25519-dalek` verifier accepts the signed UserSelection.
- One 40-byte mock authorization received through the PCB-to-sink shared page.
- Corrupted signature rejected as InvalidAttestation; altered hash rejected as
  PlanChanged. No additional authorization; sink asserts exactly one delivery.
- QEMU exited with 124 only because the runner ends the verified idle guest after
  30 seconds. This is expected, not an execution failure.

Actual boot excerpt:

```text
Bootstrapping kernel
Booting all finished, dropped to user space
SELECTION PCB: ready; no_std shared verifier ed25519-dalek
SELECTION INPUT: publish signed valid UserSelection
SELECTION PCB: VALID Ed25519; emitted bounded TEST authorization
SELECTION EXECUTOR SINK: TEST authorization received once; hash verified
SELECTION INPUT: publish bad signature
SELECTION PCB: REJECT bad signature; authority=0
SELECTION INPUT: publish altered hash
SELECTION PCB: REJECT altered hash; authority=0
SELECTION PROBE PASS: valid approval, bad signature and altered hash over IPC
VelaOS E2E: ALL E2E CHECKS PASSED; intent -> selection -> grant -> Wasm 21 -> receipt notarized
```

## Artifacts and reproduction

Local baseline serial log: `.local/e2e/verify-bridge-e2e.log`.
Combined build/boot/results: `.local/selection-probe/run-2/`.
Exported `selection-loader.img`: 7,066,156 bytes.
SHA-256: `0e1bdcd79a21539a9331ca7358558bf04ad285bc909eb8ccdff711c01f1bac92`.

WSL test invocation from the repository root:

```text
python3 scripts/verify-selection-ipc.py --input-seed .local/selection-probe/input.seed --out .local/selection-probe/run-2
```

Fixture preparation and baseline build instructions are in
`poc/m5_executor/selection-probe/README.md`.

## Scope

This proves actual Microkit notification/shared-buffer transport and the shared
cryptographic verifier in a test PCB domain. The sender is a fixture, not physical
human input. The emitted token is a mock, never dispatched into production Wasm.
Production Biscuit issuance, policy checks, expiry/revocation, durable replay
consumption, trusted preview and the native UI remain separate work.

The baseline LFS in this image is an in-memory ledger. This run does not repeat
the separate VirtIO cold-boot persistence proof and is not laptop installation
qualification. Test sender ELFs contain fixture private keys and must not ship.
