# ADR-LFS-1 / ADR-CLK-1 — LFS durevole e clock fidato per il TTL dei grant

Stato: **accettato per il path host** (branch `vela/lfs-clock`, base `f0942be`).
Piano di riferimento: `docs/PLAN_LFS_DURABLE_AND_CLOCK.md` su `origin/vela/release-prep`.
Copre M-LFS-1, M-LFS-2, M-LFS-3, M-CLK-1 e **M-CLK-3 (clock fidato nel guest QEMU)**. **Non** copre M-LFS-4/5 né M-CLK-2 (vedi §5).

## 1. Scelta di archivio: Opzione A (append-only + fsync)

- Store: `poc/m5_executor/policy/src/durable.rs` → `FileLedger` (host `std`).
- redb **non** usato: nessun blocco incontrato; Option A è auditabile e
  trasportabile su virtio-blk per M-LFS-4.
- Seam comune `LedgerStore`: `MemoryStore` (= `Ledger<64>`, stesso codice
  `no_std` del guest) e `FileLedger`. `SimulatedLfs<S = MemoryStore>` e
  `E2eHost<S = MemoryStore>` mantengono l'API precedente (test `lfs_receipt` 9/9 invariati).
- Divergenza capacità: guest `Ledger<8>`, host memoria 64, `FileLedger` capacità
  esplicita all'apertura. Superata → `QUOTA_EXCEEDED` (11).

### Formato su disco (`ledger.vlfs`)

| Parte | Byte | Contenuto |
| --- | --- | --- |
| Header | 52 | `VLFSLOG1` · version u32 = 1 · chiave pubblica LFS pinnata (32) · checksum BLAKE3/8 |
| Record | 140 | `VREC` · body `VelaOS.LFS.Receipt.v0` (64: epoch, request_id, status, value, authority_digest, seq) · firma Ed25519 (64) · checksum BLAKE3/8 |

Il record contiene **esattamente** body + firma del wire receipt: nessun cambio a
`ledger_wire.rs`, label 0x711/0x712 o wire v2.

### Ordine di scrittura e recovery (power-loss)

1. valida (C4.4: frame autenticato sotto chiave PCB, request_id, authority digest; status = 0)
2. idempotenza `(epoch, request_id)`: stesso contenuto → receipt memorizzata (`Replayed`), diverso → `IDEMPOTENCY_CONFLICT` (13)
3. firma deterministica (Ed25519 senza noise) con `seq = n + 1`
4. `write_all` → `sync_data` (fsync) → **solo dopo** la receipt torna al PCB
5. Errore I/O → store avvelenato: ogni commit successivo rifiutato (11) fino a riapertura + recovery

All'apertura: si accetta il prefisso contiguo di record con magic, checksum,
firma valida sotto la chiave LFS pinnata e `seq` consecutivo. Coda strappata →
troncata e fsync (`Recovery.truncated_bytes`). Record danneggiato **seguito** da
un record integro → **rifiuto di aprire** (non è una coda strappata; non si
scartano receipt in silenzio). Header di un'altra chiave LFS → rifiuto.

Su Windows `std` non può fare fsync di una directory: la creazione/rename si
affida al journaling NTFS; su Unix si fa `fsync(dir)`.

## 2. Epoch persistente (M-LFS-3)

`EpochStore` (`epoch.bin`: `VEPOCH01` · epoch u64 · flag clean · checksum),
scrittura atomica tmp → fsync → rename.

**Decisione (deviazione motivata dal piano):** *ogni* boot ottiene una nuova
epoch, anche dopo shutdown pulito. Motivo: il contatore `request_id` del PDP è
in memoria e riparte da 1; riusare l'epoch causerebbe collisioni
`(epoch, request_id)` con la sessione precedente. Il flag clean resta e
distingue `Boot::Clean` da `Boot::Recovered` (richieste in volo della sessione
precedente = **AMBIGUOUS** finché non risolte con `E2eHost::resolve_in(epoch_prec, id)`,
che legge notary o ledger durevole verificando la firma con la chiave LFS pinnata).
Una nuova epoch invalida anche i grant della sessione precedente (OWN-v3).
File epoch corrotto → rifiuto di avviare (fail-closed).

## 3. Clock fidato (M-CLK-1)

`poc/m5_executor/policy/src/clock.rs`:

- `Clock` (wall + monotonic), `SystemClock` (unico punto con `SystemTime::now` /
  `Instant::now`; verificato da test), `FakeClock` (wall e monotonic separabili,
  monotonic rimovibile).
- `TrustedTime`: wall con high-water mark → orologio che torna indietro =
  `ClockError::Regressed` → `EXPIRED` (7), nessun frame firmato.
- `GrantDeadline`: il TTL del grant (`not_after - now` al conio) viene ancorato
  sul clock **monotono**. Controllato **prima** di eseguire l'Executor e **prima**
  del commit LFS (`E2eHost::run_task`). Un salto del wall clock (avanti o indietro)
  non estende il grant. Monotono assente → `EXPIRED`, niente Wasm (fail-closed).
- Semantica inclusiva come Biscuit (`$t <= exp`): a `not_after` ancora valido, a `not_after + ε` scaduto.
- `run_task_at(.., now, ..)` resta per bundle riproducibili (nessuna deadline monotona).

### 3.2 Clock fidato **nel guest** (M-CLK-3)

`poc/m5_executor/app/src/guest_clock.rs` (`no_std`, condiviso col crate host via `#[path]`):

- Fonte: contatore fisico Arm generic timer `CNTPCT_EL0` + `CNTFRQ_EL0`, leggibili
  a EL0 perché il kernel seL4 dell'SDK Microkit 2.3.0 ha `CONFIG_EXPORT_PCNT_USER=1`
  (verificato in `gen_config.h` dell'immagine). È tempo hardware dal reset: non è
  il wall clock host, l'host non può modificarlo dopo il boot, non torna indietro.
- L'admission porta un TTL **relativo** (`1..=120` s, coniato dal PDP host dopo
  UserSelection). Il PCB guest lo ancora sul contatore quando pubblica il frame
  (`Deadline::start`) e, **prima** del commit LFS, verifica `remaining_ms`;
  scaduto → `EXPIRED` (7), nessuna chiamata al writer (`commit_if_alive`).
- Fail-closed: frequenza 0, TTL fuori `1..=120`, overflow, contatore sotto
  l'ancora o frequenza cambiata = scaduto.
- Composizione: host rifiuta di firmare dopo `not_after` (wall), guest rifiuta
  di persistere dopo `ttl` del proprio tempo monotono. Il tempo pre-boot non è
  misurabile dal guest: la finestra guest parte dalla pubblicazione (documentato).
- QEMU: seconda admission (request 13, TTL 1 s, nuova UserSelection) eseguita
  (Wasm 21), poi attesa sul contatore oltre la deadline → `EXPIRED`, nessun
  commit. Il runner verifica l'**assenza** di `LFS committed epoch=1 request=13`.

## 4. Evidenze (eseguito su Windows, toolchain 1.96.1, `--offline --locked`)

| Suite | Esito |
| --- | --- |
| policy totale | 69/69 (41 storici + clock_ttl 11 + guest_clock 3 + lfs_durable 11 + lfs_epoch 3) |
| lfs_receipt (regressione) | 9/9 |
| keygen / executor-host (incl. R1 MAX_FUEL 7) | 2/2 · 27/27 |
| intent / shell / glass / m0-recall / m1 | 12/12 · 13/13 · 10/10 (+1 live ignorato) · 6/6 (+2 live ignorati) · 6/6 |
| clippy policy `--all-targets --features fault-fixtures -D warnings` | pulito |
| `lfs-crash-gate 200` (abort reale del processo in 4 fasi) — `docs/reports/lfs-crash-gate-2026-10-12.json` | PASS: 0 receipt annunciate perse, 0 doppi effetti, 600/600 record, 3500 byte strappati troncati |
| QEMU `verify-bridge-e2e` (WSL Docker, con clock guest) | PASS **37** marker (33 storici incl. POLICY_DENIED e MAX_FUEL + 4 CLK), nessun panic/FAULT, exit 124, marker proibiti assenti — `docs/reports/m5-verify-bridge-e2e-lfs-clock-2026-10-12.log` |

Comandi:

```powershell
cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml
cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml --test lfs_durable
cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml --test lfs_epoch
cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml --test clock_ttl
cargo run --release --locked --manifest-path poc/m5_executor/policy/Cargo.toml --features fault-fixtures --bin lfs-crash-gate -- 200 .local/lfs-crash.json
```

## 5. Cosa NON è dimostrato (onesto)

- **Guest QEMU**: il dominio `velaos-lfs` usa ancora il ledger in memoria
  `Ledger<8>`; nessuna persistenza cross-reboot in guest (M-LFS-4 aperto).
- **TTL nel guest**: enforced dal PCB guest sul contatore hardware (M-CLK-3), ma
  il writer LFS guest non ricontrolla la deadline (fida il PCB, stesso modello
  del notary); il tempo trascorso **prima del boot** non è visibile al guest
  (coperto solo dal controllo host al conio). Nessuna fonte RTC/TPM per tempo
  assoluto cross-reboot.
- `verify-bridge` classico non rieseguito separatamente: i suoi 14 marker sono
  inclusi e verificati nel run E2E; il path non-e2e del PCB non compila `guest_clock`.
- Power-loss reale con cache disco: simulato solo con abort di processo (i dati
  scritti ma non fsyncati sopravvivono nella cache OS); serve hardware/board.
- Notary PCB ancora in memoria (risoluzione cross-boot via ledger durevole + firma).
- `result_handle` / `expected_version` sul write path (M-LFS-5) aperto.
- Seed PCB/LFS di test, non trust root di produzione.
