# Vela Soft Task 2: immutable notes

Implemented on `vela/soft-mvp`.

- Dedicated append-only ObjectStore mode; no_std, bounded buffers, no heap index.
- Every new note gets an opaque document ID and an immutable version ID.
- Updates bind the parent version and document ID into the approved manifest,
  signed capability and receipt. Stale or cross-document updates are denied.
- ReadRequest (8) / ReadResponse (9) support exact version reads. Signed catalog
  pages use fresh nonces, transaction binding and separate read/list domains.
- Glass opens a timeline version in its central text input. Approval creates a
  new version; older versions and receipts remain available after restart.
- Legacy single-note disk is backed up before extending the dedicated extent.
  Migration requires the VM to be stopped. No arbitrary host disk is accepted.
- Broker resumes admission IDs from authenticated persisted receipts.

## Verification

Rust protocol, durable notes and storage tests passed. New storage tests cover
25 torn append cases, capacity exhaustion and rejection of duplicate object IDs.
The durable note test covers independent documents, version linkage, reopen,
stale updates, wrong-document updates, read proof tampering and replay.

The AArch64 Microkit image builds and boots. The real-browser integration test
creates two documents, reopens one, authorizes an update, reads the original and
new version after page reload and again after restarting QEMU.

Run: `node poc/host-broker/tests/run-crud-fixture.cjs`.
The fixture uses a separate disk copy and explicitly labelled synthetic host
consent (`MockUnavailableDevelopment`). Ed25519, VirtIO, Wasm, disk writes and
receipt verification are real. Finally it restores the normal native-only
runtime. This does not establish FIDO2 authentication; that remains Task 3.

## Explicit bounds

This increment supports 64 persisted slots total, including retained versions
and any legacy slots, with at most 1024 UTF-8 bytes per note. The configured disk
is 128 KiB. Full storage rejects further writes; it never deletes history to
make room. This bounded implementation is not an unlimited document database.
Dates remain derived from signed admission request IDs, not a trusted RTC.
There is no deletion UI, large attachment support or two-week reliability claim.
