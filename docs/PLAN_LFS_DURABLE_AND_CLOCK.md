# Piano misurabile — LFS durevole e clock fidato per TTL grant

Stato: **solo piano**. Nessuna implementazione in questo task.  
Base: `1c94bd1` (`grok/velaos-final-e2e` / `grok/release-prep`).  
Autore: vela-bot · 2026-10-12.  
Dipendenze prodotto: **iniziare solo DOPO** la chiusura di shell/intent (`grok/product-intent-shell` o equivalente).

## 0. Cosa è già dimostrato vs cosa resta aperto

| Elemento | Stato su `1c94bd1` | Evidenza |
| --- | --- | --- |
| Receipt LFS + idempotenza `(epoch, request_id)` (simulato) | **già dimostrato** (host + QEMU E2E) | `docs/M_G1B.md` §2–3; `poc/m5_executor/policy/tests/lfs_receipt.rs` (9); `docs/reports/m5-verify-bridge-e2e-2026-10-12.log` |
| Writer guest + PCB notary | **già dimostrato** in QEMU (ledger in memoria) | `poc/m5_executor/app/src/ledger_wire.rs`; sistema `executor-e2e.system` |
| Capacità ledger / persistenza | **aperto** — ledger simulato, in memoria, cap guest **8** (`app/src/bin/lfs.rs` `CAPACITY = 8`) e host **64** (`policy/src/ledger.rs` `LEDGER_CAPACITY`), nessun storage durevole, nessuna epoch cross-boot | `docs/M_G1B.md` §4 tabella riga "Writer LFS" |
| Scadenza grant / clock fidato nel guest | **aperto** — TTL ≤ 120 s solo nel PDP host; guest senza clock fidato | `docs/M_G1B.md` §4; `docs/CONTRACTS_v0.md` C1 `not_after` / `quota.deadline_ms` |
| SelectionGatedPdp + UserSelection | **già dimostrato** (host) | `docs/M_G1B.md` §1; `policy/tests/m_g1b_pipeline.rs` |
| Wire v2 / Executor fuel-safe (R1) | **già dimostrato** | `poc/m5_executor/EXECUTOR_CONTRACT.md` EXEC-v2.1; log R1 + E2E |
| Envelope C1 completo / OWN-v3 / `result_handle` | **aperto** | `docs/CONTRACTS_v0.md` C1–C4 "oggi" vs target |

**Vincolo di questo documento:** non scrivere codice, non pushare, non rieseguire QEMU qui. Solo milestone ordinate con criteri di uscita misurabili.

---

## 1. LFS durevole — obiettivo

Portare il writer da **ledger in-memory** (`Ledger<N>` in `ledger_wire.rs` / orchestration in `policy/src/ledger.rs`) a un store **append-only con fsync** (o motore tipo redb come pattern M1, se compatibile con il guest o con un dominio host dedicati), con:

1. **Persistenza epoch** attraverso reboot (file/region dedicata, firmata o almeno checksummata + versionata).
2. **Idempotenza** invariata: chiave `(epoch, request_id)` — stesso contenuto → stesso receipt; contenuto diverso → `IDEMPOTENCY_CONFLICT` (13).
3. **Negazione scrittura non autorizzata**: output senza admission autenticata / digest sbagliato / status ≠ 0 → niente persistenza (già in host test; va ribadito sul path durevole).
4. **Nota power-loss**: crash a metà append non deve produrre receipt "OK" leggibile dal notary; recovery = ripresa da ultimo record integro o stato `AMBIGUOUS` fino a interrogazione notary (allineato a C4 / OWN-v3).

### Scelta di archivio (decisione a M-LFS-1)

| Opzione | Pro | Contro | Quando sceglierla |
| --- | --- | --- | --- |
| A. Append-only file + fsync (record fixed-size) | Semplice, auditabile, adatto a PoC QEMU virtio-blk | Serve recovery/CRC; niente indici ricchi | Default raccomandato per Gate LFS |
| B. redb (o simile) su host/std bridge | Transazioni, già familiare da M1 | `std` / non entra nel guest Microkit `no_std` senza bridge | Se il writer durevole resta **host** dietro un dominio proxy |
| C. Flash raw guest | Vicino a prodotto embedded | Board fisica + driver; fuori scope finché non c'è board | Solo dopo scelta hardware |

**Raccomandazione piano:** M-LFS-1..3 su **Opzione A** nel path di test (host +, se fattibile, guest con blocco virtio o file montato nel runner); Opzione B solo se si decide esplicitamente di tenere il durable store fuori dal guest.

---

## 2. Clock fidato per TTL grant

### 2.1 Fonti di tempo

| Dominio | Fonte oggi | Necessità TTL |
| --- | --- | --- |
| PDP host (`SelectionGatedPdp` / `admit_at`) | `SystemTime` / orologio wall host | **già** usa `not_after` ≤ 120 s |
| Guest PCB / Executor / LFS | **nessun** clock fidato | non può scadere grant in-guest |
| UX | mock / prototipo | deve mostrare expiry dal grant host, non inventare tempo |

### 2.2 Monotono vs wall

- **Wall clock** (`not_after` assoluto): serve UX ("scade tra 90 s") e policy host; vulnerabile a salti NTP/RTC.
- **Monotonic clock** (tick da boot): adatto a deadline relative in-guest; non sopravvive a reboot senza epoch nuova.
- **Policy proposta:**
  - Host PDP: wall per emettere `not_after` (come oggi), con **fake clock iniettabile** nei test.
  - Guest (quando arriverà): monotonic per *deadline relative* firmate nell'admission; a boot nuova `epoch` invalida grant vecchi (OWN-v3).
  - Se **clock assente** nel guest: comportamento attuale = **fail-open sul tempo** (TTL solo host) **non accettabile** per release "daily-use"; per preview QEMU documentare esplicitamente "TTL enforced solo pre-boot/host". Per Gate clock: se il guest non ha fonte, **rifiutare** operazioni che dipendono da `not_after` in-guest (fail-closed), oppure restare in modalità "host-only TTL" etichettata preview.

### 2.3 UX e expiry

- La Glass UI deve mostrare `not_after` / TTL residuo **solo** se proveniente dal grant emesso dopo `UserSelection` (campo audit del PDP), non da testo LLM.
- Alla scadenza: stato shell → errore policy chiaro (`EXPIRED` / C5), nessun retry automatico che firmi di nuovo senza nuova selezione.
- Prototipo HTML e shell prodotto allineati a `docs/UX_ZERO_APP.md` / `docs/UX_FLOWS.md` sul branch `grok/ux-zero-app` (non presenti su questo albero `1c94bd1`; concetti visuali: `docs/concepts/2026-10-12/`).

### 2.4 Piano test clock

1. **Host fake clock:** trait/`Clock` iniettato nel PDP; avanzare oltre `not_after` → `EXPIRED`, zero frame.
2. **Host skew:** clock che torna indietro → policy documentata (rifiuto o clamp); test esplicito.
3. **QEMU (dopo M-CLK-2):** scenario E2E in cui il bundle host emette grant con TTL cortissimo; runner avanza tempo host prima del boot **oppure** guest legge tick monotonic firmato — exit criteria: grant scaduto non produce commit LFS; grant valido sì.
4. **Assenza clock guest:** test/documentazione che il path preview dichiara "TTL host-only" e non claima enforcement guest.

---

## 3. Milestone ordinate (M-LFS / M-CLK)

### M-LFS-1 — Contratto store + ADR scelta A/B
- **Fare:** ADR breve in `docs/` (o sezione qui aggiornata) che fissa Opzione A o B; schema record append-only (magic, version, CRC/len, body receipt, pad); API `DurableLedger: append / get(epoch,request_id) / recover`.
- **Non fare:** integrare nel guest ancora.
- **Exit criteria:**
  - File ADR committato; review Fairha OK.
  - Test host su crate dedicata o `policy` con backend **memory** che implementa lo stesso trait del futuro durable (comportamento = ledger attuale).
  - Comando: `cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml lfs_receipt` → **pass** (regressione zero).
- **Dipendenze:** `ledger_wire` invariato semanticamente; `SelectionGatedPdp` non toccato.

### M-LFS-2 — Append-only + fsync (host)
- **Fare:** backend file sotto `.local/lfs-ledger/` (gitignored); ogni commit: write record → `fsync` file → `fsync` dir; recover a open.
- **Exit criteria:**
  - Test: kill simulato dopo write pre-fsync → recover senza receipt fantasma OK.
  - Test: retry ×3 stesso `(epoch, request_id)` → stesso receipt, 1 effetto.
  - Test: conflitto contenuto → 13.
  - Test: AI-only / digest sbagliato → 0 byte persistiti sul file (size invariata o solo record rifiuto non-OK se si logga — preferire **nessun** record di successo).
  - Comando: `cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml --test lfs_receipt` + nuovi test `lfs_durable_*` → **tutti pass**.
- **Dipendenze:** M-LFS-1; riuso tipi `Receipt` / `Commit` da `ledger_wire`.

### M-LFS-3 — Epoch persistente cross-boot (host)
- **Fare:** file `epoch.bin` (o header ledger) con epoch corrente; reboot processo host → stessa epoch se clean shutdown flag; crash → bump epoch **oppure** recover + marca `AMBIGUOUS` per in-flight (scelta ADR).
- **Exit criteria:**
  - Test: due sessioni processo, stesso seed → receipt della sessione 1 leggibile in sessione 2.
  - Test: bump epoch → vecchi `(epoch, id)` non collidono con nuovi id.
  - Documentato comportamento power-loss in `docs/M_G1B.md` o ADR.
- **Dipendenze:** M-LFS-2; allineamento a C4 / OWN-v3 in `CONTRACTS_v0.md`.

### M-LFS-4 — Collegamento guest QEMU (opzionale ma gate "preview forte")
- **Fare:** dominio LFS scrive sullo store durevole (file via runner host **oppure** virtio-blk minimale). Marker E2E aggiuntivi: receipt seq sopravvive a riavvio QEMU (stesso seed).
- **Exit criteria:**
  - `verify-bridge-e2e` (o variante `verify-bridge-e2e-durable`) **PASS**, log sotto `docs/reports/`.
  - Almeno 1 marker "ledger reopen after reboot".
  - Regressione: `verify-bridge` classico ancora PASS.
- **Dipendenze:** M-LFS-2/3; non mescolare con product shell.

### M-LFS-5 — `result_handle` / `expected_version` (C4 completo)
- **Fare:** write path controlla handle + versione attesa; UX/shell possono mostrare handle opaco.
- **Exit criteria:** test host negazione versione stantia; E2E opzionale.
- **Dipendenze:** product intent/shell chiusi; envelope verso C1.

### M-CLK-1 — Fake clock nel PDP host
- **Fare:** astrazione `Clock` usata da grant/TTL; test `EXPIRED` deterministici.
- **Exit criteria:** `cargo test` policy include `clock_*` / TTL con fake clock → pass; nessun uso diretto non wrappato di `SystemTime` nel path grant (grep CI).
- **Dipendenze:** nessuna LFS; può procedere in parallelo a M-LFS-1.

### M-CLK-2 — Contratto UX expiry + fail mode senza clock guest
- **Fare:** doc + test shell/intent (quando esistono) che mostrano TTL; checklist release aggiornata.
- **Exit criteria:** criterio "usable preview" elenca TTL host-only finché M-CLK-3 non passa.
- **Dipendenze:** product shell; M-CLK-1.

### M-CLK-3 — Enforcement tempo in path E2E (host pre-boot o guest monotonic)
- **Fare:** uno dei due path scelti in ADR; QEMU dimostra grant scaduto → no commit.
- **Exit criteria:** log E2E con marker expiry; `verify-bridge-e2e` esteso PASS.
- **Dipendenze:** M-CLK-1; M-LFS-2 consigliato prima se si combina durable+expiry.

---

## 4. Ordine consigliato e parallelismo

```text
[product shell/intent CHIUSO] ──┬──> M-LFS-1 → M-LFS-2 → M-LFS-3 → M-LFS-4 → M-LFS-5
                                 └──> M-CLK-1 → M-CLK-2 → M-CLK-3
```

- M-CLK-1 può iniziare in parallelo a M-LFS-1 **dopo** il via Fairha, ma **non** in questo task.
- Non merge su `main` senza CI verde e OK coordinamento.

---

## 5. Dipendenze codice esistenti (non modificare qui)

- `SelectionGatedPdp` / grant Biscuit: `poc/m5_executor/policy/src/` (`grant`, `pipeline`).
- Wire receipt: `poc/m5_executor/app/src/ledger_wire.rs`.
- Orchestrazione host: `poc/m5_executor/policy/src/ledger.rs`.
- Runner QEMU E2E: `poc/m5_executor/keygen/src/bin/verify-bridge-e2e.rs`.
- Contratti: `docs/CONTRACTS_v0.md` C4–C5; `poc/m5_executor/BRIDGE_CONTRACT.md`.

---

## 6. Rischi residui

| ID | Rischio | Mitigazione nel piano |
| --- | --- | --- |
| R-LFS-1 | Power-loss lascia record parziale letto come OK | CRC + recover; nessun commit annunciato pre-fsync |
| R-LFS-2 | Cap ledger / disco pieno | `QuotaExceeded` (11); test esplicito |
| R-LFS-3 | Guest `no_std` non può usare redb | Preferire A o proxy host |
| R-CLK-1 | Wall clock host manipolabile | Fake clock test; per produzione: fonte RTC/TPM o monotonic+epoch |
| R-CLK-2 | Claim "TTL enforced" senza guest clock | Checklist release: vietato claim daily-use |
| R-INT-1 | Product shell non ancora su questo branch | Gate esplicito "dopo shell/intent" |
| R-KEY-1 | Seed PCB/LFS di test ≠ trust root produzione | Già in `M_G1B.md`; resta per release |

---

## 7. Dichiarazione

**Questo task non avvia implementazione.** Solo documenti e commit sul branch `grok/release-prep`. L'esecuzione delle milestone richiede direttiva Fairha dopo la chiusura product-intent-shell.

---

## 8. Addendum - evidenze di codice verificate su `1c94bd1` (lette il 2026-10-12)

| Fatto | Dove | Impatto sul piano |
| --- | --- | --- |
| Ledger guest `Ledger<CAPACITY>` con `CAPACITY = 8`, marker boot "simulated in-memory ledger, capacity=8" | `poc/m5_executor/app/src/bin/lfs.rs` r. 27-48 | M-LFS-2/4: al riempimento oggi non c'è persistenza; test `QuotaExceeded` (11) obbligatorio |
| Ledger host `LEDGER_CAPACITY = 64` | `poc/m5_executor/policy/src/ledger.rs` r. 19 | divergenza host/guest da documentare nell'ADR M-LFS-1 |
| Wire commit/receipt: label 0x711/0x712, 8/11 registri, dominio firma `VelaOS.LFS.Receipt.v0\0`, `BODY_LEN = 64` | `poc/m5_executor/app/src/ledger_wire.rs` r. 27-36 | il record durevole deve contenere **esattamente** questo body + firma; nessun cambio wire senza nuovo `verify-bridge-e2e` |
| TTL max `MAX_READ_GRANT_TTL = 120 s`; check Datalog `time({now})` vs `exp` | `poc/m5_executor/policy/src/grant.rs` r. 30, 241, 281 | il TTL è **solo** host |
| Seam di tempo già esistenti: `issue_read_grant_at`, `verify_read_grant_at`, `admit_at`, `admit_task_at`, `run_task_at`; wrapper che chiamano `SystemTime::now()` | `grant.rs` r. 163-166, 255; `lib.rs` r. 170-174; `pipeline.rs` r. 215-218; `ledger.rs` r. 110 | M-CLK-1 è piccolo: introdurre `trait Clock` e far passare i wrapper da lì; i test esistenti usano già `now` esplicito |
| Pattern M1 redb: `redb = "=4.3.0"`, `Durability::Immediate` su ogni transazione di scrittura, log firmato `signed_commit_log`, probe di fase `put_with_probe` + `crash_gate` (kill reale di processo) | `poc/m1/Cargo.toml` r. 20; `poc/m1/src/lib.rs` r. 285, 490, 547; `poc/m1/src/bin/crash_gate.rs`; CI `.github/workflows/lfs-m1.yml` (200 kill) | M-LFS-2/3 riusano **il metodo** di crash test di M1 (probe per fase + kill del processo), anche se si sceglie l'opzione A |

## 9. Comandi esatti per i criteri di uscita

Tutti da eseguire dalla radice del worktree, PowerShell su Windows (come Gate 0), toolchain pin `1.96.1`.

### 9.1 Regressione (deve restare verde a ogni milestone)

```powershell
cargo test --locked --manifest-path poc/m5_executor/keygen/Cargo.toml          # oggi 2/2
cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml          # oggi 41/41
cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml --test lfs_receipt     # oggi 9/9
cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml --test m_g1b_pipeline  # oggi 10/10
cargo test --locked --manifest-path poc/m5_executor/executor-host/Cargo.toml   # oggi 27/27
cargo test --locked --manifest-path poc/m1/Cargo.toml                          # pattern redb M1
```

### 9.2 Nuovi test attesi (nomi proposti, non esistono ancora)

| Milestone | Comando | Esito richiesto |
| --- | --- | --- |
| M-LFS-1 | `cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml --test lfs_receipt` con backend memory dietro trait | 9/9 invariati |
| M-LFS-2 | `cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml --test lfs_durable` | tutti pass: recover, retry x3, conflitto 13, AI-only 0 byte, quota 11 |
| M-LFS-2 crash | `cargo run --release --locked --manifest-path poc/m5_executor/policy/Cargo.toml --features fault-fixtures --bin lfs-crash-gate -- 200 .local/lfs-crash.json` (modellato su `poc/m1` `crash-gate`) | 0 receipt "OK" senza record CRC-valido; 0 doppio effetto |
| M-LFS-3 | `cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml --test lfs_epoch` | receipt sessione 1 leggibile in sessione 2; epoch bump senza collisioni |
| M-CLK-1 | `cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml --test clock_ttl` | `EXPIRED` deterministico a `not_after + 1 s`; clock all'indietro -> rifiuto; zero frame |
| M-CLK-1 grep | `Select-String -Path poc/m5_executor/policy/src/*.rs -Pattern 'SystemTime::now'` | solo dentro l'impl `SystemClock` |

### 9.3 QEMU (M-LFS-4, M-CLK-3)

```powershell
# bundle host (seed di test, gitignored)
cargo run --locked --manifest-path poc/m5_executor/policy/Cargo.toml --example e2e_bundle -- `
  <pcb.seed> <lfs.seed> poc\m5_executor\app\generated
# regressione E2E attuale: 33 marker, nessun panicked/FAULT, uscita 124
cargo run --locked --manifest-path poc/m5_executor/keygen/Cargo.toml --bin verify-bridge-e2e -- `
  <ctx>/poc/m5_executor <pcb.seed> <lfs.seed> .local\m5-verify-bridge-e2e.log --wsl
# regressione classica
cargo run --locked --manifest-path poc/m5_executor/keygen/Cargo.toml --bin verify-bridge -- <args come in docs/VERIFICATION.md>
```

Criteri QEMU nuovi (runner esteso o `verify-bridge-e2e-durable`, da creare in M-LFS-4):

- 33 marker storici **+** marker nuovi: `LFS ledger reopened seq=<n>` dopo secondo boot con stesso store; `grant expired -> no commit` (M-CLK-3).
- Due boot consecutivi sullo stesso file/blocco: receipt seq 1 del boot 1 verificata dal PCB nel boot 2.
- Log archiviato in `docs/reports/m5-verify-bridge-e2e-durable-<data>.log`.

## 10. Note power-loss (riassunto operativo)

1. Ordine di scrittura: record (header + body + firma + CRC) -> `fsync(file)` -> `fsync(dir)` alla creazione -> **solo dopo** reply receipt al PCB.
2. A open: scansione fino all'ultimo record con CRC e firma validi; coda troncata scartata e loggata.
3. Request in volo al crash: nessuna receipt emessa -> il PCB la vede come non persistita; al riavvio, retry stesso `(epoch, request_id)` -> o receipt esistente (se il record era integro) o nuova esecuzione, mai entrambe.
4. Su QEMU il power-loss si simula con kill del processo QEMU durante la fase probe; su hardware serve misura con cache disco disattivata (fuori scope finché non c'è board).