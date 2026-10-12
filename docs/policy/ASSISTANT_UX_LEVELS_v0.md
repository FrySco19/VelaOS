# ASSISTANT_UX_LEVELS_v0

Status: **draft v0** on branch `policy/assistant-ux-levels-v0`.
Scope: OS assistant side-panel visibility (Francesco via Pulse / Fairha / Shell).
Extends: M-G1b Candidate ≠ Authority. Zero-App is unchanged.
Does not change: `GrantIssuer` order, the `DenyCode` enum, Executor wire v2, or CONTRACTS_v0 C5 numeric codes.

Normative words: **MAY** is permitted, **MUST** is required, **MUST NOT** is forbidden.

## 1. Nature

The level is a UX and visibility constraint only. It reduces **which Context and Intent signals the panel may display**.

It is **never** a grant, a signature, a Biscuit attenuation, or executive authority. Changing the level does not concede permission, does not sign anything, and does not widen scope. Entering `full` grants nothing.

Admit and deny for real effects stay on the M-G1b path, and that path does not depend on the UX level:

`UserSelection` (a human gesture on a shown object) → `TrustedSelectionChannel` → `SelectionGatedPdp`.

A UX-level preference is not an LFS `UserSelection`. It **MUST NOT** replace one, and it **MUST NOT** be treated as consent.

## 2. Three levels

Stable ids are `0` `full`, `1` `reduced`, and `2` `hidden`. The Shell toolbar cycles `0 → 1 → 2`. It **MAY** show `0` / `1` / `2` or `full` / `reduced` / `hidden`. The control **MUST** target only these three values.

### 0 `full`

Assistant and context panel, visible at **320px**.

The panel **MAY** display:

- context observations;
- Intent proposals, labeled **Proposal**;
- Policy decision outcomes (`docs/CONTRACTS_v0.md` C5).

Execute, write, and any other effect still require a human `UserSelection`, then the PDP. The level does not own Shell controls. Showing the full surface, a proposal, or a decision outcome does not admit.

### 1 `reduced`

Icon rail at **72px**. Title and glyph only.

The rail **MUST NOT** contain an assistant surface, a context surface, or an Intent proposal surface. No full assistant, context, or proposal surface is shown beyond that rail.

### 2 `hidden`

Panel chrome is absent (width **0**). The content area is full width.

The panel is non-interactive. There is no proposal surface and no assistant or context surface.

At `reduced` and at `hidden`, effect gating is that visibility rule: no full assistant, context, or proposal surface. Policy does not draw the chrome and does not own Shell buttons.

## 3. Deny behavior

This v0 adds no C5 numeric code and no parallel admit code. Reuse `POLICY_DENIED` (C5 = 10). The reason tag is mandatory whenever UI copy explains the deny. A future CONTRACTS_v0.x additive numeric code is out of scope.

| While | Attempt | Code | Reason tag |
| --- | --- | --- | --- |
| `hidden` (2) | A service presents an action or admit surface | `POLICY_DENIED` = 10 | `UX_LEVEL_HIDDEN` |
| `reduced` (1) | A service presents a full assistant and context surface beyond the icon rail | `POLICY_DENIED` = 10 | `UX_LEVEL_REDUCED` |

`full` does not deny for visibility, and it does not admit. A missing `UserSelection` remains `POLICY_DENIED` (C5 = 10) under M-G1b, with no UX-level reason tag.

Admit of an LFS object remains `UserSelection` → `SelectionGatedPdp` at every level.

## 4. Who sets the level

Context **MAY** emit an observational signal that proposes a level. The user confirms with a Shell toolbar gesture. That gesture cycles `0 → 1 → 2`.

Policy **MUST NOT** invent a level and **MUST NOT** auto-apply one. The toolbar gesture sets chrome only. It is not an LFS `UserSelection` and **MUST NOT** be supplied to `SelectionGatedPdp` in place of one.

## 5. Ownership

Shell owns panel chrome and the toolbar control. Chrome is always present in the product model and is hideable: at `hidden` it occupies no width. Policy owns the meaning of the three levels and the deny reason tags in §3. Policy does not draw UI.

## 6. Non-goals

- No Glass UI and no Shell chrome implementation.
- No Intent candidate changes.
- No merge to `main`.
- No `vela/vbox-iso`, no `os/x86_64/`, and no ISO.
- No new `DenyCode` variant in Rust on this branch.

## 7. Compatibility

This model extends M-G1b (Candidate ≠ Authority). It does not change `GrantIssuer` decision order, the `DenyCode` enum, or Executor wire v2.

Pointers on this branch: `poc/m5_executor/policy/README.md` (M-G1b), `docs/M_G1B.md`, `docs/CONTRACTS_v0.md` (C5 = 10 `POLICY_DENIED`).
