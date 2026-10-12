# SIGNALS_CONTRACT_v0

Status: **draft v0** on branch `policy/signals-contract-v0`.
Scope: PCB / policy evaluation inputs only.
Extends: M-G1b Candidate ≠ Authority (`poc/m5_executor/policy/src/grant.rs`, `src/pipeline.rs`, `README.md`).
Does not change: `DenyCode`, `GrantIssuer` decision order, Executor wire v2.

Normative words: **MAY** is permitted, **MUST** is required, **MUST NOT** is forbidden.
"Context" in this contract is the observational producer of signals (recall set, trusted clock, native services). It is not an authority domain. This v0 adds no Context crate and no new type.

## 1. Purpose

Context **MAY** emit observational signals for ranking, UX, audit, and for *narrowing evaluation* (deny, or constrain an already-requested scope). A signal is a typed observation. It is never a grant.

Signals **MUST NOT** grant, sign, widen scope, mint a token, or confirm on behalf of the user. The model interprets; the user selects; policy decides. Missing or ambiguous trusted input fails closed.

## 2. Allowed evaluation inputs

The kinds below **MAY** be consumed by Intent and Glass for display, and by the PDP only as facts that can cause **DENY** or constrain scope. None of them admits by itself. Each kind is an **evaluation input** and is **never authority**.

| Kind | Existing name | Evaluation input | Never authority |
| --- | --- | --- | --- |
| Shown candidate | `Candidate { handle, version, score }` already in the recall set (`GrantRequest.candidates`, `IntentProposal.candidates`) | Presence of a handle+version that was shown. Absence or a different handle → deny (`SCOPE`). Version mismatch → `VERSION_CONFLICT`. | Does not admit. `score` (RRF or otherwise) is display/ranking only and is never consulted as a right. |
| Clock / TTL | `TrustedTime`, `Clock`, `GrantDeadline`, `not_after`, `MAX_READ_GRANT_TTL` (`1..=120` s) | Trusted wall time mints and verifies `not_after`. Monotonic time decides whether a grant is still alive. `ClockError::Regressed` or `ClockError::Unavailable` → `EXPIRED`. | A model-asserted clock, a Context-asserted `not_after`, or a wall jump that would extend a grant is not time. |
| Resource bounds | `AdmitRequest.fuel`, `bridge::MAX_FUEL`, payload size, `request_id` | Facts the PDP already enforces. Fuel outside bounds → `SCOPE`. Bad shape → `MALFORMED`. Stale id → `REPLAY`. | Cannot raise fuel, payload, or request budget. Attenuation narrows; it never grants rights. |
| Selection-channel availability | presence or absence of `UserSelection` | Whether `TrustedSelectionChannel` has produced a selection for this request. Absence → `POLICY_DENIED`. | The selection itself comes from the shell gesture on `TrustedSelectionChannel`, not from Context. Availability is not the gesture. |
| Store poison | `FileLedger` / `LedgerStore::is_poisoned` (native durable LFS) | Fail-closed store state reported by the writer. Poisoned → later commits refused (`LfsStatus::QuotaExceeded`, C5 11) until reopen and recover. | Cannot clear poison, invent a receipt, or treat an unsynced write as committed. |

Rules for every allowed kind:

1. A signal **MAY** only name a fact already held by a native service, the trusted clock, the shell channel, or the recall set that was shown.
2. A signal **MUST NOT** be parsed into Datalog as `right(...)` or as an authority-block fact. Facts added in attenuation blocks never grant rights (M-G1a).
3. Showing a signal in Glass does not admit.

## 3. Always out

The following are **never authority** and **never admit**. The PDP **MUST NOT** parse them into Datalog, **MUST NOT** treat them as `UserSelection`, and **MUST NOT** mint a Biscuit or an Ed25519 admission signature from them.

- Model / LLM / planner prose: `planner_note`, Intent `rationale`, "user already approved", `SYSTEM` injections, and any other free text.
- `Candidate.score`, RRF score, lane `Source`, or any confidence used as a grant.
- Any token, `right(...)`, Biscuit, Ed25519 signature, capability, or privilege invented by a model or by Context.
- Forged or unshown handles or versions (not in the shown recall set / `candidates`).
- Glass HTML/JS claims, including a body that names a handle the glass is not showing.
- An Intent execute path, or a shell click that would reach `SelectionGatedPdp`, with no `UserSelection`. `IntentProposal::auto_resolve` stays false.
- Any signal that asserts consent, approval, confirmation, or authority.

`planner_note` and `rationale` stay audit-only, as in M-G1b. They are never parsed into Datalog.

## 4. Admit rule

Admit still requires the M-G1b path: `UserSelection` (from `TrustedSelectionChannel`, a shell gesture) → `GrantIssuer` → attenuated Biscuit → `SelectionGatedPdp`; Context signals cannot replace any step. Fail closed: missing or ambiguous trusted input yields `POLICY_DENIED` (C5 = 10) or the appropriate stable C5 code already used by that step, and never a silent admit.

Pointer: `poc/m5_executor/policy/README.md` (M-G1b), `src/grant.rs`, `src/pipeline.rs`.

## 5. Deny codes

Use `docs/CONTRACTS_v0.md` C5 only. This v0 adds no code and does not renumber.

| Observation | Code | Name |
| --- | --- | --- |
| Bad shape | 1 | `MALFORMED` |
| Token rejected under the pinned Biscuit root | 2 | `AUTH` |
| Handle not among shown candidates; fuel / scope | 3 | `SCOPE` |
| Selection id already consumed; stale `request_id` | 4 | `REPLAY` |
| TTL outside `1..=120` s; `time > not_after`; untrusted clock | 7 | `EXPIRED` |
| No `UserSelection`; model or Context text; signal used as authority | 10 | `POLICY_DENIED` |
| Poisoned durable store (existing LFS writer) | 11 | `QUOTA_EXCEEDED` |
| Shown version ≠ `expected_version` | 12 | `VERSION_CONFLICT` |

`DenyCode::contract_code()` already maps 1, 2, 3, 4, 7, 10, and 12. Code 11 is the existing `LfsStatus::QuotaExceeded` on the durable writer. This contract adds no `DenyCode` variant.

## 6. Non-goals

- No Glass or shell UI.
- No Intent candidate production and no change to Intent authority types.
- No ISO, VirtualBox, `vela/vbox-iso`, or `poc/m5_sel4` change.
- No merge to `main` from this branch.
- No rewrite of `grant.rs`, `pipeline.rs`, or the M-G1b decision table.

## 7. Compatibility

Extends M-G1b; does not change `DenyCode`, `GrantIssuer` decision order, or Executor wire v2.
