# Contextual Glass — implementation status

The Piega interface adapts its layout to the selected intent. Paper, Graphite and Dawn use flat surfaces, precise typography and a persistent context toolbar. This is Vela Soft on Windows, not a complete native desktop OS.

| Context | Implemented behavior | Boundary |
| --- | --- | --- |
| Notes / Tracce | Hydration, full reads, multiple documents, immutable edits, digest-bound approval, verified receipts, text export | Native broker → VirtIO → PCB → Wasmi → LFS; 64 retained records, 1 KiB per note |
| Study / Margine | Exact saved source alongside deterministic cloze recall | Extractive browser workflow; no generated answers or PDF import |
| Audio / Ritmo | User-selected local audio, playback, seeking, volume and explicit session cleanup | Browser decoder on the host; no guest audio driver |
| Code / Officina | Local text preview and preparation of a bounded snippet for signed storage | No arbitrary execution, compiler or debugger |
| Connection / Soglia | Real browser connectivity indication and Windows Wi-Fi settings delegation | No invented SSIDs; no guest networking driver |

Intent routing is a deterministic local lexical classifier. Ambiguous input stays a literal note. It is not an integrated generative SLM. The approval dialog remains bound to the exact note digest and completes only after validating the returned receipt. Switching context is blocked during transactions and prompts before abandoning an edited document.

## Verification

- `cargo test --manifest-path poc/product/glass/Cargo.toml`: 6 unit and 13 integration tests passed; one live M0 model test ignored.
- `node poc/host-broker/tests/crud-ui.cjs .local/soft-dev-e2e`: multiple documents, immutable original and reload passed through the actual guest.
- `node poc/host-broker/tests/contextual-ui.cjs .local/soft-dev-e2e`: actual timeline, exact study source, distinct themes, local code preview/release, real audio playback/release, host network adapter and 390 px layout passed; no browser errors.

These runs used a separate test disk and explicit `DevelopmentVirtualFido` terminal approval. Ed25519 and disk receipts were real; the developer touch was synthetic, not hardware FIDO2 evidence. Local screenshots and results are kept outside tracked files under `.local/soft-dev-e2e`.

## Launch and remaining gates

Use `vela-up.bat` or the existing Vela Soft DEV desktop shortcut. Keep the consent terminal open. Current launch requires the configured WSL/Docker image and local keys; it is not a standalone installer.

Still required: hardware FIDO2 positive validation, production key custody, portable installer, larger document storage, PDF ingestion, generative local inference, guest multimedia/network drivers and arbitrary sandboxed coding workflows. No idle RAM or latency target is claimed from these browser checks. Historical receipts do not record approval provenance and display it as unrecorded.
