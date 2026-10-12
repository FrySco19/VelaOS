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
## M-G1b — Candidate != Authority (`src/grant.rs`)

```
recall -> [Candidate{handle, version, score}]        (data only, no token, no right)
trusted UI -> UserSelection{handle, expected_version} (TrustedSelectionChannel, single-use id)
GrantIssuer::issue_read_grant_at(GrantRequest{caller, requested, candidates, selection, ttl, planner_note})
  -> GRANT: PDP-held delegation `right(caller,"lfs","read")` + ONE attenuation block:
            operation("read"), caller, handle(bytes32), object_version, time <= not_after
  -> DENY : nothing minted
verify_read_grant_at(token, pinned root, caller, handle, version, now) -> Ok | C5 code
```

Decision order (deny codes = CONTRACTS_v0 C5 via `DenyCode::contract_code()`):

| Condition | Code |
| --- | --- |
| no `UserSelection` (any planner/LLM text is ignored, never parsed) | `POLICY_DENIED` = 10 |
| requested handle != selected handle, or selected handle not among candidates | `SCOPE` = 3 |
| candidate version != `expected_version` | `VERSION_CONFLICT` = 12 |
| ttl = 0 or ttl > 120 s; verifier: `time > not_after` | `EXPIRED` = 7 |
| selection id already consumed | `REPLAY` = 4 |
| verifier: wrong root | `AUTH` = 2; bare delegation / extra blocks -> `POLICY_DENIED` |

Notes / limits:
* `DenyCode` gained `Expired` and `VersionConflict` (additive). The M-G1a Executor
  admission path is unchanged (its expired attenuation still reports `PolicyDenied`).
* The verifier maps failed checks of block 1 by fixed check position, not by text.
* The delegation token is PDP secret material: anyone holding it could attenuate it.
  It is in-memory only; storage/rotation is open.
* `UserSelection` has private fields and one constructor (`TrustedSelectionChannel`);
  in one process this is a structural guard, not isolation. The real boundary is which
  domain owns the channel (compositor/shell, never the planner).
* TTL cap 120 s matches the concept and M1 handle lifetime (`1..=120`); CONTRACTS_v0 C1
  `not_after` has no numeric cap yet -> needs an ADR (v0.x, additive).
* Not wired to M1 `Lfs::open` (M1 authorizes `right(caller, kind, "read")` without
  `handle`/`object_version` facts, so these grants would be denied there by design);
  no QEMU, wire v2 or `app/` change; clock is host wall clock (no trusted clock yet).

### Signals (v0)

Context may emit observational signals that Intent and Glass may display and that the PDP may use only to deny or to constrain scope. A signal never grants, signs, or stands in for `UserSelection`. The normative list and the always-out set are in [`docs/policy/SIGNALS_CONTRACT_v0.md`](../../../docs/policy/SIGNALS_CONTRACT_v0.md). This note extends M-G1b; it does not change `DenyCode`, `GrantIssuer` decision order, or Executor wire v2.
