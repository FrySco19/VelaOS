# Piattaforma seL4 / boot — stato as-built (M5)

Branch: `grok/sel4-platform` · Base: `1281f5a` (`grok/qa-policy-ci`, sopra M-G1a `031208e` e `47bb466`) · Data: 2026-10-12 · Owner: agente seL4/piattaforma.

Legenda: **eseguito oggi** = rieseguito su questo checkout il 2026-10-12; **documentato** = presente solo in report/README del repo; **non verificato** = né codice né prova.

> Nessuna affermazione hardware. Tutto ciò che segue vale per **QEMU `virt` AArch64**. QEMU non è una board: i 2 GiB configurati sono capacità, non consumo; nessuna latenza o RAM reale è stata misurata.

## 1. Cosa boota oggi (as-built)

Fonte: `poc/m5_executor/README.md`, `PHASE_2_3_REPORT.md`, `BRIDGE_CONTRACT.md`, `phase-2-3-boot.log`.

- seL4 configurato come hypervisor (EL2, `virtualization=on`), avviato dal loader Microkit; CapDL initializer; Microkit Monitor.
- **Due protection domain** Microkit, entrambi Rust `no_std` (rust-sel4):
  - `pcb` (priorità 100): produttore dei task di test, firma Ed25519 (seed di **test** incorporato nell'ELF).
  - `executor` (priorità 101): Wasmi 2.0.0 `no_std`, nessun WASI/import, fuel 1..10M, memoria lineare ≤ 64 KiB, stack interprete 16 KiB, ricorsione 64, heap 2 MiB.
- **Una pagina condivisa** di 4 KiB (`pcb_executor_page`): `pcb` RW, `executor` R, nessuna mappatura eseguibile.
- **Un canale** (id 0): `pcb` → `executor` via protected call (`pp=true`), label richiesta `0x701`, risposta `0x702` con 4 registri (status, risultato, request ID, contatore tentativi). Nessuna notifica come ingresso (`notify=false`).
- Executor fa uno snapshot privato della pagina prima di parse/verify/esecuzione (OWN-v2).
- Controllo W^X dei segmenti `PT_LOAD` di entrambi gli ELF (`check-wx.py`) a build time.
- Nove casi guest: successo (7), payload alterato, firma corrotta, offset invalido, quota eccessiva, replay, loop infinito (fuel), import WASI, successo dopo errori.

**Rieseguito oggi (2026-10-12, questo branch):** `verify-bridge` via Docker in WSL → **PASS** (tutti gli 11 marker esattamente una volta, nessun panic/FAULT, timeout 124 solo dopo il completamento). Durata ~2 min 18 s (con cache dei layer Docker toolchain/SDK). Immagine `4036923b10c9`. Log: [`docs/reports/m5-verify-bridge-2026-10-12-sel4-platform.log`](reports/m5-verify-bridge-2026-10-12-sel4-platform.log). Dettagli e comando esatto in [`SEL4_GATE1_SLICE.md`](SEL4_GATE1_SLICE.md).

## 2. Target pinnato (riferimento dei gate, non supporto hardware)

Letto da `Makefile`, `Dockerfile`, `app/rust-toolchain.toml`, `app/Cargo.toml`; verificato automaticamente da `poc/m5_executor/check-pins.py`.

| Elemento | Valore | Fonte |
| --- | --- | --- |
| Board Microkit | `qemu_virt_aarch64`, config `debug` | `Makefile` |
| Microkit SDK | **2.3.0**, SHA-256 `e12e507f…4f89` verificato in build | `Dockerfile` |
| Toolchain Rust guest | `nightly-2025-10-20` (+ `rust-src`, `llvm-tools-preview`) | `Dockerfile`, `rust-toolchain.toml` |
| Target triple | `aarch64-sel4-microkit` (target JSON custom) | `Makefile`, `app/targets/` |
| rust-sel4 | git rev `d441a880938209f661201704dfbee7b91c5aa905` | `app/Cargo.toml` |
| Runtime Wasm | `wasmi =2.0.0` (no_std) | `app/Cargo.toml` |
| Firma admission | `ed25519-compact =2.6.0` (no_std) | `app/Cargo.toml` |
| Immagine build | `ubuntu:22.04` (tag, **non** pinnata per digest) | `Dockerfile` |
| QEMU | `virt,virtualization=on`, `cortex-a53`, 2 GiB, `-nic none`, loader a `0x70000000` | `Makefile` (`run`) |
| QEMU usata dal gate (dentro l'immagine Docker) | **6.2.0** (`1:6.2+dfsg-2ubuntu6.31`, Ubuntu 22.04) | `docker run … --version`, 2026-10-12 |
| QEMU/Docker host (PC Francesco, WSL) | `qemu-system-aarch64` 10.0.13 (Debian 13), Docker 26.1.5 — **non** usata da `verify-bridge` | rilevato 2026-10-12 |

Nota: la versione QEMU del gate dipende dal pacchetto apt non pinnato dell'immagine; un rebuild futuro può cambiarla.

## 3. Domini: presenti e mancanti

| Dominio / servizio | Stato | Note |
| --- | --- | --- |
| PCB (`pcb`) | prototipo, boota in QEMU | producer di test; Biscuit **non** in-guest (`biscuit-auth` è solo `std`), PDP host in `poc/m5_executor/policy` |
| Executor (`executor`) | prototipo, boota in QEMU | Wasmi no_std, verifica Ed25519 |
| Pagina condivisa + protected call | prototipo verificato in QEMU | OWN-v2 |
| Supervisore / monitor di dominio | **mancante** | Microkit fornisce il fault handler del Monitor ma nessun PD supervisore VelaOS; nessun riavvio di dominio |
| Epoch di sessione | **mancante** | richiesto da OWN-v3 / envelope v0 |
| Clock fidato (timer) | **mancante** | nessun driver timer; expiry/TTL/deadline impossibili in guest |
| Storage persistente (virtio-blk o altro) | **mancante** | replay solo per boot; nessun LFS in guest |
| Display | **mancante** | solo seriale (`-nographic`) |
| Input (tastiera/puntatore) | **mancante** | solo seriale UART per output |
| Rete | **assente per scelta** | `-nic none` |
| Diagnostica | parziale | log seriale + `debug_println`; nessun canale diagnostico strutturato |
| LFS / Neural Bus | **solo host** | `poc/m1`, `poc/m2` su processi host, non domini seL4 |

## 4. Riproducibilità del boot — comandi esatti

Dalla radice del repo (PowerShell, Windows), con Rust e Docker (anche via WSL):

```powershell
# 1) solo la prima volta: seed di TEST + trust anchor pubblica (riscrive app/src/trusted_key.rs!)
New-Item -ItemType Directory -Force .local | Out-Null
cargo run --locked --manifest-path poc/m5_executor/keygen/Cargo.toml -- .local/m5-executor.seed poc/m5_executor/app/src/trusted_key.rs
icacls .local/m5-executor.seed /inheritance:r /grant:r "$($env:USERNAME):(F)"

# 2) test host descriptor/crypto
cargo test --locked --manifest-path poc/m5_executor/keygen/Cargo.toml

# 3) build + boot manuale
docker build --secret id=pcb_seed,src=.local/m5-executor.seed -f poc/m5_executor/Dockerfile -t velaos-executor-phase23 poc/m5_executor
docker run --rm --entrypoint timeout velaos-executor-phase23 20s make run

# 4) verifica automatica (consigliata); con Docker in WSL usare path /mnt/c/... e --wsl
cargo run --locked --manifest-path poc/m5_executor/keygen/Cargo.toml --bin verify-bridge -- poc/m5_executor .local/m5-executor.seed .local/m5-bridge-repro.log

# 5) controllo pin di piattaforma (nessun build, secondi)
python poc/m5_executor/check-pins.py
```

Attenzione: il seed deve corrispondere a `app/src/trusted_key.rs` committato. Rigenerarlo cambia `app/` e richiede rebuild di entrambi i domini. Un exit 124 (timeout) da solo **non** è un pass: serve il marker `ALL CHECKS PASSED` e nessun panic/FAULT.

Limiti di riproducibilità noti: immagine base `ubuntu:22.04` e pacchetti apt non pinnati per digest/versione; `rustup` scaricato da rete; la build non è bit-for-bit riproducibile (non verificato).

## 5. Recovery e aggiornamento — cosa NON è implementato

Da `BRIDGE_CONTRACT.md` e README M5:

- Se l'Executor crasha o non risponde, il PCB **non** deve assumere ack, sovrascrivere la pagina o rieseguire task con effetti. Recovery richiede **supervisore separato, epoch di sessione autenticata, stato durevole di admission/risultato**: **nessuno implementato**.
- Replay solo per il boot corrente; nessuno stato replay persistente; nessun TTL basato su clock; nessuna revoca/cancellazione in guest.
- Chiave di firma PCB di **test** incorporata nell'ELF PCB: nessun provisioning di chiavi di produzione, nessuna rotazione, nessun trust root hardware.
- Nessun aggiornamento atomico A/B, nessun rollback, nessun recovery boot, nessuna verifica firmata dell'immagine di boot.
- Fuel non limita il parsing del modulo né garantisce una deadline wall-clock.
- Nessuna misura RAM per dominio/kernel né distribuzione di latenza su target specificato.

## 6. Tabella stato piattaforma

| Voce | Stato | Evidenza |
| --- | --- | --- |
| Boot seL4+Microkit 2 PD in QEMU virt AArch64 | **implementato** (QEMU) — rieseguito oggi, PASS | log in `docs/reports/`, `verify-bridge` |
| Isolamento pagina condivisa RW/R, no exec | **implementato** (config + QEMU) | `executor.system`, `check-pins.py` |
| Protected call single-flight (OWN-v2) | **implementato** (QEMU) | marker `verify-bridge` |
| W^X sui segmenti ELF | **implementato** (build-time) | `check-wx.py` nel Makefile |
| Pin toolchain/SDK/dipendenze | **implementato** (config) + check host | `check-pins.py`, CI `m5-platform-pins.yml` (non ancora eseguita su GitHub: nessun push) |
| Build riproducibile bit-for-bit | **non verificato** | base image/apt non pinnati |
| Supervisore + crash/no-ack (OWN-v3) | **documentato** (design) | `docs/SEL4_GATE1_SLICE.md` §2, `CONTRACTS_v0.md` C2 |
| Epoch di sessione | **documentato** | `CONTRACTS_v0.md` C1/C2 |
| Clock fidato / expiry | **documentato** | `BRIDGE_CONTRACT.md` |
| Replay cross-boot (storage durevole) | **documentato** | `BRIDGE_CONTRACT.md` |
| Storage / display / input driver | **non verificato** (assenti) | — |
| Provisioning chiavi / rimozione seed da ELF PCB | **documentato** (limite noto) | README M5 |
| Aggiornamento atomico, rollback, recovery boot | **non verificato** (assenti) | — |
| Board fisica | **non verificato** — decisione del proprietario | `coordination/SEL4_DEPENDENCIES.md` |
| M5 guest in CI | **non verificato** | CI attuale: solo test host policy/keygen + check pin |