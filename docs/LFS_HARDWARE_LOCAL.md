# Local Microkit block-persistence integration

Baseline: main `88798c92d5d684fce0966619874940b57592ed42`.
Initially developed and verified locally; now integrated into the public
v0.1.0 native persistence milestone. The filename is retained for existing links.

## Implementation

- `poc/storage-core` retains its `no_std` bounded block contract; `read_block`,
  `write_block` and `fsync` are explicit aliases for the existing primitive API.
- `virtio_block.rs`: modern VirtIO-MMIO, queue size 8, one request outstanding,
  512-byte blocks, dedicated eight-sector extent, negotiated FLUSH support.
  Completion ID/status and configuration generation are checked. Poll timeout
  poisons the device; no descriptor/DMA buffer is reused after an ambiguous I/O.
- `hardware.system`: only the LFS protection domain maps the VirtIO registers
  and uncached contiguous DMA region. Microkit supplies its physical address;
  no physical RAM address is guessed. PCB and Executor have no block endpoint.
- `disk_ledger.rs`: persist all bounded receipt records and note bytes in the
  two-slot storage engine. Authenticate the complete snapshot and each receipt
  with Ed25519 before importing state at boot. The note is recovered from an
  authenticated `vela.note` section in the admitted Wasm payload.
- Reply ordering: authorize admission, execute Wasm, stage signed state, publish
  it through ordered block writes and FLUSH, then release the receipt to PCB.
  The signature is prepared before publication so the authenticated snapshot
  can be recovered; it is never returned as a durable success before flush.
- PCB verifies the LFS signature and records the receipt in its bounded notary.
  An identical retry after guest restart returns the stored receipt and does
  not rewrite the disk. Persistent notary storage itself is not added here.

This is a dedicated test-domain backend, not complete native LFS: immutable
BLAKE3 objects, encryption, queries and production write authorization remain
separate integration work. The existing C4 receipt attests epoch/request/result/
authority/sequence; the whole archive signature authenticates the note bytes.
The receipt wire does not gain a separate note-content digest in this change.

## Real proof and reproduction

The hardware verifier launches two distinct QEMU processes against the same
raw image with `cache=writeback`. First boot checks 41 serial markers (the
existing 37-marker flow plus four disk checks, replacing its volatile readiness
marker). Second boot checks five recovery/notary markers and requires a
byte-identical SHA-256 of the disk before and after the idempotent retry.

Windows entry point:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-bridge-e2e.ps1 -Hardware
```

Requires Docker in WSL Debian and host Rust. The wrapper builds the Microkit
foundation image automatically when missing, creates local test keys and restores the
original public anchor in `finally`. The verifier refuses to overwrite an
existing test image. Seeds, disk image and image binaries remain under `.local`
or local Docker storage, not tracked Git files. Public serial evidence is in
`docs/reports/vela-hardware-*`.

Sources used for the driver contract:
[VirtIO 1.2 block device and flush semantics](https://docs.oasis-open.org/virtio/virtio/v1.2/virtio-v1.2.html)
and [Microkit mappings and region_paddr](https://docs.sel4.systems/projects/microkit/manual/latest/).
The build uses the existing pinned Microkit 2.3.0 SDK, not the latest manual's
SDK version; the accepted XML and actual guest execution are the compatibility
evidence.

## Scope limits

- UserSelection/Biscuit verification are the existing host-generated fixture,
  not interactive Glass input or an in-guest Biscuit verifier. Existing LFS_READ
  and execution admission authorize the test result pipeline, not a newly
  implemented production LFS_WRITE grant.
- The second boot intentionally reuses fixture epoch/request identity. Durable
  LFS idempotency is tested; native Executor admission replay state across
  boots, real boot epochs, absolute expiry and revocation are not solved.
- PCB checks relative TTL before the storage call; no new commit-time deadline
  check in the block driver is claimed.
- Wasm returns the typed value 21. The note comes from its signed custom section,
  not an arbitrary Wasm output buffer or the native x86 Glass editor.
- Two process terminations/restarts prove QEMU file-backend persistence after
  acknowledged flush. They do not simulate host power loss, disk firmware lies,
  DMA isolation by an IOMMU, or qualify physical hardware.
- The x86 VirtualBox preview is untouched; its UI notes remain RAM-only.
- No full-system RAM, latency or throughput claim is made.

## Verified local results — 2026-10-12

- Hardware build image: `f81227ca30f74511bcd51cf7b471e31f721f8749c93c634720e9d1ca20347e30`.
- Cold boot 1: **41/41** required markers, no panic/FAULT.
- Cold boot 2: **5/5** recovery/notary markers, no panic/FAULT.
- Recovered note: `appunti: design bridge IPC` (26 bytes).
- Recovered receipt: epoch 1, request 12, value 21, sequence 1; authenticated and notarized again.
- Disk SHA-256 identical after both boots: `044bc511f59c435666e687ec4cc8040e7e0a785189e180ac1f9a621c41a0267d`.
- Standard non-hardware E2E regression: **37/37** markers passed, 164s including build.
- Receipt tests: **10 passed**; storage tests: **6 entries passed**, including the subprocess helper and 2,565 torn-sector cases.
- Clippy for storage with warnings denied passed; Python/PowerShell entry points passed syntax checks.
- ELF segment W^X checks passed for PCB, Executor and LFS in the hardware build.

Evidence: [first boot](reports/vela-hardware-boot-1.log),
[second boot](reports/vela-hardware-boot-2.log),
[build](reports/vela-hardware-build.log),
[result hashes](reports/vela-hardware-result.json),
[standard regression](reports/vela-hardware-standard-regression.log).

These results establish the local hardware-backed test-domain persistence path.
They do not close native Glass integration or the consumer OS gate.
