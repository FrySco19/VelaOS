# Checklist release pubblica utilizzabile — VelaOS

Stato: **checklist di gate**, non certificazione di release.  
Base evidenze: `1c94bd1` (`grok/velaos-final-e2e`). Branch documenti: `grok/release-prep`.  
Autore: vela-bot · 2026-10-12.  
Coordinamento: Fairha decide tag/push/PR; questo file non autorizza release.

Legenda evidenze:
- **già dimostrato** = presente e verificato su `1c94bd1` (o log/report citati).
- **ancora aperto** = non soddisfa ancora un criterio "usabile pubblico".

---

## 1. Cosa possiamo affermare oggi (onesto)

| Affermazione | Verdetto | Evidenza |
| --- | --- | --- |
| Bridge PCB↔Executor in QEMU virt AArch64, fuel-safe (R1 status 6, dominio vivo) | **già dimostrato** | `poc/m5_executor/EXECUTOR_CONTRACT.md` EXEC-v2.1; `docs/reports/m5-verify-bridge-2026-10-12-r1-max-fuel.log`; E2E log sotto |
| Pipeline E2E: intent mock → UserSelection → Biscuit grant → frame 4 KiB → Wasm → receipt LFS → PCB notary (+ negativi POLICY_DENIED / replay / conflitto) | **già dimostrato** | `docs/M_G1B.md`; `docs/reports/m5-verify-bridge-e2e-2026-10-12.log` (33 marker) |
| Candidate ≠ Authority (AI senza selezione → nessun frame) | **già dimostrato** (host + guest E2E) | `docs/M_G1B.md` §1; `policy/tests/m_g1b_pipeline.rs` |
| Contratti v0 congelati (envelope, authority levels, receipt, errori) | **già dimostrato** come docs | `docs/CONTRACTS_v0.md`; wire reale ancora v2 |
| Concetti UI zero-app / vision prodotto | **già dimostrato** come asset | `docs/concepts/2026-10-12/` (da `4b9e8fd`); prototipo HTML e `UX_ZERO_APP.md` / `UX_FLOWS.md` sul branch sibling `grok/ux-zero-app` (**non** tutti i file UX sono su questo albero `1c94bd1`) |
| Shell prodotto + Intent Engine reale (BM25/RRF/LLM) collegati al PDP | **ancora aperto** | lavoro su `grok/product-intent-shell` (fuori da questo commit) |
| LFS durevole + epoch cross-boot + clock guest per TTL | **ancora aperto** | piano: `docs/PLAN_LFS_DURABLE_AND_CLOCK.md` |
| Boot nativo su hardware fisico dichiarato | **ancora aperto** | `docs/PLATFORM_SEL4.md` / QEMU-only oggi |
| ISO Debian/KDE preview storica (`components/`) = VelaOS M0–M5 | **non equivalenti** | non vendere la preview KDE come il PoC seL4; sono linee diverse |

**Gate obbligatorio:** non dichiarare **uso quotidiano (daily-use)** senza hardware target + clock/LFS durevole + shell reale. Massimo onesto oggi: **preview sviluppatore / PoC QEMU**.

---

## 2. Packaging — cosa spedire

### 2.1 Artefatto raccomandato per prima release pubblica "preview"

| Artefatto | Contenuto | Piattaforma consumatore | Priorità |
| --- | --- | --- | --- |
| **A. Bundle PoC QEMU** | script `verify-bridge` + `verify-bridge-e2e`, seed di test documentati, istruzioni WSL/Docker, log di esempio | Windows host + WSL2 + Docker (come oggi) | **P0** — allineato a ciò che è dimostrato |
| **B. Source tag** | tree git taggato + `BUILD.md` / `VERIFICATION.md` aggiornati | sviluppatori | **P0** |
| **C. ISO Debian/KDE** (`components/`, report `docs/reports/iso-*.json`) | desktop preview storico | x86_64 ISO | **P2** — solo se etichettata "preview desktop legacy", **non** come prova del bridge seL4 |
| **D. Immagine nativa board** | boot su hardware | TBD board | **bloccato** finché Fairha/Francesco non scelgono board |

**Decisione checklist:** la release "utilizzabile pubblica" v1 preview = **A+B**. C è opzionale e separata. D non entra nel tag senza gate hardware.

### 2.2 Cosa deve contenere il bundle A

- [ ] Binari/script di build Microkit pinned (versione da `docs/PLATFORM_SEL4.md` / pin check).
- [ ] Istruzioni path Windows → WSL (`--wsl`) come in `docs/M_G1B.md` §3.
- [ ] Seed **di test** chiaramente marcati non-produzione (`.local/`, gitignored).
- [ ] Log golden: `docs/reports/m5-verify-bridge-e2e-2026-10-12.log`.
- [ ] LICENSE + SECURITY note: chiavi e root Biscuit di test.

---

## 3. Installazione / avvio su macchina pulita

Target documentato: **Windows 10/11 + WSL2 + Docker**, checkout del tag.

### 3.1 Prerequisiti (da verificare su macchina pulita)

- [ ] Git, Rust toolchain (versione allineata a CI / `1.96.x` usata in Gate 0).
- [ ] WSL2 con Docker e `qemu-system-aarch64`.
- [ ] Microkit SDK come da `docs/PLATFORM_SEL4.md` / pin check.
- [ ] Nessuna dipendenza da worktree locali "sporchi" di agenti.

### 3.2 Passi minimi (bozza da validare pre-tag)

1. `git clone` + `git checkout <tag>`.
2. Host tests:
   - `cargo test --locked --manifest-path poc/m5_executor/keygen/Cargo.toml`
   - `cargo test --locked --manifest-path poc/m5_executor/policy/Cargo.toml`
   - `cargo test --locked --manifest-path poc/m5_executor/executor-host/Cargo.toml`
3. Generare seed test (documentare comandi keygen esistenti).
4. `cargo run … --example e2e_bundle -- …` poi `verify-bridge-e2e … --wsl`.
5. Criterio: exit PASS, marker attesi, nessun `panicked`/`FAULT` (come `M_G1B.md`).

### 3.3 Ancora aperto per "macchina pulita"

- [ ] Script one-shot `scripts/preview-e2e.ps1` (o equivalente) — **ancora aperto**.
- [ ] Installer Windows nativo senza WSL — **ancora aperto** (non richiesto per preview QEMU).
- [ ] Flusso utterance→UI reale senza mock — **ancora aperto** (product shell).

---

## 4. Documentazione utente necessaria

| Doc | Scopo | Stato |
| --- | --- | --- |
| Quickstart preview QEMU | clone → test → E2E | parziale (`M_G1B.md`, `BUILD.md`, `VERIFICATION*.md`) — unificare pre-tag |
| Flusso utterance → selezione → azione | cosa vede l'utente; Candidate ≠ Authority | concetti `docs/concepts/2026-10-12/`; UX prose su `grok/ux-zero-app` — **integrare nel tag** |
| Cosa l'AI non può fare | nessuna autorità, solo candidati | **già dimostrato** tecnicamente (`M_G1B.md`); manca pagina utente corta |
| Limiti noti | mock intent, LFS memoria, TTL host-only, no hardware | bozza §7 sotto |
| Undo / compensazione | aspettativa UX | modello UX sibling; **non** E2E guest completo |
| Privacy / chiavi di test | seed non sono trust root | citare `M_G1B.md` §4 |

---

## 5. Criteri "usabile" (must pass prima del tag preview)

### 5.1 Boot

| Criterio | Gate |
| --- | --- |
| QEMU virt AArch64: `verify-bridge` PASS | **già dimostrato** (anche su albero final-e2e) |
| QEMU: `verify-bridge-e2e` PASS (33 marker) | **già dimostrato** — `docs/reports/m5-verify-bridge-e2e-2026-10-12.log` |
| Boot nativo hardware | **ancora aperto** — non bloccante per tag **preview**; **bloccante** per daily-use |

### 5.2 Flusso

| Criterio | Gate |
| --- | --- |
| Intent mock → selezione → grant → execute → receipt | **già dimostrato** in E2E |
| AI-only → `POLICY_DENIED`, nessuna persistenza | **già dimostrato** |
| Utterance reale (shell + retrieval) → stessi gate | **ancora aperto** (product) |
| Click UI → `UserSelection{handle, expected_version}` | contratto PDP **già dimostrato**; bridge UI reale **ancora aperto** |

### 5.3 No-crash / fail-closed

| Criterio | Gate |
| --- | --- |
| MAX_FUEL → status 6, dominio risponde ancora | **già dimostrato** (R1 + marker E2E request 9–10) |
| POLICY_DENIED / AUTH / replay → no panic guest | **già dimostrato** nei path E2E/host |
| Fault dominio non-fuel (supervisor) | **ancora aperto** (`BRIDGE_CONTRACT` / PLATFORM) |

### 5.4 Limitazioni da elencare nel README di release (obbligatorio)

1. Intent/planner = mock finché product shell non è nel tag.
2. Biscuit verificato solo host (`std`); guest vede Ed25519 admission + digest.
3. LFS = ledger simulato in memoria (no durable, no epoch cross-boot).
4. TTL grant non enforced nel guest (no trusted clock).
5. Risultato Executor = un `u32` (EXEC-v2.1 E5) — niente UX ricca sul canale risultato.
6. Chiavi/seed di test ≠ produzione.
7. Nessuna board fisica certificata.

---

## 6. Gate preview vs daily-use

| | **Preview pubblica (sviluppatori)** | **Daily-use** |
| --- | --- | --- |
| Ambiente | QEMU virt (+ eventuale ISO legacy etichettata) | Hardware dichiarato + misura |
| UI | mock / prototipo / shell early OK se limiti chiari | Glass + Intent reali, undo |
| LFS | memoria OK se dichiarato | durable + fsync + power-loss note verificata |
| Clock/TTL | host-only OK se dichiarato | enforcement path documentato + test |
| Claim marketing | "PoC autoritativo + bridge verificato" | vietato senza gate hardware + durable + shell |
| Tag git | `preview-YYYYMMDD` o `v0.x.0-preview` | `v1.0.0` solo con checklist daily-use 100% |

**Regola:** un tag che non passa la colonna daily-use **non** può chiamarsi 1.0 "utilizzabile ogni giorno".

---

## 7. CI obbligatoria prima del tag

Minimo su commit di tag:

- [ ] `cargo test --locked` su `poc/m5_executor/keygen`, `policy`, `executor-host`.
- [ ] Workflow policy CI (da `grok/qa-policy-ci` / equivalente) verde.
- [ ] Pin Microkit check (come su `grok/sel4-platform`) se incluso nel tree.
- [ ] Job QEMU `verify-bridge` **o** artefatto log riprodotto e archiviato (se CI non ha Docker/WSL: richiedere run manuale firmata da Fairha con path log).
- [ ] Job/documentazione `verify-bridge-e2e` con log sotto `docs/reports/`.
- [ ] Nessun secret/seed di produzione nel tree.
- [ ] Lint docs: sezione "Limitazioni" presente nel README di release.

**ancora aperto:** CI full QEMU su runner GitHub hosted (dipende da Docker/AArch64); finché assente, il gate è "log locale + hash commit" approvato da Fairha.

---

## 8. Rollback / recovery

| Scenario | Azione |
| --- | --- |
| Tag preview difettoso | `git tag -d` locale + delete remote tag (solo Fairha); comunicare "usare tag precedente" |
| Regressione bridge | ripristinare ultimo SHA con `verify-bridge-e2e` PASS noto (`1c94bd1` o successivo verde) |
| Ledger durable corrotto (futuro) | recover da ultimo record CRC-ok; epoch bump; receipt ambigui → notary query (vedi piano LFS) |
| Chiave di test leak | ruotare seed; non è trust root produzione ma invalidare bundle che la embedda |
| Utente su ISO legacy | percorso separato; non rollback del PoC seL4 |

---

## 9. Checklist operativa pre-annuncio pubblico

1. [ ] Product shell/intent chiuso **oppure** annuncio limita esplicitamente a "E2E mock + QEMU".
2. [ ] README limitazioni (§5.4) rivisto da Fairha.
3. [ ] CI + log E2E allegati al release GitHub.
4. [ ] Nessun claim daily-use / hardware.
5. [ ] Piano LFS/clock (`docs/PLAN_LFS_DURABLE_AND_CLOCK.md`) linkato come "next".
6. [ ] Decisione packaging A+B confermata; C solo con disclaimer.
7. [ ] Contatto/security disclosure path indicato.

---

## 10. Riferimenti rapidi su `1c94bd1`

- `docs/M_G1B.md` — M-G1b/c + istruzioni E2E  
- `docs/reports/m5-verify-bridge-e2e-2026-10-12.log` — log QEMU E2E  
- `docs/reports/m5-verify-bridge-2026-10-12-r1-max-fuel.log` — R1  
- `poc/m5_executor/EXECUTOR_CONTRACT.md` — EXEC-v2.1  
- `poc/m5_executor/BRIDGE_CONTRACT.md` — ownership / gate aperti  
- `docs/CONTRACTS_v0.md` — C1–C5  
- `docs/concepts/2026-10-12/` — vision UI  
- Sibling (non base): `grok/ux-zero-app` → `docs/UX_ZERO_APP.md`, `docs/UX_FLOWS.md`, `docs/ux-prototype/index.html`
