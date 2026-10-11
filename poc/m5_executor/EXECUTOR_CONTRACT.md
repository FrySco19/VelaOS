# Executor contract (EXEC-v2.0)

Status: **EXEC-v2.0 = what the M5 Executor guest implements today** (wire frame v2, `VBRG`/`VEX2`).
Base: `1281f5a` (`grok/qa-policy-ci`). Scope: `poc/m5_executor/app/src/main.rs` (Executor domain) and
`app/src/bridge.rs` (wire + admission verification). Related: [BRIDGE_CONTRACT.md](BRIDGE_CONTRACT.md)
(ownership/ack, remaining gates), `docs/CONTRACTS_v0.md` (C1 envelope, C2 ownership, C3 authority
levels, C5 error codes).

Versioning: changes to any obligation below that alter wire bytes, status codes or limits require a
new EXEC version and re-running `verify-bridge` in QEMU. Additive host tests and documentation do not.
EXEC-v3 (target, not implemented) = envelope v0 (C1) + OWN-v3 (C2) + C4 receipt; see "Gap to v0".

Evidence levels used here: **host-tested** = asserted by `cargo test` in `poc/m5_executor/executor-host`
or `keygen` on this branch; **QEMU-documented** = asserted by guest code and recorded in
`PHASE_2_3_REPORT.md` / `phase-2-3-boot.log`, not re-run for this document.

The host tests compile the **unchanged** `app/src/bridge.rs`. The Wasmi path in `main.rs` cannot be
linked on the host (it depends on `sel4-microkit`), so `executor-host/src/lib.rs` mirrors it, and
`tests/guest_parity.rs` fails if the guest configuration or the PCB test sequence drifts from the mirror.

## Obligations

### E1 — Owned snapshot before parse/execute
The Executor copies the whole 4 KiB shared page into a private buffer (`bridge::snapshot`, volatile
reads after an Acquire fence) **before** parsing the descriptor or verifying anything. Verification
and execution both use that snapshot; no borrowed slice of shared memory escapes. Later producer
writes cannot change authenticated executable bytes. The copy is bounded (4 KiB), not zero-copy.
- host-tested: `contract::e1_owned_snapshot_is_what_executes`, keygen `signed_snapshot_rejects_tampering_scope_replay_and_bounds`.

### E2 — Admission = Ed25519 verify only (no Biscuit in the guest)
The Executor verifies one Ed25519 signature with the **pinned PCB public key**
(`trusted_key.rs`) over `"VelaOS.Executor.SignedFrame.v2\0" ‖ header[0..20] ‖ claims[32] ‖ payload`.
Claims bind magic `VEX2`, audience 2 (Executor), request ID (must equal header ID), operation 1
(execute only), fuel, memory quota, payload length. Any mismatch is AUTH (bad signature/bytes) or
SCOPE (signed but out of bounds). The guest does **not** parse or evaluate Biscuit tokens
(`biscuit-auth` 6.0.0 is `std`-only; C3): Biscuit is checked by the host PDP
(`poc/m5_executor/policy`), which signs a v2 frame only on ALLOW. The signature is evidence of PCB
admission, not a seL4 capability; no token or signature can add kernel capabilities (C3 L3).
- host-tested: `contract::e2_admission_rejections_happen_before_instantiation` (wrong key, payload,
  header ID, fuel/memory/audience claims, signature byte → AUTH, 0 instantiations, ID not consumed).

### E3 — Resource limits (enforced by the Executor itself, not trusted from the PCB)
| Limit | Value | Enforcement | On violation |
| --- | --- | --- | --- |
| Signed fuel | 1 ..= 10 000 000 | `verify_with_key` | SCOPE (3), ID not consumed |
| Signed linear memory | 1 ..= 65 536 B | `verify_with_key` + `StoreLimits::memory_size` | SCOPE (3) / instantiation fails → 5 / `memory.grow` returns -1 |
| Memories / instances | 1 / 1 | `StoreLimits` | WASM (5) |
| Tables / table elements | 0 / 0 | `StoreLimits` | WASM (5) |
| Interpreter stack height | 16 Ki | `Config::set_max_stack_height` | trap → WASM (5) |
| Recursion depth | 64 | `Config::set_max_recursion_depth` | trap → WASM (5) |
| Payload size | ≤ 3 980 B (`PAGE_SIZE − PAYLOAD_OFFSET`) | descriptor bounds | MALFORMED (1) |
| Executor heap / native stack | 2 MiB / 64 KiB | Microkit `executor.system` | domain fault (no status) |
Note: the PCB's signed memory quota is always `MAX_MEMORY` today (`signed_frame`), so a smaller
memory quota is verified by the Executor but not exercised end-to-end.
- host-tested: `contract::e3_*`, `fixtures::fixture_outcomes_fail_closed` (`memory_over_quota`,
  `grow_denied`, `table_declared`, `deep_recursion`), `fixtures::bounded_by_limits_not_by_time`.

### E4 — No imports, no WASI, no ambient storage
The linker is empty. Any module declaring **any** import (function, memory, WASI or not) is rejected
with WASM (5) **before instantiation** (`attempts` unchanged), after the ID is consumed. The Executor
has no filesystem, clock, random, network or LFS access and interprets no paths. Input/output data
can only arrive in the signed payload and leave as the u32 result (E5). Opaque LFS handles (C1/C4)
are not implemented.
- host-tested: `contract::e4_wasi_import_rejected_without_instantiation`, fixtures `import_env_func`, `import_memory`.

### E5 — Typed, bounded result (today: one u32)
Entry point: export `run : () -> i32`. Missing export, wrong signature or trapping start function →
WASM (5). Reply = label `0x702`, four message registers:
`regs[0]` status (C5 0..=6), `regs[1]` result `i32` as **u32 bits** (0 on error), `regs[2]` request
ID, `regs[3]` instantiation attempts in this boot.

**Limitation (UX gap):** the only result channel is 32 bits in an IPC register. There is no result
byte buffer, length, digest, type tag, `fuel_used`, provenance or receipt hint. A UX surface that
shows a rich result (text, table, file diff) plus provenance cannot be fed by the Executor today; it
needs the C1 v0 typed response block (`epoch, request_id, status, result_digest, result_len,
fuel_used, receipt_hint`) in a return page/region bounded by `quota.output_bytes`, then persisted
by the LFS writer with a C4 receipt. Until then UX must treat the Executor result as an opaque
integer and must not present it as persisted or as having provenance beyond "PCB-admitted frame N".
- host-tested: `contract::e5_result_is_single_u32_register`, fixtures `negative_result` (-1 → `0xFFFFFFFF`),
  `no_run_export`, `wrong_signature`, `start_trap`.

### E6 — Status codes (C5 mapping)
| Code | Executor `Status` | C5 name | ID consumed? | Wasmi instantiated? |
| --- | --- | --- | --- | --- |
| 0 | `Success` | OK | yes | yes |
| 1 | `Malformed` | MALFORMED | no | no |
| 2 | `Authentication` | AUTH | no | no |
| 3 | `Scope` | SCOPE | no | no |
| 4 | `Replay` | REPLAY | no (already) | no |
| 5 | `Wasm` | WASM_REJECT_OR_TRAP | yes | parse/import rejects: no; traps: yes |
| 6 | `Fuel` | FUEL_EXHAUSTED | yes | yes |

Codes 7..15 (EXPIRED, REVOKED, CANCELLED, **POLICY_DENIED = 10**, QUOTA_EXCEEDED, VERSION_CONFLICT,
IDEMPOTENCY_CONFLICT, AMBIGUOUS, UNAVAILABLE) are **never** produced by the Executor today.
POLICY_DENIED is a host-PDP outcome (`policy::DenyCode::PolicyDenied`); on DENY no frame is signed,
so the Executor never sees the request. (Note: `DenyCode` has no explicit numeric discriminants yet;
the mapping to 10 is by contract only.) Memory over quota at instantiation is reported as 5, not 11.

Correlation caveat: wrong channel/label/count → MALFORMED with `regs[2] = 0`. For AUTH/SCOPE/REPLAY/
MALFORMED-descriptor replies `regs[2]` echoes the **unauthenticated** request-ID bytes of the
snapshot; the PCB must correlate through single-flight (OWN-v2), not trust that value.
- host-tested: `contract::e6_*`.

### E7 — Replay and consumption
Request IDs are u32, must be > 0 and strictly greater than the last consumed ID (monotonic, gaps
allowed). An authenticated, in-scope ID is consumed **before** instantiation, so failed executions
(5, 6) cannot be retried with the same ID. AUTH/SCOPE/MALFORMED do not consume IDs.
**Known gap:** replay state is a boot-local `u32` in RAM. A reboot accepts a previously executed
frame; no epoch, no durable state (C2 OWN-v3, Gate 2).
- host-tested: `contract::e7_replay_rules_within_session`, `contract::e7_known_gap_replay_state_is_boot_local`
  (pins the gap so a durable-epoch implementation must consciously flip it).

### E8 — Fuel is not a wall-clock deadline
Fuel is a deterministic instruction budget (host: return-7 needs exactly 30 fuel in every run with
this Wasmi build; one less → status 6). It does not bound: module parsing/validation (bounded only
by the 3 980-byte payload cap), engine/store setup, the 4 KiB snapshot copy, signature verification
or scheduling. There is no trusted clock in the Executor, no `deadline_ms`, no `not_after`, no
cancellation. Fuel counts must never be reported as latency.
- host-tested: `contract::e8_fuel_is_deterministic_budget_not_time`, `fixtures::bounded_by_limits_not_by_time`.

**Risk R1 (found while writing these tests):** with the guest feature set (`stable`, no
`portable-dispatch`), Wasmi 2.0.0 uses a tail-call dispatch backend that is constant-stack only if
LLVM performs sibling-call optimisation. On the host (Windows x86_64) a `MAX_FUEL` infinite loop
**overflows the native stack** in a dev build (opt 0) and also with opt-level 3 when
debug-assertions are on; it returns FUEL_EXHAUSTED only with release-like settings. The guest is
built `cargo build --release` (opt 3, no debug-assertions), and QEMU only exercises the loop with
fuel 64, so: (a) the Executor **must** be built with release settings, and (b) a long-running task
at `MAX_FUEL` on AArch64 is **not verified** in QEMU. A native overflow would be a domain fault
(no reply, PCB must not assume ack — BRIDGE_CONTRACT), not status 6. Mitigation candidates
(require a guest change + QEMU re-run, not done here): enable Wasmi `portable-dispatch`, or add a
QEMU case `LOOP_MODULE` at `MAX_FUEL` and a build check that rejects non-release Executor ELFs.

### E9 — Size bounds fail closed
`signed_frame` refuses payloads > 3 980 B (PCB side); a descriptor whose payload runs past the page
is MALFORMED before signature verification (no consumption, no instantiation). A 3 980-byte module
(fixture `max_payload.wasm`) executes; a valid 3 981-byte module (`oversized.wasm`) cannot be admitted.
- host-tested: `contract::e9_payload_size_bound`, `fixtures::oversized_module_fails_closed`.

## Gap to envelope v0 (EXEC-v3 target)
| C1/C4/C5 field or rule | EXEC-v2.0 today |
| --- | --- |
| `epoch` u64, `request_id` u64 | absent / u32 boot-local |
| `payload_digest` BLAKE3 | signature directly over bytes (equivalent binding, no digest) |
| `authority_digest`, `revocation_gen`, `not_after` | absent (PDP checks expiry on host before signing; guest cannot) |
| `objects`, `result_handle`, LFS read/write ops | absent; ops = execute only |
| `quota.output_bytes`, `quota.deadline_ms` | absent; output is 32 bits; no clock |
| typed response block | absent; 4 registers only |
| durable consume-before-effects, receipt (C4) | absent; RAM only, result not persisted |
| codes 7..15 | not produced |

## Evidence
- `cargo test --locked --manifest-path poc/m5_executor/executor-host/Cargo.toml` — contract 13, fixtures 5, guest_parity 2.
- `cargo test --locked --manifest-path poc/m5_executor/keygen/Cargo.toml` — 2.
- `cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml` — 12.
- QEMU: `PHASE_2_3_REPORT.md`, `phase-2-3-boot.log` (not re-run for this document; guest unchanged).
- Fixtures: `poc/m5_executor/fixtures/` (`.wat` sources + committed `.wasm`; drift-checked).
