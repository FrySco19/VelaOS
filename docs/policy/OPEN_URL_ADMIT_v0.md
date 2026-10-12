# OPEN_URL_ADMIT_v0

Status: **draft v0** on branch `policy/open-url-admit-v0`.
Extends: M-G1b Candidate ≠ Authority. Complements `ASSISTANT_UX_LEVELS_v0` (UX levels are never grants).
Does not change: `GrantIssuer` order, `DenyCode` enum, Executor wire v2, CONTRACTS_v0 C5 numerics.

Normative: **MAY** / **MUST** / **MUST NOT**.

## 1. Path B fixture (sole admit target)

Observed Proposal row (Geolier WINNER lyric video, Chrome History):

| Field | Value |
| --- | --- |
| `candidate_index` | `0` |
| `handle` | `obs-yt-WIX4bc9MoiY` |
| `expected_version` | `history-observed` |
| `action` | `open-url` |
| `url` | `https://www.youtube.com/watch?v=WIX4bc9MoiY` |
| UX level required for Seleziona surface | `0` (`full`) |

Intent **MAY** only *propose* this row (`authority: false`). Intent **MUST NOT** execute, grant, sign, or admit. No `DEMO_ADMIT`.

## 2. Admit rule (tight)

Policy **MUST** admit operation `open-url` for **exactly** the URL above **only when all** hold:

1. Shell UX level is `0` (`full`).
2. Human Seleziona on that shown row emits `UserSelection` with `handle=obs-yt-WIX4bc9MoiY` and `expected_version=history-observed` (Shell `TrustedSelectionChannel`; not a UX-level gesture; Intent does not mint it).
3. Requested action is `open-url` and URL is byte-identical to `https://www.youtube.com/watch?v=WIX4bc9MoiY`.

Then Intent **MAY** execute open on that URL only.

## 3. Deny

| Condition | Code | Reason |
| --- | --- | --- |
| No `UserSelection` | `POLICY_DENIED` = 10 | (M-G1b; no UX tag) |
| Wrong handle / version / URL / action | `POLICY_DENIED` = 10 or `SCOPE` = 3 | mismatch |
| Level `reduced` (1) and full Seleziona surface attempted | `POLICY_DENIED` = 10 | `UX_LEVEL_REDUCED` |
| Level `hidden` (2) and action/admit surface attempted | `POLICY_DENIED` = 10 | `UX_LEVEL_HIDDEN` |

UX levels **MUST NOT** grant `open-url`. Level `0` alone **MUST NOT** admit.

## 4. Non-goals

No new C5 numeric code. No Rust `DenyCode` change on this branch. No Glass/Shell/Intent implementation. No `main`. No `vbox-iso` / `os/x86_64`.

## 5. Compatibility

Extends M-G1b. `open-url` is an additional selection-gated effect class, not a replacement for `LFS_READ`. After this admit, Intent may open; until then Proposal display only.
