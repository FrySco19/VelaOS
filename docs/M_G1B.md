# M-G1b / M-G1c / verify-bridge-e2e — Candidate ≠ Authority, State Ledger, QEMU E2E

Branch `vela/velaos-final-e2e` · 2026-10-12 · stato: **PoC, gate verificato in QEMU virt AArch64** (non hardware).

Questo documento descrive cosa è implementato, cosa è simulato e come riprodurlo. Contratti di riferimento: `docs/CONTRACTS_v0.md` (C1 `authority_digest`, C3 livelli di autorità, C4 receipt, C5 codici), `poc/m5_executor/BRIDGE_CONTRACT.md`, `poc/m5_executor/EXECUTOR_CONTRACT.md`.

## 1. M-G1b — Candidate ≠ Authority (host PDP, std)

Codice: `poc/m5_executor/policy/src/grant.rs` (merge di `vela/m-g1b-candidate-grant` @ `2fc093c`) + `policy/src/pipeline.rs` (nuovo).

```text
mock AI intent -> MockPlanner::propose -> [Candidate]            (solo dati: nessun token, nessun diritto)
  -> TrustedSelectionChannel::select -> UserSelection{handle, expected_version}   (unico costruttore)
  -> GrantIssuer: Biscuit LFS_READ attenuato (op, caller, handle, version, not_after <= 120 s)
  -> verify_read_grant_at (verifica lato lettore, difesa in profondità)
  -> authority_digest = BLAKE3-256(grant)  -> sezione custom Wasm `vela.grant` nel payload firmato
  -> token execute attenuato (caller, executor, execute, payload_bytes, fuel, not_after del grant)
  -> PolicyDecisionPoint::admit_at (M-G1a) -> bridge::signed_frame (wire v2 invariato)
```

Regole:
- Il planner/LLM produce solo `Candidate`. `planner_note` è testo non fidato, solo audit, mai interpretato.
- `SelectionGatedPdp` tiene private la delega read, la delega execute e la chiave PCB: non esiste API che firmi un frame senza un `UserSelection` valido.
- Senza `UserSelection` → `POLICY_DENIED` (C5 10): nessun grant, nessun frame, nessuna Wasm (`grants_issued == frames_signed == 0`).
- Il digest del grant è dentro il payload, quindi coperto dalla firma Ed25519 di admission: l'Executor lo vede solo dopo aver autenticato il frame.

Test (`cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml`):
- `tests/candidate_grant.rs` (10): grant solo da selezione esplicita, single-use, TTL, versione, scope.
- `tests/m_g1b_pipeline.rs` (10): AI senza selezione → `POLICY_DENIED`; nota AI che "dichiara consenso" ignorata → `POLICY_DENIED`; AI che ridirige la selezione su un altro handle o su un handle mai proposto → `SCOPE`; riuso selezione → `REPLAY`; versione stantia → `VERSION_CONFLICT`; TTL > 120 s → `EXPIRED`; fuel > MAX_FUEL dopo il grant → `SCOPE`, nessun frame; caso valido → grant attenuato verificabile, scaduto dopo `not_after`, frame verificabile con digest legato; parser `vela.grant` robusto (mancante/duplicato/troncato); frame ammesso eseguito da Wasmi host (`vela-executor-host`) → 21, replay → status 4.

## 2. M-G1c — State Ledger (receipt LFS, simulato)

Codice: `poc/m5_executor/app/src/ledger_wire.rs` (`no_std`, condiviso guest/host come `bridge.rs`) + `policy/src/ledger.rs` (orchestrazione host).

- Commit PCB → LFS sincrono: `(epoch, request_id, status, value, authority_digest)`.
- Il writer persiste solo se: status Executor = 0; il frame sulla pagina RO autentica con la chiave PCB pinned; `request_id` coincide; il payload contiene esattamente lo stesso `authority_digest`. Altrimenti `POLICY_DENIED` (10) / `MALFORMED` (1), **nulla persistito** (C4.4).
- Idempotenza `(epoch, request_id)` (C4): stessa chiave e stesso contenuto → stesso receipt (stesso `seq`, stessa firma), nessun nuovo effetto; contenuto diverso → `IDEMPOTENCY_CONFLICT` (13).
- Receipt: Ed25519 della chiave LFS su `VelaOS.LFS.Receipt.v0\0 ‖ epoch ‖ request_id ‖ status ‖ value ‖ authority_digest ‖ seq`.
- Il PCB (`Notary`) verifica firma con la chiave LFS pinned, che il corpo corrisponda a ciò che ha chiesto, e archivia (append-only); receipt falsificato/estraneo/riscritto → rifiutato.
- Esito ambiguo (risposta persa): si ritenta il commit o si interroga il notary; mai riesecuzione.

Test `tests/lfs_receipt.rs` (9): percorso di successo con receipt notarizzato; retry ×3 → stesso receipt, nessuna riesecuzione Wasm, 1 solo effetto; conflitto di idempotenza; tentativo AI-only → Executor mai chiamato, nulla persistito; output non autorizzato (request mai ammessa, digest sbagliato, esecuzione fallita, frame forgiato) non persistito; esecuzione fallita / ack sbagliato → nessun commit; notary rifiuta firma alterata, firma di chiave non-LFS, corpo diverso, storia riscritta; layout registri IPC host = guest; capacità limitata.

## 3. verify-bridge-e2e (QEMU)

Sistema `executor-e2e.system`: PD `pcb` (100, page rw), `executor` (101, page r), `lfs` (102, page r); canali pp PCB→Executor (0x701/0x702) e PCB→LFS (0x711/0x712, 8/11 registri). Build `make build E2E=1` (feature cargo `e2e`; il build normale e `verify-bridge` restano invariati).

Esecuzione:

```powershell
# 1) metà host (std PDP): intent mock -> candidati -> UserSelection -> grant Biscuit -> frame firmato
cargo run --locked --manifest-path poc/m5_executor/policy/Cargo.toml --example e2e_bundle -- `
  ..\VelaOS-public\.local\m5-executor.seed .local\m5-lfs.seed poc\m5_executor\app\generated
# 2) guest in QEMU (Docker in WSL: path /mnt/c/... e --wsl)
cargo run --locked --manifest-path poc/m5_executor/keygen/Cargo.toml --bin verify-bridge-e2e -- `
  <ctx>/poc/m5_executor <pcb.seed> <lfs.seed> .local\m5-verify-bridge-e2e.log --wsl
```

Il runner esige 33 marker esattamente una volta (14 storici incluso R1 MAX_FUEL + 19 e2e), nessun `panicked`/`FAULT`, uscita 124 (timeout di idle dopo il completamento). Log: `docs/reports/m5-verify-bridge-e2e-2026-10-12.log`.

Sequenza guest dopo i 10 request storici: [1] candidati (dati); [2] request 11 AI-only negata (`POLICY_DENIED`, nessun frame pubblicato); [3] selezione → grant (472 B, TTL 120 s, digest); [4] frame request 12 sulla pagina 4 KiB; Executor verifica Ed25519 e riporta il digest autenticato; [5] Wasm → 21 (fuel 10 000); [6] LFS commit → receipt seq 1; [7] PCB notary verifica e archivia; N1 retry → stesso receipt `replayed=1`; N2 valore diverso → 13; N3 output per request 11 → 10 non persistito; N4 receipt alterato rifiutato; N5 replay frame 12 → status 4, nessuna Wasm, nessun commit.

## 4. Cosa è reale e cosa è simulato

| Elemento | Stato |
| --- | --- |
| AI intent / planner | **mock** deterministico (ranking per parole); nessun LLM |
| UserSelection | **mock** (canale fidato host); nessuna shell/compositor reale |
| Biscuit (grant LFS_READ + token execute) | **reale** (`biscuit-auth` 6.0.0) ma **solo host std**, eseguito prima del boot; root Biscuit **effimera** per run (trust root di test) |
| Verifica Biscuit nel guest | **assente** (std-only, C3); il guest vede la firma di admission PCB + digest del grant nel payload firmato |
| Firma admission Ed25519 / Executor / Wasmi fuel-safe | **guest-native** in QEMU |
| Writer LFS | **dominio guest reale** ma **ledger simulato**: in memoria, capacità 8, nessuno storage durevole, nessun handle/versione di scrittura, nessuna epoch cross-boot |
| Notary PCB | **guest-native**, in memoria |
| Scadenza grant / clock fidato | **non** verificata nel guest (nessun clock fidato); il TTL vale solo nel PDP host |
| Valore risultato | il PCB inoltra al writer il valore restituito dall'Executor (il writer si fida del PCB per il valore; canale Executor→LFS diretto: futuro) |
| Chiavi | seed PCB e LFS locali (`.local/`, gitignored, BuildKit secret); non sono trust root di produzione |

Prossimi passi naturali: epoch persistente/OWN-v3 con supervisore, envelope v0 sul filo, Biscuit (o verificatore Datalog minimale) nel guest, storage LFS durevole con `result_handle`/`expected_version`, misure su hardware dichiarato.
