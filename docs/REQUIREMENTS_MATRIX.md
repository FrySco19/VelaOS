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
| G0-05 | Boot QEMU bridge M5 riproducibile (`verify-bridge`) | implemented (QEMU) | `verify-bridge` rieseguito 2026-10-12 su `grok/sel4-platform`: PASS, log `docs/reports/m5-verify-bridge-2026-10-12-sel4-platform.log`; `docs/SEL4_GATE1_SLICE.md` §2 | seL4 |
| G0-06 | Contratti v0 congelati (envelope, ownership, autorità, receipt, errori, target) | documented | `docs/CONTRACTS_v0.md` | PCB |
| G0-07 | M5 nella CI (almeno test host keygen) | unverified | nessun workflow M5 | QA |
| G0-08 | Target di riferimento dichiarato (QEMU virt AArch64) | implemented (config check) | `docs/CONTRACTS_v0.md` §C6, `docs/PLATFORM_SEL4.md` §2; pin verificati da `poc/m5_executor/check-pins.py` (31/31 oggi) + `.github/workflows/m5-platform-pins.yml` (non ancora eseguito su GitHub) | seL4 |

## Gate 1 — Vertical slice autorizzata

| id | requisito | stato | evidenza | owner |
| --- | --- | --- | --- | --- |
| G1-01 | UI mostra piano/intento prima dell'esecuzione | unverified | — | UX |
| G1-02 | Utente emette Biscuit attenuato per operazione/oggetto/durata | prototype | `poc/m1`, `poc/m2/src/authority.rs` (host) | PCB |
| G1-03 | PDP host (std) verifica Biscuit contro root pinned e rifiuta fuori scope; firma solo se consentito | implemented (host; in-guest rinviato) | `poc/m5_executor/policy` - 12/12 test host (M-G1a) | PCB |
| G1-04 | PCB produce admission strutturata firmata (envelope v0) | documented | `docs/CONTRACTS_v0.md` §C1 | PCB |
| G1-05 | Executor verifica firma admission, binding byte esatti, quote | implemented (host) / prototype (QEMU) | `poc/m5_executor/EXECUTOR_CONTRACT.md` E2/E3; `executor-host/tests/contract.rs` (`e2_*`, `e3_*`), keygen 2/2; QEMU: `PHASE_2_3_REPORT.md` (non rieseguito) | Executor |
| G1-06 | Wasm senza import ambientali, risultato bounded | implemented (host) / prototype (QEMU) | `EXECUTOR_CONTRACT.md` E4/E5; `executor-host/tests/fixtures.rs` (import env/memoria/WASI rifiutati prima dell'istanziazione); risultato = **solo u32** in `regs[1]` (limite UX, E5) | Executor |
| G1-07 | Executor legge input LFS solo via handle opaco | documented | `BRIDGE_CONTRACT.md`; `EXECUTOR_CONTRACT.md` E4 (oggi nessun accesso LFS né handle: solo payload firmato) | Executor |
| G1-08 | LFS persiste risultato + receipt idempotente `(epoch, request_id)` | documented | `docs/CONTRACTS_v0.md` §C4; `EXECUTOR_CONTRACT.md` "Gap to envelope v0" (nessun receipt, risultato non persistito) | Executor |
| G1-09 | Output non autorizzato non viene persistito (test negativo) | unverified | — | QA |
| G1-10 | UI mostra autorizzazione, risultato, provenienza, undo/compensazione | unverified | — | UX |
| G1-11 | Test host: Biscuit fuori scope rifiutato senza esecuzione Wasm | implemented (host) | `poc/m5_executor/policy/tests/admit.rs` (deny = nessun frame firmato; il PDP non linka runtime Wasm); CI host: `.github/workflows/m5-policy.yml` | PCB |

## Gate 2 — Robustezza e sicurezza

| id | requisito | stato | evidenza | owner |
| --- | --- | --- | --- | --- |
| G2-01 | Replay entro sessione rifiutato | implemented (host) / prototype (QEMU) | `EXECUTOR_CONTRACT.md` E7; `executor-host/tests/contract.rs` `e7_replay_rules_within_session`; `guest_parity.rs` rigioca la sequenza di `pcb.rs`; QEMU: `PHASE_2_3_REPORT.md` | Executor |
| G2-02 | Replay attraverso reboot (epoch persistente) | documented | `BRIDGE_CONTRACT.md`; design supervisore/epoch OWN-v3 in `docs/SEL4_GATE1_SLICE.md` §3 (nessuno storage in guest) | seL4 |
| G2-03 | Expiry con clock fidato | documented | `BRIDGE_CONTRACT.md` | seL4 |
| G2-04 | Revoca e cancellazione | prototype | revoca handle host in `poc/m2/src/authority.rs`; nessuna nel bridge | PCB |
| G2-05 | Crash prima/durante/dopo esecuzione e commit | prototype | crash-gate M1 (processi host); nulla su seL4 | QA |
| G2-06 | Esito ambiguo gestito via receipt | documented | `docs/CONTRACTS_v0.md` §C2 OWN-v3, §C5; `EXECUTOR_CONTRACT.md` E6/E8 (R1 chiuso: esaurimento fuel = status 6, non fault; altri fault restano senza risposta) | Executor |
| G2-07 | Rifiuto input malformati (bounds, offset, firma) | implemented (host) / prototype (QEMU) | keygen 2/2; `executor-host/tests/contract.rs` (`e2_*`, `e6_bad_call_shape_is_malformed`, `e9_payload_size_bound`), `fixtures.rs` `oversized_module_fails_closed`; QEMU non rieseguito | Executor |
| G2-08 | Fuzzing parser frame/envelope | unverified | — | QA |
| G2-09 | Aggiornamento atomico, rollback, trust root | unverified | — | seL4 |
| G2-10 | Seed di test rimosso dall'ELF PCB; provisioning chiavi | documented | `poc/m5_executor/README.md` (limite noto) | seL4 |
| G2-11 | Task a `MAX_FUEL` (loop infinito): niente overflow dello stack nativo, status 6 via IPC, Executor continua a servire (R1) | implemented (host + QEMU) | Wasmi `portable-dispatch` in `app/Cargo.toml`; `executor-host/tests/r1_max_fuel.rs` 7/7 (dev e `--release`; controllo negativo senza feature = STATUS_STACK_OVERFLOW); QEMU `verify-bridge` PASS 2026-10-12, richieste 9-10, log `docs/reports/m5-verify-bridge-2026-10-12-r1-max-fuel.log` | Executor |

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

## Note Executor (EXEC-v2.1)

- Contratto: `poc/m5_executor/EXECUTOR_CONTRACT.md`; test host: `cargo test --locked --manifest-path poc/m5_executor/executor-host/Cargo.toml` (contract 13, fixtures 5, guest_parity 2, r1_max_fuel 7).
- Limiti aperti: risultato solo u32; replay solo per boot (test `e7_known_gap_replay_state_is_boot_local`); nessun clock/deadline/revoca nell'Executor; R1 **chiuso** (2026-10-12, `grok/r1-wasm-fuel`): `portable-dispatch`, loop a `MAX_FUEL` = status 6 in QEMU (G2-11); residuo: tempo di interpretazione a `MAX_FUEL` su hardware reale non misurato.
