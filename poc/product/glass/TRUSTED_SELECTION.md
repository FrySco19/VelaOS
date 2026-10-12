# Exact-plan selection bridge — host conformance slice

The Studio browser serializes a fixed-order JSON array as UTF-8 bytes: format,
title, description, operation, scope, and visible note draft (null for other
widgets). It computes SHA-256 and posts byte-hex, digest, revision and an empty
signature to `/studio/approve`. Controls are frozen during that request.

Glass independently decodes and bounds the payload to 4096 bytes, recomputes
the digest, checks the current Preview revision, and forwards a typed selection
through `TrustedSelectionChannel`. Default channel returns MISSING_ATTESTATION
or PCB_UNAVAILABLE; it cannot mint authority. Generic `/studio/event` no longer
accepts approve/unavailable, so it cannot impersonate this approval path.

The attestation message is domain-separated and binds protocol version,
operation `approve`, session binding, revision and plan SHA-256 under Ed25519.
SHA-256 here binds a plan; it does not replace BLAKE3 LFS object identity.
The host session binding is derived from the existing per-run transport nonce;
it is not a kernel identity or a durable boot epoch.

`verify_user_selection` is a cryptographic conformance primitive. Native PCB
integration must independently supply the expected session/revision and pinned
Trusted Input public key, and enforce replay, expiry, revocation and policy
before creating any policy selection or authority. Hash consistency alone does
not establish which bytes a human saw. A trusted compositor/provider is required
for that claim; there is deliberately no JavaScript signing hook.

The in-process test channel represents a PCB receiver. A signed test fixture
can be forwarded once, with an HTTP replay refused. No Biscuit, Wasm admission,
execution or LFS receipt is created by this slice. A forwarding acknowledgement
is not authorization. Completion/timeout handling for a real native asynchronous
channel remains integration work.

Validation on 2026-10-12:

- 19 Rust tests passed; the live M0 test was ignored.
- Clippy with warnings denied passed.
- Host browser lifecycle passed in Edge at desktop and mobile viewport sizes,
  including approval denial, disposal and reduced motion, with authority
  counters all zero.
- Tampering with the digest, payload, session, revision, or signer is rejected.
- Changing both payload and digest still fails the original signature.

Commands:

```text
cargo test --locked --offline --manifest-path poc/product/glass/Cargo.toml
cargo clippy --locked --offline --manifest-path poc/product/glass/Cargo.toml --all-targets -- -D warnings
```

This work completes the unfinished host implementation. It does not claim a
working native Trusted Input, Microkit transport or new production authorization.
Studio and the legacy PDP demonstration still share a host process. The latter
retains its existing demo delegations; no new process isolation is established
by the transport trait. Separate native protection domains remain required.
