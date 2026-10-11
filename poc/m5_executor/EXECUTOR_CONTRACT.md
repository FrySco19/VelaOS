# Executor contract (EXEC-v2.1)

Status: **EXEC-v2.1 = what the M5 Executor guest implements today** (wire frame v2, `VBRG`/`VEX2`, unchanged).
v2.1 (2026-10-12) = v2.0 + R1 closed: Wasmi built with `portable-dispatch`, `MAX_FUEL` survival
asserted on host and in QEMU (PCB requests 9-10). Wire bytes, status codes and limits are identical to v2.0.
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
| Executor heap / native stack | 2 MiB / 64 KiB | Microkit `executor.system`; Wasmi dispatch is constant-stack (`portable-dispatch`, R1) | domain fault (no status) |
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
- host-tested: `contract::e8_fuel_is_deterministic_budget_not_time`, `fixtures::bounded_by_limits_not_by_time`,
  `r1_max_fuel::*` (MAX_FUEL -> status 6 on a bounded native stack, R1).
- QEMU-verified 2026-10-12: `LOOP_MODULE` at fuel 64 (request 6) and at `MAX_FUEL` (request 9), both status 6.

**Risk R1 - CLOSED 2026-10-12 (branch `grok/r1-wasm-fuel`).**
*Cause.* The guest enabled `wasmi` 2.0.0 with `default-features = false` and features
`stable, validate, prefer-btree-collections`, i.e. **neither** `auto-dispatch` **nor**
`portable-dispatch`. In that configuration Wasmi always selects its tail-call dispatch backend
(`src/engine/executor/handler/dispatch/mod.rs`), which is constant-native-stack only if LLVM performs
sibling-call optimisation; nothing in the build guarantees that (opt 0/1, debug assertions, a
toolchain or codegen change can break it). Reproduced on the host before the fix: a `MAX_FUEL`
`LOOP_MODULE` in the dev profile aborts with `STATUS_STACK_OVERFLOW` (also on a 1 MiB thread),
whereas the 64-fuel loop fits. A native overflow in the guest is a domain fault: no reply, status
never reaches the PCB.

*Mitigation chosen: `portable-dispatch`* (guest `app/Cargo.toml` **and** host mirror). It forces
Wasmi's loop-based dispatch backend, which executes every instruction from one native frame
regardless of opt-level or target, so fuel exhaustion always surfaces as `TrapCode::OutOfFuel`
-> FUEL_EXHAUSTED (6). Preferred over stack tuning (would only move the threshold: tail-call growth
is proportional to executed instructions, ~10^7 at `MAX_FUEL`) and over `auto-dispatch` (still
picks tail calls whenever the build is optimising, i.e. still depends on LLVM). Cost: slower
interpretation; not measured on target (fuel is not latency, E8). No wire, status, limit or
Cargo.lock change (features are not recorded in the lockfile).

*Evidence.*
- host: `executor-host/tests/r1_max_fuel.rs` (7 tests) runs a signed frame end-to-end at
  `MAX_FUEL` on a fixed-size thread - 1 MiB in the dev profile (opt 0, debug assertions; the
  unoptimised admission path alone needs > 256 KiB), 64 KiB = Executor `stack_size` under
  `--release` - for `bridge::LOOP_MODULE` and hostile fixtures `max_fuel_loop`,
  `max_fuel_call_loop`, `max_fuel_recursion_churn` (status 6, value 0, then request 10 still
  returns 7), plus `deep_recursion` at `MAX_FUEL` (status 5) and a manifest guard
  `r1_portable_dispatch_is_pinned_in_guest_and_host`. Negative control: with `portable-dispatch`
  removed, the three MAX_FUEL loop cases abort with `STATUS_STACK_OVERFLOW` in the dev profile.
  The old `[profile.dev.package.wasmi]` release override was removed: the tests now hold in the
  strictest profile.
- QEMU: PCB requests 9 (`LOOP_MODULE` at `MAX_FUEL` -> `status=6 value=0 attempts=4`, marker
  `MAX_FUEL stack-safe status=6`) and 10 (return-7 after it -> `status=0 value=7 attempts=5`);
  `verify-bridge` PASS, log [`docs/reports/m5-verify-bridge-2026-10-12-r1-max-fuel.log`](../../docs/reports/m5-verify-bridge-2026-10-12-r1-max-fuel.log). QEMU inside the Docker image: 6.2.0
  (Ubuntu 22.04 `qemu-system-arm`), `-cpu cortex-a53`, TCG.

*Residual.* (a) Proven for the shipped Wasm feature set (no SIMD/tail-call proposal in use by
fixtures); Wasm-level recursion stays bounded by `set_max_recursion_depth(64)` and stack height
16 Ki (heap-allocated, not native). (b) The 64 KiB native-stack proof on the host is x86_64
Windows; on AArch64 it is the QEMU run (one `MAX_FUEL` loop), not a stack-usage measurement.
(c) Interpretation time at `MAX_FUEL` on real hardware is unmeasured; the Executor has no
deadline (E8). (d) Host functions/imports are not allowed (E4), so no host-call re-entrancy path
exists to overflow; adding imports would need a new review.

### E9 — Size bounds fail closed
`signed_frame` refuses payloads > 3 980 B (PCB side); a descriptor whose payload runs past the page
is MALFORMED before signature verification (no consumption, no instantiation). A 3 980-byte module
(fixture `max_payload.wasm`) executes; a valid 3 981-byte module (`oversized.wasm`) cannot be admitted.
- host-tested: `contract::e9_payload_size_bound`, `fixtures::oversized_module_fails_closed`.

## Gap to envelope v0 (EXEC-v3 target)
| C1/C4/C5 field or rule | EXEC-v2.1 today |
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
- `cargo test --locked --manifest-path poc/m5_executor/executor-host/Cargo.toml` — contract 13, fixtures 5, guest_parity 2, r1_max_fuel 7 (also passes with `--release`).
- `cargo test --locked --manifest-path poc/m5_executor/keygen/Cargo.toml` — 2.
- `cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml` — 12.
- QEMU: `verify-bridge` re-run 2026-10-12 on this branch, PASS, 14 markers, log `docs/reports/m5-verify-bridge-2026-10-12-r1-max-fuel.log`;
  earlier: `PHASE_2_3_REPORT.md`, `phase-2-3-boot.log`.
- Fixtures: `poc/m5_executor/fixtures/` (`.wat` sources + committed `.wasm`; drift-checked).
