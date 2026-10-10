# M1 — LFS storage kernel

This is the Rust storage kernel for the LFS + Neural Bus pivot. Objects have content OIDs and opaque handles, never hierarchical addresses. The host backing supplied during unlock is a physical test adapter; it is not an object namespace.

The user explicitly requested M1 while the full M0 evaluation remains open. Work proceeds under that instruction; passing storage tests does not retroactively close M0.

## Implemented

- Strict `vela.net.config.v1` schema and a pure deterministic card extractor. Canonical plaintext is RFC 8785 JSON; identity is its BLAKE3 digest.
- FastCDC chunks (2/8/32 KiB), a random data key per object, XChaCha20-Poly1305 with random 192-bit nonces, and object/chunk-bound associated data. The data key is wrapped with the collection key.
- Whole-object deduplication and repeated-chunk deduplication inside each object. Cross-object shared-chunk deduplication is not implemented: different objects retain separate encryption keys.
- Argon2id collection-key derivation with an externally supplied 16-byte salt. Unlock accepts the derived collection key and a trusted Biscuit public key; it persists neither passwords nor plaintext keys. Key management and the unlock UI are not implemented.
- Encrypted commit bodies, Ed25519 signatures, sequence numbers and a hash-linked append-only log. Card metadata and idempotency bindings are encrypted at rest too.
- Blob durability followed by a durable redb transaction containing both the signed commit and its encrypted idempotency binding. The reply follows the transaction and index update. Reusing a key with different content fails; retries return the original reply after restart.
- Replay verifies every signature, link, referenced encrypted blob, chunk digest, plaintext OID, deterministic card and idempotency binding before exposing the recovered catalog.
- Biscuit signature and policy checks inside every object operation. Read, write and list permissions are separate and scoped to the caller and object kind. Attenuation checks remain enforced.
- Bounded authorization: tokens are at most 16 KiB; Datalog rule-execution limits are configured to 256 facts, 32 iterations and 250 ms. Exhausting a limit denies the request. Standard current-time facts enforce token expiry checks.
- Read-only, caller-bound handles with a MAC, a maximum lifetime of 120 seconds, monotonic expiry and a random boot session. Restart invalidates old handles. Opening always rechecks capability and content integrity.
- Deterministic paginated listing by commit order. The catalog is rebuilt in memory from the log and is never authoritative.

The only admitted schema in this slice has `schema`, `ssid`, and `dns` fields. A new idempotency key for existing content creates a new signed operation referencing the same blob; it does not create another object. The same key never creates another commit.

## Build and verify

Rust 1.90 or later; dependencies and their checksums are pinned in `Cargo.lock`.

```powershell
cargo test --locked
cargo clippy --all-targets --features fault-fixtures --locked -- -D warnings
cargo run --release --locked --features fault-fixtures --bin crash-gate -- 10000 crash-report.json 8
```

The harness uses eight independent test threads, each spawning real child processes against its own temporary backing. Distinct writes accumulate in groups of 32. It randomly selects one of five write phases and forcibly terminates the child; some terminations race the durable log commit, while others interrupt an open uncommitted transaction. Every recovery checks the acknowledged baseline, an independently expected commit count, durable operations, retry identity, object count and verified handle reads for every earlier object in the group. Synthetic fixture credentials are available only with the explicit `fault-fixtures` feature or in unit tests.

The small `crash-smoke.json` is the preliminary 100-kill run with fresh storage per trial, not the 10,000-kill gate. The final `crash-report.json`, when present, records the completed grouped-write gate. A failing run returns an error and does not write a success report. CI runs 200 kills as a regression smoke check; that is not a substitute for the full recorded gate.

An initial full run failed with `Reached Datalog execution limits`: Biscuit's default wall-clock budget is 1 ms and could expire under concurrent test load. This was an authorization denial, not an observed lost commit. The explicit bounded limits above replace that implicit default; unit tests still verify denial of expired, attenuated, forged, oversized and excessive-fact tokens. Existing-storage unlock also validates through a read transaction rather than committing an unchanged format record on every restart.

## Invariants and boundaries

I1 lives in canonicalization and read verification; I2 in typed validation and Biscuit authorization; I3 in the absence of any planner or LLM from byte operations; I4 in MAC'd exact handles; I5 in immutable objects and durable append-only operations; I6 in the explicitly versioned extractor epoch.

This is an internal kernel tested directly, not a deployed LFS bus service. `open` currently produces an internal zeroizing buffer. It does not claim zero-copy IPC. M2 must expose typed envelopes and route that buffer through real iceoryx2; Wasmtime hosting also remains pending. There is no external transport or compatibility shortcut.

The replay catalog is not LanceDB hybrid recall. M0's semantic experiment has not yet been ported into this kernel; the epoch deliberately says `no-embedding`. Unknown epochs fail closed rather than being silently reinterpreted. Time-range queries, semantic recall, schema migration, garbage collection of orphan blobs, production key provisioning, and the restore timer remain pending. No live network configuration is changed here; the restore timer belongs to the future apply operation.

Crash testing covers process termination, not hardware power loss. A signed chain detects modified entries and broken links; it cannot detect rollback to a complete earlier snapshot without an external trusted head anchor. The PoC does not yet supply such an anchor. Rebuilding validates every object and scales linearly with the log; this is not a large-corpus performance claim. Content OIDs and storage sizes remain observable at rest.

## Dependency/API references

- [redb 4.3.0 durability semantics](https://docs.rs/redb/4.3.0/redb/enum.Durability.html)
- [FastCDC v2020](https://docs.rs/fastcdc/5.0.0/fastcdc/v2020/index.html)
- [Biscuit 6.0.0 authorization and attenuation](https://docs.rs/biscuit-auth/6.0.0/biscuit_auth/)
- [XChaCha20-Poly1305 0.10.1](https://docs.rs/chacha20poly1305/0.10.1/chacha20poly1305/)

Stable crypto APIs are pinned deliberately rather than adopting a new major version during the durability spike. Biscuit 6.0.0 requires its `datalog-macro` feature in this build because disabling it exposed unresolved upstream imports. The crate follows the repository's GPL-3.0 license; upstream dependencies retain their own licenses.
