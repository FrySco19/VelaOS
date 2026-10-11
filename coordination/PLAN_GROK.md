# Piano di coordinamento Grok Bot — Gate 0/1

Base: `47bb466` (`codex/lfs-neural-m1`). Coordinatore: Grok Bot (architettura comune, contratti, integrazione, decisioni finali).

## Regole vincolanti

- **Nessuna autorità all'LLM**: prompt, intenti e output LLM non concedono mai permessi; possono solo proporre piani mostrati all'utente.
- **Nessun push, merge su `main`, tag o release senza il proprietario** (Francesco). Lavoro solo su branch `grok/*` in worktree dedicati.
- Non toccare `components/`, `build/`, `archive/` (distro Debian/KDE storica) né i worktree esistenti (`VelaOS-m3-m4-design`, `VelaOS-runtime`, pool kilo).
- I contratti in `docs/CONTRACTS_v0.md` sono congelati: modifiche solo via ADR approvato dal coordinatore.
- Ogni affermazione distingue eseguito / documentato / non verificato. Niente "completo", "sicuro", "real-time" senza misure.

## I cinque agenti

| Agente | Branch / worktree previsto | Scope | Fuori scope | Dipende da |
| --- | --- | --- | --- | --- |
| **A1 UX** | `grok/ux-shell` | modello d'interazione, superfici effimere, preview piano, provenienza, consenso, undo; prototipo collegato a mock dell'envelope v0; accessibilità e test di usabilità riproducibili | firma/verifica, driver | C1, C5 (per mostrare stati/errori) |
| **A2 seL4/piattaforma** | `grok/sel4-platform` | `executor.system`, boot riproducibile, epoch di sessione, clock fidato, supervisore, M5 in CI, rimozione seed dall'ELF | policy Biscuit, UI | C2, C6 |
| **A3 PCB/authority** | `grok/pcb-biscuit` | PDP host-side (std) Biscuit attenuato contro root pinned → `signed_frame` esistente; poi admission v0, revoca generazionale; owner di `BRIDGE_CONTRACT.md` | runtime Wasm, LFS storage | C1, C3, crate `biscuit-auth =6.0.0` già usato in M1/M2 |
| **A4 Executor/LFS** | `grok/executor-lfs` | (inizialmente nessuna modifica a `bridge.rs`/`main.rs`) poi verifica admission v0, quote, handle opachi LFS, writer risultato + receipt `(epoch, request_id)` | policy Datalog | C1, C2, C4; output di A3 |
| **A5 QA** | `grok/qa-gates` | test negativi, fuzzing parser frame/envelope, harness verify-bridge, matrice requisiti, misure con metodo dichiarato | implementazione feature | tutti i contratti |

Ordine: A3 e A5 partono subito; A4 parte sul mock dell'admission di A3; A2 in parallelo su CI/epoch; A1 in parallelo su mock.

## Prima milestone (bloccante, piccola, misurabile) — M-G1a

**Sostituire il token PCB solo-Ed25519 con un Biscuit attenuato + admission strutturata.**

Vincoli dal read-only inventory (2026-10-12): `biscuit-auth =6.0.0` è solo `std`; PCB/Executor Microkit sono `no_std`. Quindi in Gate 1 **non** si aggiunge `biscuit-auth` a `poc/m5_executor/app/Cargo.toml`.

Slice minima:
1. Nuovo **PDP host-side (std)** in `poc/m5_executor/policy/` (o test additivi in `keygen`), riusando i pattern Biscuit di M1/M2: verifica contro root pinned; attenuazione per operazione (`execute`), hash del modulo, quota, scadenza; fatti/check sconosciuti rifiutati.
2. Se consentito → il PDP chiama il **`signed_frame` esistente** (wire v2 invariato); se negato → **non firma mai** e restituisce `POLICY_DENIED`.
3. Root Biscuit (`ed25519-dalek` via biscuit-auth) ≠ chiave di firma PCB (`ed25519-compact`): due trust anchor distinti, documentati in C3.
4. L'Executor resta verificatore della sola firma Ed25519 di admission; Biscuit in-guest rinviato. Admission strutturata v0 sul filo rinviata a Gate 1b.
5. Test m5 esistenti verdi (`keygen` 2/2), m1/m2 senza regressioni, **marker di `verify-bridge` invariati**.
6. Nuovo test host: **Biscuit fuori scope rifiutato con `POLICY_DENIED`, nessun frame firmato e nessuna esecuzione Wasm**.
7. Nessuna build QEMU lunga per chiudere M-G1a.

Ownership dei file per questa slice:
- **A3 PCB/Authority**: PDP host (`poc/m5_executor/policy/` o test in `keygen`) e aggiornamenti a `BRIDGE_CONTRACT.md`.
- **A4 Executor**: non modifica `app/src/bridge.rs` né `app/src/main.rs` inizialmente.
- **A5 QA**: test negativi e harness; **A2 seL4**: CI M5 senza toccare il wire.

## Contratti da congelare per primi

C1 envelope · C3 livelli di autorità · C4 chiave di idempotenza · C5 codici errore (C2 e C6 già allineati a M5).

## Decisioni che spettano al proprietario

- Scelta della board fisica (Gate 3).
- Politica di consenso per invio dati in rete e telemetria.
- Gestione chiavi di produzione (provisioning, rotazione, recovery).
- Quando e se pubblicare (push/PR/release).
