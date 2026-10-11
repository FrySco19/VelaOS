# Contratti d'integrazione v0 (bozza congelata) / Integration contracts v0 (frozen draft)

Stato: **DRAFT-FROZEN v0** · Base: `47bb466` · Data: 2026-10-12.
"Congelato" significa: nessun agente cambia questi campi/semantiche senza un ADR approvato dal coordinatore; aggiunte retro-compatibili vanno in v0.x, rotture in v1. Il codice attuale **non** implementa ancora tutto ciò che è descritto; la colonna "oggi" indica lo stato reale.

Principio non negoziabile: **nessun prompt, intento o output LLM concede autorità.** L'LLM può solo proporre un piano; autorità = token verificato + admission PCB + capability kernel configurate.

---

## C1 — Envelope richiesta/risposta (`VelaEnvelope v0`)

Nota Gate 1: il **wire v2** (`VBRG`/`VEX2`, 20+32+64 B) e i marker di `verify-bridge` restano **invariati** nella prima slice. L'envelope v0 sotto è il contratto di destinazione (Gate 1b/2); i campi non presenti nel frame v2 vengono vincolati dal PDP host prima della firma, non ancora trasportati sul filo.

Codifica: binaria little-endian a layout fisso nel data plane (pagina condivisa), canonicalizzata per firma. Digest: BLAKE3-256 salvo diversa indicazione.

### Richiesta

| Campo | Tipo | Semantica | Oggi (frame v2) |
| --- | --- | --- | --- |
| `magic` | `[u8;4]` = `VENV` | identifica envelope v0 | `VBRG` / `VEX2` |
| `protocol_version` | u16 | versione envelope (v0 = `0x0100`, distinta dal frame legacy 2) | 2 |
| `header_len` | u16 | lunghezza header, validata | 20 |
| `epoch` | u64 | epoch di sessione autenticata Executor/PCB; cambia a ogni boot/ripristino | assente |
| `request_id` | u64 | unico e monotono entro `epoch`; mai riusato, mai wrap | u32, boot-local |
| `audience` | u32 | dominio destinatario (es. 1 = Executor) | claim op=1 |
| `ops` | bitset u32 | operazioni ammesse (EXECUTE=1, LFS_READ=2, LFS_WRITE_RESULT=4) | solo execute |
| `payload_len` | u32 | lunghezza esatta dei byte eseguibili/dati | sì |
| `payload_digest` | `[u8;32]` | BLAKE3 del payload esatto | firma diretta sui byte |
| `authority_digest` | `[u8;32]` | BLAKE3 del token Biscuit serializzato verificato dal PCB | assente |
| `objects` | lista ≤ 8 × (`handle:[u8;32]`, `expected_version:u64`) | handle opachi LFS in input + versione attesa | assente |
| `result_handle` | `[u8;32]` | handle opaco dove LFS può scrivere il risultato | assente |
| `quota.fuel` | u64 | 1..=10 000 000 | sì |
| `quota.memory` | u32 | 1..=65 536 B | sì |
| `quota.output_bytes` | u32 | massimo byte di risultato | assente |
| `quota.deadline_ms` | u32 | deadline wall-clock (richiede clock fidato) | assente |
| `not_after` | u64 | scadenza (ms da epoch fidato) | assente |
| `revocation_gen` | u64 | generazione di revoca vista all'admission; Executor rifiuta se < generazione corrente | assente |
| `admission_sig` | `[u8;64]` | Ed25519 del PCB su dominio `VelaOS.Admission.v0\0` ‖ tutti i campi sopra ‖ payload | Ed25519 su `VelaOS.Executor.SignedFrame.v2\0` |

### Risposta

Registri IPC (label `0x702`) restano: `status`, `result_u32`, `request_id_lo`, `attempts`. In v0 si aggiunge un blocco risposta tipizzato (pagina di ritorno o regione dedicata) con: `epoch`, `request_id`, `status`, `result_digest`, `result_len`, `fuel_used`, `receipt_hint`. Una risposta senza `request_id` fidato (malformed) non può essere correlata che tramite single-flight.

---

## C2 — Ownership pagina condivisa e ciclo di ack

Allineato a `poc/m5_executor/BRIDGE_CONTRACT.md`.

| Versione | Stato | Regole |
| --- | --- | --- |
| **OWN-v1** | storico (Phase 1, `278a0df`; dettagli da ricostruire dalla storia git) | consegna basata su notifica + pagina; superato da v2 |
| **OWN-v2** | **implementato e verificato in QEMU** (non rieseguito oggi) | PCB unico writer (RW), Executor RO, nessuna mappatura eseguibile; un solo chiamante, max 1 richiesta in volo; PCB pubblica tutti i byte, poi protected call `0x701`; nessuna mutazione fino alla risposta `0x702` (4 registri); Executor fa snapshot privato prima di parse/verify ed esegue solo lo snapshot; risposta (successo o errore) = ack che autorizza il riuso; ID autenticati consumati prima dell'istanziazione; firme invalide non consumano ID |
| **OWN-v3** | **documentato (target Gate 1/2)** | come v2 + `epoch` di sessione; se Executor crasha o non risponde, PCB **non** assume ack, **non** sovrascrive la pagina, **non** riesegue task con effetti; il supervisore dichiara nuova epoch, l'esito della vecchia richiesta si risolve solo dal receipt LFS (C4) o resta `AMBIGUOUS` |
| OWN-v4 | futuro | ingresso asincrono: coda bounded con sequence number e cursore consumatore ack separato; le notifiche sono solo hint |

Regola invariata: le notifiche seL4 non sono mai il canale d'ingresso dei task.

---

## C3 — Livelli di autorità (non intercambiabili)

| Livello | Chi lo emette | Chi lo verifica | Cosa prova | Cosa NON fa |
| --- | --- | --- | --- | --- |
| **L1 Token Biscuit** (attenuabile, Datalog) | issuer utente/servizio con root fidata | **PCB** (policy decision point) contro root pinned; rifiuta fatti/check sconosciuti | chi può chiedere quale operazione su quali oggetti, con quali vincoli | non è una capability kernel; non arriva all'Executor in v0 (solo `authority_digest`) |
| **L2 Firma di admission PCB** (Ed25519) | PCB | **Executor** con chiave PCB pinned | che il PCB ha ammesso *esattamente* questa envelope (payload, ops, oggetti, quote, scadenza, revoca) | non concede capability nuove; non sostituisce la verifica Biscuit |
| **L3 Capability seL4** | configurazione di sistema (`executor.system`, Microkit) a build time | kernel | quali endpoint, pagine e canali un dominio può toccare | non è derivabile da token o firme a runtime |

Chiavi distinte (da non confondere): la **root Biscuit** usa Ed25519 via `ed25519-dalek`/`biscuit-auth` (pattern M1/M2, host `std`); la **chiave di firma admission PCB** usa `ed25519-compact` (`no_std`, M5). Sono chiavi e trust anchor diversi: compromettere/ruotare l'una non implica l'altra.

Vincolo di piattaforma: `biscuit-auth =6.0.0` è solo `std`; i domini Microkit PCB/Executor sono `no_std`. In Gate 1 **non** si aggiunge `biscuit-auth` a `poc/m5_executor/app/Cargo.toml`: la verifica Biscuit avviene in un **PDP host-side (std)** che, solo in caso di consenso, chiama il `signed_frame` esistente; in caso di rifiuto **non firma mai**. L'Executor resta verificatore della sola firma Ed25519 di admission; Biscuit in-guest è rinviato.

Regole: un token fornito non può aggiungere capability kernel; l'Executor non interpreta path di filesystem né ottiene accesso ambientale; nessun LLM firma, trasporta byte eseguibili o decide autorizzazioni.

---

## C4 — Receipt LFS e chiavi di idempotenza

- Chiave di idempotenza: **`(epoch: u64, request_id: u64)`**, unica globalmente insieme all'identità PCB (`pcb_key_id`).
- Record receipt (persistito atomicamente con il risultato, prima di annunciare completamento):
  `{ pcb_key_id, epoch, request_id, payload_digest, authority_digest, result_handle, result_digest, committed_version, status, committed_at }`.
- Semantica:
  1. Writer LFS verifica autorità esatta su `result_handle` e `LFS_WRITE_RESULT`, e `expected_version`.
  2. Se esiste già un receipt per la chiave: restituisce l'esito registrato, **non** riapplica effetti.
  3. Stessa chiave con `payload_digest` o `authority_digest` diversi → `CONFLICT`, nessuna scrittura.
  4. Output non autorizzato (handle/op fuori scope) → rifiutato e **non persistito**.
- Oggi: nessun receipt implementato; il risultato 7 arriva solo nei registri PCB.

---

## C5 — Codici di errore e recovery (bozza)

Compatibili con gli stati attuali 0–6 del frame v2; nuovi codici ≥ 7.

| Codice | Nome | Origine | Recovery |
| --- | --- | --- | --- |
| 0 | `OK` | Executor | — |
| 1 | `MALFORMED` | Executor | non ritentare; correggere il produttore |
| 2 | `AUTH` | Executor (firma admission) | non consuma ID; rigenerare admission |
| 3 | `SCOPE` | Executor/PCB | rifiuto definitivo per quella envelope |
| 4 | `REPLAY` | Executor | risolvere via receipt (C4) |
| 5 | `WASM_REJECT_OR_TRAP` | Executor | definitivo; ID consumato |
| 6 | `FUEL_EXHAUSTED` | Executor | definitivo; ID consumato |
| 7 | `EXPIRED` | PCB/Executor (clock fidato) | nuova admission |
| 8 | `REVOKED` | PCB/Executor | definitivo |
| 9 | `CANCELLED` | PCB/supervisore | definitivo; compensazione se applicabile |
| 10 | `POLICY_DENIED` | PCB (Biscuit) | nessuna Wasm eseguita |
| 11 | `QUOTA_EXCEEDED` | PCB/Executor | ridurre quota o nuova autorizzazione |
| 12 | `VERSION_CONFLICT` | LFS writer | rileggere e ripianificare |
| 13 | `IDEMPOTENCY_CONFLICT` | LFS writer | definitivo |
| 14 | `AMBIGUOUS` | PCB dopo crash/timeout | non ripetere effetti; interrogare receipt |
| 15 | `UNAVAILABLE` | supervisore | retry dopo nuova epoch |

---

## C6 — Target di riferimento dei gate

- Riferimento Gate 0–2: **QEMU `virt` AArch64**, seL4 via **Microkit 2.3.0**, rust-sel4 @ `d441a88`, toolchain `nightly-2025-10-20`, Executor **Wasmi 2.0.0 `no_std`**.
- Host test: Rust 1.96.1 (CI M1/M2).
- **Nessuna affermazione hardware.** Una board fisica sarà scelta e dichiarata in Gate 3 (decisione del proprietario). Cifre QEMU non sono latenza né consumo RAM reale.
