# Matrice dei requisiti Gate 0–4 / Requirements matrix

Base: `47bb466` · Data: 2026-10-12. Stati: `implemented` (codice + test eseguiti o CI) · `prototype` (PoC, evidenza documentata/parziale, es. QEMU non rieseguito oggi) · `documented` (solo specifica) · `unverified` (nessun codice né prova).
Agenti: **UX**, **seL4** (boot/piattaforma), **PCB** (authority/policy), **Executor** (runtime Wasm + LFS bridge), **QA** (test, fuzzing, misure).

## Gate 0 — Baseline e contratti

| id | requisito | stato | evidenza | owner |
| --- | --- | --- | --- | --- |
| G0-01 | Inventario as-built con distinzione eseguito/documentato | implemented | `docs/GATE0_BASELINE.md` | QA |
| G0-02 | Test host M1 verdi | implemented | `poc/m1` — 6/6 oggi | QA |
| G0-03 | Test host M2 verdi | implemented | `poc/m2` — 7/7 oggi | QA |
| G0-04 | Test host M5 keygen (descriptor/crypto) verdi | implemented | `poc/m5_executor/keygen` — 2/2 oggi | QA |
| G0-05 | Boot QEMU bridge M5 riproducibile (`verify-bridge`) | prototype | `poc/m5_executor/PHASE_2_3_REPORT.md`, `phase-2-3-boot.log` (non rieseguito oggi) | seL4 |
| G0-06 | Contratti v0 congelati (envelope, ownership, autorità, receipt, errori, target) | documented | `docs/CONTRACTS_v0.md` | PCB |
| G0-07 | M5 nella CI (almeno test host keygen) | unverified | nessun workflow M5 | QA |
| G0-08 | Target di riferimento dichiarato (QEMU virt AArch64) | documented | `docs/CONTRACTS_v0.md` §C6 | seL4 |

## Gate 1 — Vertical slice autorizzata

| id | requisito | stato | evidenza | owner |
| --- | --- | --- | --- | --- |
| G1-01 | UI mostra piano/intento prima dell'esecuzione | unverified | — | UX |
| G1-02 | Utente emette Biscuit attenuato per operazione/oggetto/durata | prototype | `poc/m1`, `poc/m2/src/authority.rs` (host) | PCB |
| G1-03 | PDP host (std) verifica Biscuit contro root pinned e rifiuta fuori scope; firma solo se consentito | documented | `BRIDGE_CONTRACT.md` | PCB |
| G1-04 | PCB produce admission strutturata firmata (envelope v0) | documented | `docs/CONTRACTS_v0.md` §C1 | PCB |
| G1-05 | Executor verifica firma admission, binding byte esatti, quote | prototype | `app/src/bridge.rs` (frame v2, Ed25519) | Executor |
| G1-06 | Wasm senza import ambientali, risultato bounded | prototype | `PHASE_2_3_REPORT.md` (risultato 7, WASI rifiutato) | Executor |
| G1-07 | Executor legge input LFS solo via handle opaco | documented | `BRIDGE_CONTRACT.md` | Executor |
| G1-08 | LFS persiste risultato + receipt idempotente `(epoch, request_id)` | documented | `docs/CONTRACTS_v0.md` §C4 | Executor |
| G1-09 | Output non autorizzato non viene persistito (test negativo) | unverified | — | QA |
| G1-10 | UI mostra autorizzazione, risultato, provenienza, undo/compensazione | unverified | — | UX |
| G1-11 | Test host: Biscuit fuori scope rifiutato senza esecuzione Wasm | unverified | prima milestone, `coordination/PLAN_GROK.md` | PCB |

## Gate 2 — Robustezza e sicurezza

| id | requisito | stato | evidenza | owner |
| --- | --- | --- | --- | --- |
| G2-01 | Replay entro sessione rifiutato | prototype | `PHASE_2_3_REPORT.md` (replay → status 4) | Executor |
| G2-02 | Replay attraverso reboot (epoch persistente) | documented | `BRIDGE_CONTRACT.md` | seL4 |
| G2-03 | Expiry con clock fidato | documented | `BRIDGE_CONTRACT.md` | seL4 |
| G2-04 | Revoca e cancellazione | prototype | revoca handle host in `poc/m2/src/authority.rs`; nessuna nel bridge | PCB |
| G2-05 | Crash prima/durante/dopo esecuzione e commit | prototype | crash-gate M1 (processi host); nulla su seL4 | QA |
| G2-06 | Esito ambiguo gestito via receipt | documented | `docs/CONTRACTS_v0.md` §C2 OWN-v3, §C5 | Executor |
| G2-07 | Rifiuto input malformati (bounds, offset, firma) | prototype | keygen test + QEMU | Executor |
| G2-08 | Fuzzing parser frame/envelope | unverified | — | QA |
| G2-09 | Aggiornamento atomico, rollback, trust root | unverified | — | seL4 |
| G2-10 | Seed di test rimosso dall'ELF PCB; provisioning chiavi | documented | `poc/m5_executor/README.md` (limite noto) | seL4 |

## Gate 3 — Sistema usabile

| id | requisito | stato | evidenza | owner |
| --- | --- | --- | --- | --- |
| G3-01 | Shell base: input, output, workspace effimeri, diagnostica | unverified | — | UX |
| G3-02 | Board fisica scelta e dichiarata | unverified | decisione del proprietario | seL4 |
| G3-03 | Driver minimi display/input/storage/clock | unverified | — | seL4 |
| G3-04 | Setup, recovery, provisioning, aggiornamenti verificati | unverified | — | seL4 |
| G3-05 | Accessibilità (tastiera, screen reader), localizzazione IT/EN | unverified | — | UX |
| G3-06 | Protezione dati locali, offline, nessun invio senza consenso | prototype | LFS cifrato host (`poc/m1`) | PCB |
| G3-07 | Documentazione utente/sviluppatore e installazione riproducibile | prototype | `poc/m5_executor/README.md` (solo bridge) | QA |

## Gate 4 — Qualificazione

| id | requisito | stato | evidenza | owner |
| --- | --- | --- | --- | --- |
| G4-01 | Benchmark end-to-end su hardware identificato | unverified | — | QA |
| G4-02 | RAM idle < 1,5 GB (definizione + metodo + distribuzioni) | unverified | ipotesi in `docs/ROADMAP.md` | QA |
| G4-03 | Intent→execution < 250 ms (scomposto per fase) | unverified | ipotesi | QA |
| G4-04 | Threat model aggiornato + review indipendente | unverified | — | QA |
| G4-05 | Matrice compatibilità, limiti noti, recovery | prototype | `docs/KNOWN_ISSUES.md` (parziale, distro storica) | QA |
| G4-06 | Gate "ready for preview" separato da "ready for daily use" | documented | questo file | QA |
