# Floating Glass shell — presentation-only redesign

Branch: `vela/glass-redesign`. Complete offline HTML, CSS and Vanilla JS live in
`poc/product/glass/assets/studio.html`. No frontend dependency or transpilation
step is added. The existing Rust host embeds this file and is rebuilt to serve
the new asset. This remains a host renderer, not a native framebuffer port.

## Phase 0 audit: existing contract

The original file was read in full before editing; CSS and JS were inline.
No separate linked assets existed. The existing functions were `propose`,
`reject`, `authorize`, `updateDigest`, `renderTimeline` and `lock`, connected to
form submission, draft edits, reject/approve clicks, filters and keyboard input.

The WebSocket URL remains exactly `ws://127.0.0.1:7990/`. It is opened only on
authorization and closed after a result. There was no automatic reconnection,
heartbeat or idle connection. No automatic resend is introduced: a timeout may
have an ambiguous durable outcome. The dot reports the actual socket state;
it cannot represent broker health while the one-shot socket is closed.

Outbound JSON is unchanged, with exactly these fields:

```json
{"type":"Proposal","operation":"write-note","intent":"…","note":"…","note_sha256":"…"}
```

Accepted results remain:

- `Receipt`: `type`, `request` (decimal string), `note_sha256`, `receipt_hex`
  (340 lowercase hex characters), `verified` and `consent`.
- `Denied`: `type` and `reason`.
- `Ack` / `Progress`: transport progress only; never Completed.

The current broker actually returns Receipt or Denied. No inbound Proposal
exists, so the prior local proposal preparation is preserved rather than
inventing an API or pretending the backend generated it. Backend generation
would require a separate protocol change, outside this presentation task.

SHA-256 uses the same exact UTF-8 snapshot, 1024-byte limit and stale-digest guard.
Receipt checks retain the same type/verified/hash/request/length/consent checks,
version/request/generation/hash decoding and duplicate-request rejection.
Neither Rust cryptographic code nor the payload schema was changed.

## Presentation and data assumptions

- One 640px floating column on a full-height canvas; no sidebar, header, footer,
  send button, debug paragraph or fabricated initial history.
- Command placeholder is `Qual è il tuo intento?`; Enter submits, composition
  Enter is ignored, Escape clears. Preparation is an unobtrusive indicator.
- Rounded, muted system surfaces; no colored gradients or neon. Blur is confined
  to the modal backdrop. Animation changes only transform and opacity.
- The tertiary text token is slightly lighter than the supplied example to keep
  functional small text readable. The hash reduces to 12px on narrow screens.
- Tab filters support arrows/Home/End and roving focus. The current operation is
  always write-note, so its deterministic category is Lavoro; Sistema currently
  has no records. Categories are client metadata and are never sent as authority.
- The timeline remains session-only completed receipts. The old detail affordance
  is preserved with readable outcome/consent/hash/signature, without raw JSON,
  technical request IDs or raw timestamps in the visible UI.
- The modal initially focuses RIFIUTA, is inert to backdrop clicks, traps Tab,
  displays all 64 hash characters in eight groups, and locks the payload after
  authorization. The native dialog makes the underlying page non-interactive.
- Escape/reject before sending emits nothing. After sending it can dismiss the
  surface but cannot revoke an already authorized operation: no cancellation
  protocol is available. The pending receipt continues to be processed.
- Missing/invalid receipts never create a completed entry. Errors are short
  Italian toasts. Development consent remains identified in receipt details and
  in a brief save toast; it is not represented as biometric consent.

## Executed verification

```text
cargo build --manifest-path poc/product/glass/Cargo.toml
node poc/host-broker/tests/e2e-ui.cjs .local/v1
node poc/host-broker/tests/redesign-ui.cjs .local/v1
```

Playwright/Edge used the real broker and QEMU. Request `1791902412728` completed
with a 170-byte signed receipt, `MockUnavailableDevelopment`, unchanged outbound
field set, dismissed modal, timeline insertion and working context filters.

Separate UI checks passed: zero frames on refusal; UTF-8 overflow blocks signing;
exact independently computed SHA-256; initial reject focus; forward/reverse focus
trap; Escape and restored command focus; arrow-key filters; no horizontal overflow
or truncated hash at 360px. Empty Enter and a composed IME key event were ignored;
Enter on the default reject focus refused without transmission; a backdrop click
did not dismiss the dialog. Base text contrast checks were 4.70:1 for placeholder,
5.59:1 metadata, 5.24:1 reject label and 14.45:1 primary text. An isolated injected
ACK left the UI waiting and the
payload locked, without a completed timeline entry; that is not the live E2E.

Evidence: `.local/v1/ui-result.json`, `redesign-ui-result.json`,
`redesign-idle.png`, `redesign-security.png`, `redesign-proposal.png`, `redesign-mobile.png`.
The broker, keys, native domains, storage and OS delivery gates are unchanged.
