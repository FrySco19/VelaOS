# Glass Studio — Phase 3 scaffold

Open `/studio` on the existing loopback Glass server, for example
`http://127.0.0.1:7881/studio` after starting:

```text
cargo run --locked --manifest-path poc/product/glass/Cargo.toml -- --port 7881
```

The existing `/` host PDP demonstration is preserved separately. Studio never
calls `/submit` or `/click` and owns no policy delegation or system capability.
Its per-run HTTP token is a host transport nonce, not a Biscuit authority token.

## Components

- `assets/studio.html`: responsive frosted command bar and generic authorization
  card; note/media/network adapters; persistent minimal toolbar.
- `src/ui_state.rs`: renderer-independent Rust lifecycle using no allocations
  or authority types. Stale revisions, invalid transitions and execution
  shortcuts are refused. Browser events affect preview state only.
- `/studio/event`: strict bounded event/revision endpoint, using the existing
  loopback/Host/Origin/body-token checks. This is not trusted physical input.
- `/studio/state`: lifecycle inspection. One host Studio session per server;
  tabs are not independent protected sessions. A reload clears that scaffold
  session; multi-user/native session binding remains future work.

## Behavior

Use the summon button, Alt+Space or Ctrl+K within the focused tab. Windows can
reserve Alt+Space for its own menu; this is not an installed global OS shortcut.
Enter classifies finite example keywords locally; no model or BM25 is called by
Studio. Unknown intents enter Clarifying. Escape cancels. Edited note drafts
require explicit discard before closing. Review is informational only.

Approval goes through Authorizing and then Denied with an explicit unconnected
backend message. No `UserSelection`, grant, signed frame, execution or receipt
is produced. Executing/Active/Completed remain reserved states, deliberately
unreachable through scaffold events. No fake networks or audio playback.

Revisions demonstrate invalidation, not cryptographic byte binding: Studio does
not send the draft to the server. Actual note-byte digest/handle/version binding,
the trusted compositor approval strip, credential ingress, admission and LFS
receipt integration must be implemented before enabling real actions.

Entry/exit animation is 180 ms with reduced-motion support. Cards have dialog
labels, explicit buttons, keyboard focus handling and live state feedback. The
UI clears query/draft references on discard; no claim of secure memory erasure,
domain destruction or zero total OS RAM is made.

The note field is a visual draft with a character bound; a future provider must
advertise and enforce its actual UTF-8 byte quota before authorization.

## Validation

`cargo test --locked --manifest-path poc/product/glass/Cargo.toml` verifies the
existing host behavior plus Studio lifecycle and absence of emitted authority.
Browser interaction should cover summon, clarification, editing, read-only
review, rejected approval, discard, focus, and reduced motion on desktop/mobile.

Browser verification on Windows Edge passed at 1440×1000 and 390×844, including
reduced motion; screenshot captures are real host UI frames in `screenshots/`.
The three authority counters remained zero. One live M0 test was not run.

The reusable browser check is `tests/studio_browser.cjs`. Start a dedicated server
on port 7881; provide Playwright through Node module resolution or
`VELA_PLAYWRIGHT_MODULE`, and optionally an installed browser via `VELA_BROWSER`.
Run it from the repository root with `node poc/product/glass/tests/studio_browser.cjs`.
It saves temporary captures under `.local/` and expects an unused host session.

## Exact-plan bridge update

Studio now posts SHA-256-bound UTF-8 plan bytes to `/studio/approve`.
The host rechecks digest/revision and uses the typed fail-closed transport;
no JavaScript signer exists. See [TRUSTED_SELECTION.md](TRUSTED_SELECTION.md)
for the signature contract, conformance tests and remaining native boundary.
