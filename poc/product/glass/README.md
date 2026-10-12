# VelaOS Glass (host demo) + real M0 recall lane

Host-only product demo on top of `poc/intent`, `poc/product/shell` and the
`SelectionGatedPdp` in `poc/m5_executor/policy`. "L'IA propone, l'umano dispone,
la crittografia garantisce."

```
utterance --POST /submit--> ShellRuntime::submit -> IntentEngine
   lanes: BM25 (Rust, vela-intent)  +  M0 (poc/m0/bridge_recall.py: e5-small dense + LanceDB BM25 + RRF)
   -> RRF k=60 -> Candidates (data only, auto_resolve=false)  -> glass page
click Avvia / Rivedi piano --POST /click {action, candidate INDEX}-->
   ShellRuntime::click -> UserSelection{handle, expected_version} (re-derived from the shown row)
   -> TrustedSelectionChannel -> SelectionGatedPdp::admit_task_at -> (Avvia) HostExecutor
```

## Surfaces

Glass is a temporary panel above the ISO §7 desktop (frost, teal-underline dock,
window controls, top-right notifications, Welcome in English or Italian, themes
Light / Dawn / Deep). It is not a second shell and it is not an authority.
Allowed components, the chrome Glass must not own, and the anti-patterns are in
[docs/ALLOWED_COMPONENTS.md](docs/ALLOWED_COMPONENTS.md). The same lists are
[`src/catalog.rs`](src/catalog.rs).

The host page still paints a header and a theme row. Those are stand-ins. The
catalog denies them as product chrome. This crate does not grow them into a toolbar
or a dock.

## Open it

```powershell
cd poc\product\glass
cargo run --offline --release -- --m0 <path to a poc/m0 checkout with .venv and .model-cache>
# e.g. --m0 ..\..\..\..\VelaOS-public\poc\m0   (or set VELA_M0_DIR)
# then open http://127.0.0.1:7878/
```

Without `--m0`/`VELA_M0_DIR` (or if the M0 checkout lacks `.venv`/`.model-cache`) the
glass runs BM25 + the deterministic title stand-in and shows why in the footer.
Each M0 query spawns the Python bridge (cold model load: ~5-11 s on this host).

## Guarantees (tested, host model)

* Only `POST /click` with `avvia`/`rivedi` and a shown candidate index reaches the PDP.
  Bodies carrying `handle`, `expected_version`, `selection`, `frame`, `grant`, `ttl`, ... are 400.
  No `/admit`, `/grant`, `/select`, `/execute` route (404).
* Loopback bind, `Host` pinned to the bound port, per-run token, `application/json`
  required, foreign `Origin` refused; untrusted text rendered via `textContent` only.
* M0 lane fails closed (empty lane) on missing checkout, timeout, schema error,
  `auto_resolve != false` or unpinned embedding epoch. It never returns versions or rights.

## Tests

```powershell
cargo test --offline                      # poc/product/glass (10) and poc/product/m0-recall (6)
$env:VELA_M0_DIR='<poc/m0 checkout>'; cargo test --offline -- --ignored   # live M0: glass 1, m0-recall 2
```

`tests/fixtures/m0_demo_output.json` is verbatim real bridge output for the demo
corpus (reproduce with `cargo run --offline --example m0_request | python bridge_recall.py`).

## Not verified / limits

* Host model only: in-process privacy + loopback HTTP is not the compositor's trusted
  input path or a protection-domain boundary.
* M0 bridge returns every card (limit 64, no score floor), so low-relevance rows appear.
* Demo keys (fixed PCB seed, fresh Biscuit root per run) are NOT production key management.
* No persistence: corpus is the five authored M0 fixture cards + two workspace items.