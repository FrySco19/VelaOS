# M4 — durable restore coordinator (gate open)

The Rust coordinator persists an Ed25519-signed, hash-chained redb operation log before applying a change. Preparation binds the caller, exact target OID and previous snapshot. An unexported random confirmation ticket is delivered only to the trusted client boundary. The planner cannot forge a confirmation field in its restricted output.

Confirmation rechecks capability, ticket, deadline and the previous active state's content hash. The provider must implement atomic compare-and-apply. The resulting state is checked against the exact target OID. Keeping the change requires a fresh write capability, the same caller/ticket and a live timer. Ticket replay is rejected.

Unkept changes roll back after timeout. On journal reopening, an unfinished apply rolls back rather than extending an uncertain clock. A conflicting third-party state becomes `RecoveryRequired`; it is not overwritten automatically. Reading target and previous object bytes uses the typed iceoryx2 adapter in M2; no LLM sees or handles those bytes.

## Verified scope

Tests use real encrypted M1 storage, signed durable operation records, current Biscuit checks and real local iceoryx2 loans, with an in-memory NetConfig provider. They cover wrong caller, false ticket, repeated confirmation, changes between preparation and apply, keep without write permission, expired keep timer, restart rollback and finalized restart persistence.

The malformed-output test covers distinct attempts to inject authority, caller, commands, steps, handles, risk and duplicate fields, plus a valid Restore proposal lacking write authority. It is **not** the specification's 100-case end-to-end prompt-injection gate. Timer tests advance the host test clock; they are not a production supervisor qualification. Recovery also rejects a tampered signed journal or a different signing key.

## Remaining before M4 closure

1. A persistent protected LFS service and production control/bulk ingress, rather than local native library adapters.
2. A real network provider, a trusted authenticated confirmation client and an automatic persistent rollback supervisor.
3. A dedicated, exact-object rollback authority that remains usable for recovery without preserving unrestricted or expired user authority.
4. The complete SAVE → semantic RECALL → explicit exact selection → prepare → human CONFIRM → verified APPLY → KEEP/rollback path.
5. A frozen 100-case injection matrix exercised against the actual ingress, capability broker, Wasm tasks and side-effect provider, plus crash/failure recovery tests.
6. Integrated idle RAM below 1.5 GB measured on the VelaOS runtime, not inferred from one worker on the Windows development host.

M5 is the seL4/QEMU port described by the specification. It starts after these M4 requirements pass; this coordinator alone does not establish a complete operating system or a bootable image.
