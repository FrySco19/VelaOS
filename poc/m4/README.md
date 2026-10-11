# M4 — persistent property domain and rollback (qualification open)

The current `provider.rs` stores real encrypted M1 objects and a signed durable
logical head in an isolated property domain. `supervisor.rs` adds a persistent
recovery ledger, exact-object rollback authority and a 120-second watchdog. It
does not configure a physical network interface or claim cross-database/device
atomicity. Recovery authority is independent of the original user token expiry.

`session.rs` now connects authenticated SAVE, capability-filtered card enumeration,
bounded untrusted rankings, explicit exact-OID selection, CONFIRM and KEEP.
Private single-use tickets remain outside model output. Eleven Rust tests and Clippy
pass locally, including session SAVE through KEEP and database reopening. This
default test supplies a ranking fixture. The additional `live_m0_session_to_durable_keep`
test passes with the real E5/LanceDB Vector + BM25 adapter, then exact selection,
CONFIRM, KEEP and durable reopen. It is enabled explicitly after pinned asset
bootstrap in CI. Selection and confirmation are trusted test events, not a real UI.
Protected production UI ingress, full injection matrix, process-kill qualification
and global memory measurements remain open. The seL4 fixed demo uses volatile
storage and does not replace the native durable provider.

The sections below describe the original coordinator fixture and its gate scope;
the provider/supervisor items have since been implemented at the native API level.

The Rust coordinator persists an Ed25519-signed, hash-chained redb operation log before applying a change. Preparation binds the caller, exact target OID and previous snapshot. An unexported random confirmation ticket is delivered only to the trusted client boundary. The planner cannot forge a confirmation field in its restricted output.

Confirmation rechecks capability, ticket, deadline and the previous active state's content hash. The legacy coordinator's provider contract requires atomic compare-and-apply; the new persistent provider separately stages and verifies its logical head without claiming device atomicity. The resulting state is checked against the exact target OID. Keeping the change requires a fresh write capability, the same caller/ticket and a live timer. Ticket replay is rejected.

Unkept changes roll back after timeout. On journal reopening, an unfinished apply rolls back rather than extending an uncertain clock. A conflicting third-party state becomes `RecoveryRequired`; it is not overwritten automatically. Reading target and previous object bytes uses the typed iceoryx2 adapter in M2; no LLM sees or handles those bytes.

## Verified scope

Tests use real encrypted M1 storage, signed durable operation records, current Biscuit checks and real local iceoryx2 loans, with an in-memory NetConfig provider. They cover wrong caller, false ticket, repeated confirmation, changes between preparation and apply, keep without write permission, expired keep timer, restart rollback and finalized restart persistence.

The malformed-output test covers distinct attempts to inject authority, caller, commands, steps, handles, risk and duplicate fields, plus a valid Restore proposal lacking write authority. It is **not** the specification's 100-case end-to-end prompt-injection gate. Timer tests advance the host test clock; they are not a production supervisor qualification. Recovery also rejects a tampered signed journal or a different signing key.

## Remaining before M4 closure

1. A persistent protected LFS service and production control/bulk ingress, rather than local native library adapters.
2. A real network-device provider and production authenticated ingress. The native isolated persistent provider, client boundary and automatic supervisor are implemented.
3. Production provisioning and process-kill qualification of the implemented dedicated exact-object recovery authority.
4. Expose the now-tested native SAVE → semantic RECALL → explicit selection → CONFIRM → APPLY → KEEP path through a protected production human interface and persistent Bus services.
5. A frozen 100-case injection matrix exercised against the actual ingress, capability broker, Wasm tasks and side-effect provider, plus crash/failure recovery tests.
6. Integrated idle RAM below 1.5 GB measured on the VelaOS runtime, not inferred from one worker on the Windows development host.

M5 is the seL4/QEMU port described by the specification. It starts after these M4 requirements pass; this coordinator alone does not establish a complete operating system or a bootable image.
