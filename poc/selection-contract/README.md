# Shared approval contract

Allocation-free Rust `no_std` wire encoding and Ed25519 verification for host
and future Microkit adapters. This is not a complete PCB and issues no authority.

The canonical signed message is the existing Glass v1 domain separator,
little-endian version, `approve\0`, session binding, revision and SHA-256 digest.
Wire decoding rejects unknown versions, truncated frames and trailing bytes.
Verification uses ed25519-dalek strict verification and rejects weak keys.

The trusted caller must independently pin the displayed proposal, authenticated
principal, fresh session, current revision, policy, scope and expiry. It must
consume approvals durably before effects. Verification alone does not prove a
human click, trusted display, freshness, or authorization.

## Verified on 2026-10-12

- Four contract tests pass, including Compact signing / Dalek verification.
- Glass: six unit and thirteen integration tests pass; one real-model test ignored.
- Clippy with warnings denied passes for contract and Glass.
- `cargo check --locked --lib --target aarch64-unknown-none` passes for this crate.

These checks are not a linked Microkit image, hardware execution, or installation
qualification. The browser remains fail-closed without native attestation.
