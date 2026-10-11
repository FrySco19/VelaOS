# vela-executor-policy — host PDP (Gate 1, M-G1a)

Host-only (`std`) policy decision point in front of the M5 Executor bridge.

```
AdmitRequest{biscuit, caller, resource, operation, request_id, payload, fuel}
  -> bounds/replay checks -> Biscuit::from(token, pinned root) -> authorizer
  -> ALLOW: bridge::signed_frame(PCB Ed25519 key, ...)   (wire format v2, unchanged)
  -> DENY : {Malformed | Authentication | PolicyDenied | Scope | Replay}, nothing signed
```

* Authorizer facts: `caller`, `resource`, `operation`, `payload_bytes`, `fuel`, `time`;
  policy `allow if caller($c), resource($r), operation($o), right($c,$r,$o)`
  (authority-block rights only). Limits as M1/M2: 256 facts, 32 iterations, 250 ms;
  token <= 16 KiB.
* `attenuate()` appends checks for caller/resource/operation, max payload bytes,
  max fuel and expiry. Facts added in attenuation blocks never grant rights.
* The wire protocol is compiled from `../app/src/bridge.rs` via `#[path]` (as the
  keygen tests do); `app/` is not modified and gets no Biscuit dependency.
* The Executor stays an Ed25519 verifier of the PCB admission signature.
  **Biscuit root key != PCB signing key** (`PolicyDecisionPoint::new` rejects reuse).
* The PDP links no Wasm runtime: a deny can never execute a task.

Not covered (still open): in-guest Biscuit (biscuit-auth 6.0.0 is `std`-only),
durable/cross-boot replay state (the request-id high-water mark is in memory),
revocation source, trusted clock, envelope v0 fields (`authority_digest`, epoch),
LFS receipt. No QEMU `verify-bridge` rerun is implied by these tests.

```
cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml
```