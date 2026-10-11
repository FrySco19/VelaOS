# C–D bridge lifecycle and remaining integration gates

Executor-side obligations (snapshot, Ed25519-only admission, limits, no imports/WASI, u32 result, C5 status mapping, replay, fuel vs deadline) are versioned in [EXECUTOR_CONTRACT.md](EXECUTOR_CONTRACT.md) (EXEC-v2.1) and asserted by host tests in `executor-host/` with fixtures in `fixtures/`.

## Implemented v2 ownership and acknowledgement

Only PCB owns write access to the one shared page. There is one caller and at most one request in flight. PCB publishes all bytes before protected call label 0x701, then must not reuse or mutate the page until the synchronous reply (label 0x702, four registers). Executor captures an owned snapshot before any parsing/verification and never executes bytes read again from shared storage. PCB verifies response label/count, status and matching request ID for every successfully parsed frame. Malformed replies have no trusted request ID and report status 1; the single in-flight call provides correlation for that error.

A successful or failed task reply is the acknowledgement permitting page reuse. Valid authenticated request IDs are consumed before instantiation. Repeated calls with a consumed ID are rejected without execution; invalid signatures do not consume IDs. IDs cannot wrap or restart within an Executor session. No seL4 notification carries task delivery here: notification coalescing/duplicates cannot lose requests because notification dispatch is not the ingress. An asynchronous future ingress needs a bounded sequence-numbered queue and a separately acknowledged consumer cursor; notifications may then be hints only.

If Executor crashes or never replies, PCB must not assume an acknowledgement, overwrite the page or replay a side-effecting task. Recovery needs a separate supervisor, authenticated session epoch and durable admission/result state. These are not implemented. Fuel alone does not bound module parsing or ensure a wall-clock response deadline.

Fuel exhaustion itself is a reply, not a crash (R1, closed 2026-10-12): the Executor's Wasmi is built with `portable-dispatch`, so a task admitted at `MAX_FUEL` returns status 6 over IPC from the 64 KiB native stack and the Executor keeps serving. `verify-bridge` asserts this with PCB requests 9 (`LOOP_MODULE` at `MAX_FUEL`, `status=6 value=0 attempts=4`, marker `MAX_FUEL stack-safe status=6`) and 10 (`status=0 value=7 attempts=5`); log `docs/reports/m5-verify-bridge-2026-10-12-r1-max-fuel.log`. Other domain faults (bugs, heap exhaustion) still produce no reply and need the supervisor above.

## Required authorization boundary (not yet implemented)

PCB must verify the original Biscuit chain against pinned roots, enforce attenuation and issuer/subject/operation/object scope, and reject unknown facts or checks. It must derive an admission record granting only the permitted object set, operation and resource quotas. The envelope must bind protocol version, session epoch, globally unique request ID, payload digest, authority digest, object/version set, expiry and cancellation/revocation generation. A signature is evidence of PCB admission, not a newly minted seL4 capability.

Executor must independently verify the PCB signature, exact executable-byte binding and supported version/scope, enforce its own hard budget ceilings, reject expired or revoked admissions against a trusted clock/revocation source, and consume the request durably before any effects. Executor need not evaluate Biscuit Datalog if PCB is the designated policy decision point. The pinned PCB key and configured kernel endpoints establish the trust boundary; a supplied token cannot add kernel capabilities. The current Ed25519 v2 proof covers byte/request/operation/budget binding, but has no Biscuit verifier, clock, durable epoch/replay state or revocation source.

## Host PDP (M-G1a)

`poc/m5_executor/policy` is a host-side (`std`) policy decision point: it pins a Biscuit root public key, verifies the (attenuated) token for one request — caller, resource, operation, payload bytes, fuel and expiry via Datalog checks, with M1/M2 authorizer limits — and only on ALLOW calls the unchanged `signed_frame` to emit a wire-format v2 frame signed with the PCB Ed25519 key. On DENY (`Malformed`, `Authentication`, `PolicyDenied` = `POLICY_DENIED`, `Scope`, `Replay`) it signs nothing and links no Wasm runtime. The Biscuit root and PCB signing key must be distinct keys. Host tests (`cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml`) cover allow + Executor-side verification and deny for out-of-scope, wrong root, expired, missing right, quota, block-forged rights, replay and malformed input.

This slice does **not** change `app/`, the guest, `verify-bridge` markers or the v2 frame format. In-guest Biscuit verification remains deferred (`biscuit-auth` 6.0.0 is `std`-only); durable/cross-boot replay state, revocation, trusted clock and envelope v0 binding (`authority_digest`, epoch) remain open, so the "Required authorization boundary" above is only partially met on the host.

## Required LFS and Bus flow (not yet demonstrated)

Define a versioned request envelope carrying an opaque LFS input handle, admission identity, expected object versions and result handle. LFS supplies bounded bytes through the typed data plane; Executor cannot interpret filesystem paths or obtain ambient storage access. Results travel over a bounded typed response to the authorized LFS writer. The writer checks exact handle/operation authority, validates expected version and commits result plus receipt atomically where the storage provider proves that property. Persist receipts keyed by epoch/request before advertising completion; retries resolve the recorded outcome instead of repeating effects. No LLM transfers executable/data bytes or makes authorization decisions.

The present return-7 result reaches PCB registers only. It is not persisted into M1 LFS, and neither full Biscuit admission nor native-target LFS access is available in this proof.

## Evidence required before closure

1. Reproducible build/boot plus descriptor, authentication, scope, replay, fuel (64 and `MAX_FUEL`, R1) and import rejection: implemented in the Rust host runner and guest assertions.
2. Expiry, cross-boot replay, revocation, cancellation and crash/restart tests: pending the real authority/clock/supervisor integration.
3. Request → PCB → Executor → result → LFS including unauthorized result writes and interrupted persistence: pending.
4. RAM accounting for every domain plus kernel/shared mappings, and end-to-end latency distributions on a specified target/workload: pending. QEMU's configured 2 GiB is capacity, not consumption. ELF W^X checks establish properties only of the examined load segments; runtime mappings need separate inspection.

These gates must not be closed using host-only mocks, fuel counts as latency, or a signed fixture as proof of Biscuit policy enforcement.


## M-G1b end-to-end admission: Candidate != Authority (host PDP, 2026-10-12)

`policy/src/pipeline.rs` (`SelectionGatedPdp`) chains the M-G1b grant (`grant.rs`) to the M-G1a admission: the planner/LLM outputs only `Candidate` data; only a `UserSelection{handle, expected_version}` from the trusted channel mints an attenuated Biscuit `LFS_READ` grant (TTL <= 120 s), which is re-verified, hashed (BLAKE3-256 = C1 `authority_digest`) into a `vela.grant` custom section of the Wasm payload (so the unchanged v2 Ed25519 signature authenticates it), and only then is an execute token attenuated to (caller, `executor`, `execute`, payload bytes, fuel, grant `not_after`) and admitted by `PolicyDecisionPoint`. Without a selection: `POLICY_DENIED` (10), no grant, no frame, no Wasm. Tests: `policy/tests/m_g1b_pipeline.rs`. Details: `docs/M_G1B.md`.

## M-G1c State Ledger: LFS receipts (host + shared no_std, 2026-10-12)

`app/src/ledger_wire.rs` (`Ledger`, `Notary`, `Receipt`) is shared by host and guest like `bridge.rs`; `policy/src/ledger.rs` orchestrates it on the host (`E2eHost`, `SimulatedLfs`). After a successful Executor reply the PCB synchronously commits `(epoch, request_id, status, value, authority_digest)`. The writer persists only if the admission frame on the RO page authenticates under the pinned PCB key, names the same request and carries the same `vela.grant` digest (else `POLICY_DENIED` 10, nothing persisted; non-success `MALFORMED` 1). Idempotency key `(epoch, request_id)` (C4): identical retry returns the same Ed25519 receipt (same `seq`), different content is `IDEMPOTENCY_CONFLICT` 13. The PCB `Notary` verifies the receipt under the pinned LFS key and that the body matches its commit, then stores it append-only; forged/foreign/rewritten receipts are rejected. Ledger is **simulated** (in memory, bounded, no durable medium, no cross-boot epoch). Tests: `policy/tests/lfs_receipt.rs`.
