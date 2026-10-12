# Native storage core — first integration primitive

Status: **tested component, not integrated into the running OS**.

Rust `no_std`, zero dependencies, no heap, no POSIX calls. A trusted adapter
supplies a dedicated 8-sector (4096-byte) extent through `BlockDevice`.
The implementation retains two versions of one bounded slot (1536 bytes).
The slot key is not an LFS object identifier: immutable BLAKE3 OIDs, encryption,
signed commit history, exact handles and authorization belong above this layer.
This format does not replace the existing receipt ledger.

## Publication protocol

1. Require the expected generation and configured slot key.
2. Invalidate the inactive slot header, then flush.
3. Write three zero-padded data sectors, then flush.
4. Write versioned header and CRC32, then flush.
5. Return the new generation only after that final flush succeeds.

The active slot remains untouched throughout the update. Recovery validates
headers, lengths, generations and payloads before exposing bytes. A valid commit
header whose data is corrupt fails closed. A damaged uncommitted header can be
ignored when another complete version exists. With no valid version and a
nonzero damaged header, opening refuses rather than presenting an empty store.

An I/O error poisons the instance. The outcome can be ambiguous: a device may
persist the final write and still return an error. Reopen and reconcile the
generation; do not blindly repeat an operation. There is no automatic format,
no implicit disk discovery and no destructor performing writes.

CRC32 detects accidental damage; it is **not cryptographic authentication**.
No power-loss guarantee is made for devices that lie about flush completion,
reorder across completed flushes, or corrupt previously flushed sectors.
Two slots are a bounded storage primitive, not the signed append-only LFS log
required by I5. No token replay, receipts, encryption or query index is provided.

## Verification

```text
cargo test --locked --manifest-path poc/storage-core/Cargo.toml
cargo clippy --locked --manifest-path poc/storage-core/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path poc/storage-core/Cargo.toml --target x86_64-unknown-none
cargo check --locked --manifest-path poc/storage-core/Cargo.toml --target aarch64-unknown-none
```

Tests inject all 513 byte prefixes into each of five sector writes (2565 cases),
including reuse of a previously committed slot; reject stale generations,
wrong slot identity, oversized payloads and corrupted committed data; simulate
flush errors; and launch separate writer and reader processes on a real synced
test file. Host filesystem calls exist only in the test adapter.

Cross-target checks establish `no_std` compilation, not a working Microkit block
driver. Next: a dedicated virtual block-device domain with bounded IPC and flush
semantics, then immutable object storage and signed commit publication above it.
