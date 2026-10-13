# Glass redesign — native note bridge client

Implemented on `vela/glass-redesign`, from the working native note bridge at
`998efa0`. The complete self-contained HTML/CSS/JS is in
`poc/product/glass/assets/studio.html`. No external font, asset or frontend package
is required. This is a host renderer, not a new native framebuffer domain.

## Implemented and exercised

- Monochrome slate interface, one blue action accent, rectangular controls,
  system typography and monospace hashes/receipt identifiers. No gradients,
  shadows, glow, rounded cards, chat bubbles or backdrop filters.
- Central universal input. Enter prepares an explicitly labelled local
  simulation of a note proposal; Enter never sends an approval.
- Editable UTF-8 payload, exact SHA-256 preview, 1024-byte boundary, explicit
  reject and authorize controls. Signing freezes a private snapshot of the text.
- Real WebSocket `ws://127.0.0.1:7990/`, unchanged broker Proposal schema and
  native QEMU workflow. The browser waits for the broker-verified signed receipt.
- A transport ACK leaves the UI Loading. Only a consistent Receipt with matching
  hash, request ID and generation completes the transaction. Missing, denied or
  malformed results produce no Completed timeline entry.
- Completed prompt removal, newest-first scrollable session receipt timeline,
  receipt inspection, and clickable Everything/Work/System context filters.
  Context metadata has no authority effect and is not transmitted as permission.
- No fake initial history or browser-cache claim of disk durability. Timeline
  disappears on page reload; the real guest note remains on disk.

## Executed checks

```text
cargo build --manifest-path poc/product/glass/Cargo.toml
node poc/host-broker/tests/e2e-ui.cjs .local/v1
node poc/host-broker/tests/redesign-ui.cjs .local/v1
```

Playwright + Edge E2E used the real broker and QEMU, reached Completed, dismissed
the prompt, inserted one receipt and exercised context filters. Request
`1791901085007` returned a 170-byte signed receipt using
`MockUnavailableDevelopment`. No real Windows biometric verification is claimed.

UI checks passed: reject emits zero frames, UTF-8 oversize blocks approval,
displayed SHA-256 matches independently computed payload hash, an isolated
injected ACK cannot complete, payload stays locked while Loading, and the 390px
viewport has no horizontal overflow. ACK fault injection is a separate UI test;
it is not the real-broker integration test.

Evidence: `.local/v1/ui-result.json`, `redesign-ui-result.json`,
`redesign-idle.png`, `redesign-proposal.png`, `redesign-mobile.png`.

## Project work still open

The redesign closes these four host UI components, not the entire project table.
Remaining delivery gates are:

1. Guest-backed multi-object index, reads, versions, encryption, import/export,
   and recovery; the current note action replaces a single fixed test object.
2. Qualified real human input, key custody, enrollment/revocation, cross-boot
   authority freshness and a separate LFS/notary domain.
3. Persistent semantic recall and native AI service integration, with a portable
   local backend and measured whole-system memory/latency.
4. Native display/input Glass and actual target hardware support: storage, GPU,
   Wi-Fi, audio, battery, USB and suspend/resume.
5. Browser, media decoding/playback, development tools and demonstrated game
   compatibility. None is enabled by this HTML redesign.
6. Windows installer/pilot packaging and native installer/update/recovery,
   accessibility qualification, clean-machine reproduction and user trials.

Each gate needs its own executable test and supported hardware/software scope.
The simulation label and synthetic-consent label must remain visible until their
respective real implementations are qualified.
