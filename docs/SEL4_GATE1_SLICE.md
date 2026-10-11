# Slice piattaforma Gate 1 — seL4 (non bloccante per M-G1a)

Branch: `grok/sel4-platform` · Base `1281f5a` · Data: 2026-10-12.

Obiettivo: la più piccola slice di piattaforma che **non** tocca `app/`, il wire v2, i marker di `verify-bridge` né il PDP host M-G1a (`poc/m5_executor/policy`), ma riduce il rischio di regressione e fissa le regole che il supervisore dovrà rispettare.

Contenuto della slice:
1. **Controllo pin host-side** (`poc/m5_executor/check-pins.py`) + workflow CI `m5-platform-pins.yml` — eseguibile, secondi, nessun build guest.
2. **Riesecuzione di `verify-bridge`** su questo branch — eseguita, PASS.
3. **Regole di supervisione / crash-no-ack per OWN-v3** — solo design, nessun codice guest.

---

## 1. Controllo pin (`check-pins.py`)

Python 3 stdlib, nessuna dipendenza. Esegue 31 asserzioni su:

- `Makefile`: `BOARD=qemu_virt_aarch64`, `CONFIG=debug`, triple `aarch64-sel4-microkit`, `cargo --locked`, W^X su entrambi gli ELF, QEMU `virt,virtualization=on` + `cortex-a53` + `-nic none`.
- `Dockerfile`: Microkit `2.3.0`, SHA-256 SDK pinnato e verificato con `sha256sum -c`, toolchain `nightly-2025-10-20`, seed PCB solo come BuildKit secret.
- `app/rust-toolchain.toml` coerente con il Dockerfile; `app/Cargo.toml`: rust-sel4 `d441a88…`, `wasmi =2.0.0`, `ed25519-compact =2.6.0`, **assenza** di `biscuit-auth` nel guest (C3); `Cargo.lock` presente.
- `executor.system`: una sola regione 4 KiB; esattamente due PD; `pcb` RW, `executor` R, nessuna mappatura `x`; priorità executor > pcb; un solo canale con `pp=true` solo lato `pcb` e `notify=false` lato executor (nessuna notifica come ingresso, C2).
- `docs/CONTRACTS_v0.md` C6 cita Microkit, toolchain e rev rust-sel4 coerenti.

Risultato oggi (Windows Python 3.13.14 e WSL Python 3.13.5): **31/31 ok**, exit 0.

Cosa **non** prova: che il guest compili o booti, che le mappature runtime corrispondano a quelle dichiarate, né la riproducibilità bit-for-bit. Serve a far fallire subito una modifica accidentale ai pin o ai permessi della pagina condivisa.

```powershell
python poc/m5_executor/check-pins.py
```

## 2. Riesecuzione `verify-bridge` (QEMU) — PASS

Eseguito il 2026-10-12 alle ~00:50 (ora di Roma) sul PC di Francesco, dal worktree `VelaOS-sel4-platform` @ `1281f5a` (nessuna modifica a `app/`):

```powershell
cargo run --locked --manifest-path poc/m5_executor/keygen/Cargo.toml --bin verify-bridge -- `
  /mnt/c/Users/tella/Documents/Codex/2026-10-09/in-c-users-tella-documents-codex/outputs/VelaOS-sel4-platform/poc/m5_executor `
  /mnt/c/Users/tella/Documents/Codex/2026-10-09/in-c-users-tella-documents-codex/outputs/VelaOS-public/.local/m5-executor.seed `
  .local/m5-bridge-sel4-platform.log --wsl
```

- Seed: il seed **di test** locale già esistente (ignorato da git) in `VelaOS-public/.local/`, letto solo come BuildKit secret; corrisponde a `app/src/trusted_key.rs` (ultimo cambio `3e8cfe1`, identico tra i due checkout). Non copiato né committato.
- Esito: `PASS: fresh build, matched acknowledgements, all guest rejection cases, clean idle timeout.` Exit 0, ~2 min 18 s.
- Docker 26.1.5 in WSL; immagine risultante `4036923b10c9`; i layer toolchain/SDK erano probabilmente in cache da una build precedente della stessa sessione (immagine `22da3c48cb31`), quindi "fresh build" del runner significa `docker build` rieseguito, non necessariamente senza cache.
- QEMU nel container: 6.2.0 (Ubuntu 22.04).
- Log seriale completo: [`docs/reports/m5-verify-bridge-2026-10-12-sel4-platform.log`](reports/m5-verify-bridge-2026-10-12-sel4-platform.log) (nessun materiale di chiave nel log; controllato).

Questo chiude per oggi la riproduzione di G0-05; non dice nulla su hardware, RAM o latenza.

## 3. Design OWN-v3: supervisione e crash senza ack (solo design)

Stato: **documentato**, nessun codice. Estende OWN-v2 senza cambiarne il wire. Da implementare in Gate 1b/2 dopo decisione sul PD supervisore.

### 3.1 Nuovo PD `supervisor` (proposta)

- Priorità **più alta** di `pcb` ed `executor` (es. 110), stack piccolo, nessuna mappatura della pagina dati.
- È il **fault handler** Microkit di `pcb` ed `executor` (in Microkit un PD figlio ha come fault handler il genitore: richiede di annidare `pcb` ed `executor` dentro `supervisor` in `executor.system`; da verificare sulla versione 2.3.0 prima di implementare).
- Possiede l'unica fonte di **epoch**: un `u64` monotono incrementato a ogni boot e a ogni riavvio dell'Executor. Senza storage persistente l'epoch è solo boot-local → **non** sufficiente per replay cross-boot (serve storage, vedi dipendenze).
- Canali: `supervisor`↔`pcb` (notifica "executor ripristinato, nuova epoch") e `supervisor`↔`executor` (avvio con epoch). Nessun byte di task transita dal supervisore.

### 3.2 Regole (normative per l'implementazione)

| # | Regola |
| --- | --- |
| S1 | Il PCB considera una richiesta conclusa **solo** con risposta `0x702` correlata (request ID atteso, numero registri corretto). Nessun altro evento vale come ack. |
| S2 | Se l'Executor va in fault durante una protected call, il kernel non consegna una risposta al PCB; il supervisore riceve il fault. Il PCB marca la richiesta in volo come **`AMBIGUOUS` (14)**, non la ripete e **non scrive** la pagina finché il supervisore non annuncia la nuova epoch. |
| S3 | Il supervisore **non** riavvia l'Executor riutilizzando stato in memoria: azzera lo stato del PD (o lo riavvia dall'immagine), incrementa l'epoch, poi notifica il PCB. |
| S4 | Dopo la nuova epoch, request ID ripartono **solo** sotto la nuova epoch; la chiave di idempotenza è `(epoch, request_id)` (C4). Nessun riuso di `(epoch, id)`. |
| S5 | L'esito di una richiesta `AMBIGUOUS` si risolve **solo** dal receipt LFS (C4). Senza LFS in guest (oggi), resta `AMBIGUOUS` e i task con effetti non vengono rieseguiti automaticamente; i task puri (senza effetti) possono essere riproposti dall'utente come **nuova** richiesta. |
| S6 | Il PCB non è mai riavviato silenziosamente: un fault del PCB porta il supervisore a fermare l'Executor e dichiarare `UNAVAILABLE` (15) fino a nuova epoch. |
| S7 | Timeout: senza clock fidato non esiste deadline wall-clock. Finché manca un driver timer, il supervisore non può rilevare un Executor bloccato fuori dal fuel (es. parsing). Fino ad allora: limite noto, non un comportamento garantito. |
| S8 | Le notifiche seL4 restano hint: nessun task e nessun ack viaggiano su notifica. |
| S9 | Nessun LLM nel percorso supervisore/epoch/ack. |

### 3.3 Prove richieste prima di dichiarare OWN-v3 implementato

1. Executor che va in fault a metà task (modulo di test che provoca fault fuori dal Wasm, es. via flag di test nel build) → PCB riporta `AMBIGUOUS`, pagina non riscritta, nuova epoch annunciata, richiesta successiva eseguita con `(epoch+1, 1)`.
2. Replay di un frame della vecchia epoch dopo il riavvio → rifiutato.
3. Fault del PCB → Executor fermo, nessuna esecuzione fino a nuova epoch.
4. Marker `verify-bridge` esistenti invariati (o versionati con ADR).

## Cosa resta fuori da questa slice (volutamente)

- Modifiche a `app/` (guest), al wire v2 o ai marker.
- Biscuit in guest, epoch sul filo, clock, storage, driver.
- CI che compila/avvia il guest (vedi dipendenze: runner Linux con Docker + secret di test, ~minuti per run).