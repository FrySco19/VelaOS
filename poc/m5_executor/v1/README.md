# Native note bridge — verified development slice

Verified on 2026-10-13: real Glass browser → Windows broker → VirtIO RX →
Microkit PCB → signed scoped capability → no-WASI Wasmi → ObjectStore/VirtIO
flush and read-back → signed LFS receipt → VirtIO TX → broker verification →
Glass `Completed`.

This is a development note workflow, not an installable VelaOS 1.0 release.
Storage is the existing ObjectStore, not LittleFS. A note is at most 1024 UTF-8
bytes and replaces the single logical object `/lfs/notes/note1.txt` on a dedicated
4096-byte test disk. Never use an existing user disk as that extent.

## Consent and keys

The current Windows host reports `UserConsentVerifierAvailability::DeviceNotPresent`
(1). The recorded full-flow tests explicitly used `--mock-unavailable`, producing
`MockUnavailableDevelopment`. This is synthetic development consent, not a human
attestation or a verified Windows PIN. Ed25519 approval, capability and receipt
signatures are real. Native-only mode fails closed; console development mode asks
for `APPROVE` in an interactive terminal. Cancellation, policy denial and a busy
verifier never select the fallback. No Windows PIN is read by the broker.

`RequestVerificationAsync` exists on Win10 but requires a UWP UI context. A plain
desktop executable cannot claim that this call alone makes Windows Hello work.
The real available-verifier path is not qualified on this host.

Local enrollment seeds are development secrets. PCB/LFS seeds are embedded in the
local demo ELF images via BuildKit secret mounts. This is not TPM-backed production
custody. Do not publish those images or seeds. The broker pins enrolled PCB and
LFS public roots; Glass has no signing key. Browser-to-broker identity is a local
development trust boundary, not a trusted Microkit input surface.

## Build and run

Prerequisites: Rust, Node, WSL Debian, Docker, and the previously built pinned
`velaos-executor-e2e:latest` SDK image. This run did not prove clean-machine setup.
Run from the repository root. Enrollment files used by this machine live under
`.local/v1`; the broker seed is `.local/host-bridge/broker.seed` and the enrolled
Rust public constant is `poc/m5_executor/selection-probe/generated/host_public.rs`.

```text
cargo build --manifest-path poc/host-broker/Cargo.toml
cargo build --manifest-path poc/product/glass/Cargo.toml
docker build --secret id=pcb_seed,src=.local/v1/pcb.seed --secret id=lfs_seed,src=.local/v1/lfs.seed --secret id=pcb_public,src=.local/v1/pcb.public --secret id=broker_public,src=poc/m5_executor/selection-probe/generated/host_public.rs -f poc/m5_executor/Dockerfile.v1 -t velaos-v1 .
node scripts/run-v1-host.cjs --mock-unavailable
```

Run the Docker command inside WSL. Open `http://127.0.0.1:7882/studio`.
Omit the development flag to require real native consent. For an interactive
console fallback, run the broker directly in a terminal:

```text
poc/host-broker/target/debug/vela-host-broker.exe serve .local/host-bridge/broker.seed .local/v1/pcb.public .local/v1/lfs.public --console-development
```

The launcher keeps a WSL client alive because background-only WSL shut down on
this host. Its PIDs are recorded in `.local/v1/running.json`. An already-running
broker is reused; stop it explicitly before changing its consent mode or roots.
QEMU has 2 GiB configured for this SDK's loader address layout. This is not an
idle-RAM measurement of VelaOS.

## Recorded checks

```text
cargo test --manifest-path poc/note-action/Cargo.toml
node poc/host-broker/tests/e2e-ui.cjs .local/v1
node poc/host-broker/tests/negative.cjs .local/v1 .local/host-bridge/broker.seed
```

The browser tests require Playwright and Edge. No WebSocket or guest response is
mocked. Negative signing uses the enrolled development seed only in the test.
Passed: durable file reopen, capability tampering, expiry including expiry before
the actual host write, duplicate operation rejection, broker hash rejection,
native PCB rejection of altered approval hashes and signatures, unchanged disk
after rejected requests, and receipt returned to Glass.

Two independent QEMU processes used the same disk. The later boot authenticated
the previous note before another successful save. Actual final logs include:

```text
V1 LFS: AUTHENTICATED AFTER REBOOT note=Vela real browser → Ed25519 → Microkit → Wasm → durable VirtIO note. 2026-10-13T13:58:03.952Z
V1 LFS: recovered generation=5
V1 PCB: VALID Ed25519; scoped signed capability issued
V1 EXECUTOR: Wasm executed; VirtIO flush/readback complete; signed receipt generation=6
V1 INGRESS: response queued on real VirtIO TX
```

The broker/browser verified request `1791900001956`, receipt length 170 bytes,
state `Completed`, consent `MockUnavailableDevelopment`. Evidence is in
`.local/v1/boot-final.log`, `boot-verified-flow.log`, `broker-final.log`,
`ui-result.json`, `negative-result.json` and `glass-receipt.png`.

## Remaining boundaries

- Capabilities are purpose-specific Ed25519 tokens, not Biscuit tokens.
- Epoch 1 is a deployment generation, not an authenticated fresh guest boot nonce.
  ObjectStore enforces the persisted operation counter for this single object;
  disk rollback attacks and global multi-object replay are not solved here.
- In this slice Executor owns the block adapter and LFS signing key; a separate
  LFS protection domain and PCB-side notary verification are not implemented.
- Wasm is a pinned note writer and has a 64 KiB linear memory cap and 10,000 fuel.
  Fuel and bounded polling are not proofs of a real-time deadline.
- Guest errors can time out without a success receipt. A timeout can have an
  ambiguous persistence outcome; it must not be represented as a successful save.
- Glass displays the broker-verified receipt but its host lifecycle backend has
  not yet adopted guest authority events. Audio, Wi-Fi and dense M0 are not wired.
- No hardware laptop boot, installer, production Hello/FIDO2 enrollment, revocation,
  protected key storage, or complete OS memory/performance claim is made.
