# Host bridge increment 2 — executed 2026-10-13

Branch: `vela/virtio-bridge-completion`.
Includes increment 1 protocol/Ingress sources plus the framed PCB and std broker.

## Verified

- Windows native broker compiles; Clippy with warnings denied passes.
- Protocol: three tests pass (maximum frame, partial/coalesced input and rejection).
- AArch64 Ingress and framed PCB compile and pass ELF W^X segment checks.
- Real Windows broker sends 170-byte Approval frames to loopback port 7991.
- QEMU runs in WSL; a single-port VirtIO console forwards bytes through queue 0,
  IRQ 79 and Ingress shared mailbox into the Microkit PCB.
- Guest validates one signed request and emits one mock authorization to the sink.
- Corrupted signature and altered hash are rejected; replay is rejected in-session.
- All 37 baseline E2E markers occur exactly once in the final captured boot.
- No panic/FAULT in that boot. Local logs/results: `.local/host-bridge/boot.log`
  and `.local/host-bridge/result.json`.

```text
SELECTION PCB: VALID Ed25519; framed host approval; TEST authority=1
SELECTION EXECUTOR SINK: TEST authorization received once; hash verified
SELECTION PCB: REJECT CRYPTO_OR_CONTEXT; authority=0
SELECTION PCB: REJECT CRYPTO_OR_CONTEXT; authority=0
SELECTION PCB: REJECT REPLAY; authority=0
```

QEMU qtree verifies console MMIO 0xa003e00. Its dumped DTB contains
`reg=(0,0xa003e00,0,0x200)` and `interrupts=(0,47,1)`: SPI 47 -> IRQ 79,
edge-triggered. The SDF maps its containing page at 0xa003000; the qemu-port0
feature selects offset 0xe00. DMA is fixed/contiguous through region_paddr.

## Reproduce

Prepare directories, then build the Windows broker and generate a fresh local
test key with the CLI `keygen <seed-path> <public-rs-path>`. The public Rust file
must be `selection-probe/generated/host_public.rs` when building the guest image.
The seed stays on Windows and is not embedded in any guest ELF.

Build `poc/m5_executor/Dockerfile.host-bridge` from the repository root after
the baseline E2E image exists. Launch the image with QEMU AArch64, modern MMIO,
`virtio-serial-device` plus `virtconsole` port 0, and a loopback socket chardev.
Load `/work/build/host-bridge-loader.img`; keep the debug UART separate.

Broker invocation used on Windows:

```text
vela-host-broker.exe send tcp:127.0.0.1:7991 .local/host-bridge/broker.seed valid
vela-host-broker.exe send tcp:127.0.0.1:7991 .local/host-bridge/broker.seed bad-signature
vela-host-broker.exe send tcp:127.0.0.1:7991 .local/host-bridge/broker.seed bad-hash
vela-host-broker.exe send tcp:127.0.0.1:7991 .local/host-bridge/broker.seed valid
```

The final command must yield REPLAY. Restart the guest to reset this test profile.
Unix and Windows named-pipe backends are implemented in the broker but were not
the endpoints exercised in this Windows/WSL run. Named MULTIPORT virtserialport
negotiation remains unimplemented; this proof uses console port 0 explicitly.

## Limits

Fixed test operation/session/revision, enrolled test key, volatile one-shot state
and mock authority. This is not native human consent, Hello/FIDO2, an enclave,
production Biscuit policy, durable replay prevention or a Glass save transaction.
Cross-boot replay protection requires a fresh session and durable state.
