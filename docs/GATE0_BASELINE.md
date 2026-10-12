# Gate 0 — Baseline as-built / Inventario reale

Data: 2026-10-12 · Branch di riferimento: `codex/lfs-neural-m1` @ `47bb4669ca393e19e2cde9314b76fa54319f9a2a` (worktree pulito, allineato a `origin`).
Questo documento distingue ciò che è stato **eseguito oggi** da ciò che è **documentato** nel repository. Nessuna affermazione qui vale come qualificazione hardware.

## 1. Checkout ispezionato

| Voce | Valore |
| --- | --- |
| Repo principale | `outputs/VelaOS-public` |
| Branch / HEAD | `codex/lfs-neural-m1` / `47bb466` — "test(m5): reproduce bridge boot and specify ownership and remaining gates" |
| Modifiche locali | nessuna (worktree pulito) |
| Altri worktree (non toccati) | `VelaOS-m3-m4-design` (`codex/m3-m4-design` @ 110a596), `VelaOS-runtime` (`docs/qemu-preview-boot` @ 95885c2), pool kilo `solstice-power` (detached @ 4a4de8f) |
| Questo worktree | `outputs/VelaOS-gate0-contracts`, branch `vela/gate0-contracts` da `47bb466` (solo documenti) |

Commit recenti rilevanti: `53a873d` (M2 Biscuit endpoint authority + opaque handles) → `ff6cba9` (M5 Gate D Wasmi in QEMU) → `278a0df` (M5 unsafe review, Phase 1 IPC) → `3e8cfe1` (M5 PCB auth + Wasm bounded over IPC) → `47bb466` (riproduzione boot bridge + contratto ownership).

## 2. Crate e binari

| Percorso | Crate | Binari | Ruolo | Dipendenze chiave |
| --- | --- | --- | --- | --- |
| `poc/m0/` | (Python, non Rust) | `hybrid_recall.py`, `evaluate_gate.py` | Spike di semantic hybrid recall | — |
| `poc/m1/` | `vela-lfs-m1` (edition 2024, rust ≥1.90) | `crash-gate` (feature `fault-fixtures`) | Kernel LFS: oggetti cifrati (ChaCha20-Poly1305), commit firmati (Ed25519), policy Biscuit, redb | `biscuit-auth =6.0.0`, `redb =4.3.0`, `blake3`, `argon2` |
| `poc/m2/` | `vela-neural-bus-m2` | `bus-spike` (feature `fixture-e2e`) | Neural Bus iceoryx2, authority Biscuit con handle opachi, sandbox Wasmtime + WIT (`wit/task.wit`) | `iceoryx2 =0.10.0`, `wasmtime =36.0.2`, `biscuit-auth =6.0.0`, `vela-lfs-m1` |
| `poc/m5_executor/app/` | `velaos-m5-executor` (rust ≥1.92, nightly-2025-10-20, target `aarch64-sel4-microkit`) | `velaos-executor`, `velaos-pcb` | Due domini Microkit: PCB ed Executor Wasmi `no_std` | `sel4-microkit` (rust-sel4 @ d441a88), `wasmi =2.0.0`, `ed25519-compact =2.6.0` |
| `poc/m5_executor/keygen/` | `vela-executor-keygen` | `vela-executor-keygen`, `verify-bridge` | Generazione seed/anchor, runner QEMU di verifica, test host descriptor/crypto | `ed25519-compact`, `rand` |

Altro in `poc/m5_executor/`: `Dockerfile` (Microkit 2.3.0), `Makefile`, `executor.system`, `check-wx.py`, `BRIDGE_CONTRACT.md`, `PHASE_2_3_REPORT.md`, `phase-2-3-boot.log`.

## 3. CI (GitHub Actions)

| Workflow | Trigger | Contenuto | Esito su `47bb466` |
| --- | --- | --- | --- |
| `checks.yml` "Source checks" | ogni push/PR | Debian trixie: sintassi shell, compileall, pytest componenti storici (terminal, assistant, apps) | **success** (verificato) |
| `lfs-m1.yml` | path `poc/m1/**` | Windows, Rust 1.96.1: fmt, test, clippy `-D warnings`, crash smoke 200 kill | non attivato da `47bb466` (path non toccati); ultimo stato da verificare sul commit che lo ha attivato |
| `neural-bus-m2.yml` | path `poc/m1/**`, `poc/m2/**` | Windows, Rust 1.96.1: fmt, test `--all-features`, clippy, build fixture | come sopra |

Nota: **nessun workflow CI compila o avvia `poc/m5_executor`**. La verifica M5 è solo locale (Docker + QEMU).

## 4. Eseguito oggi vs documentato

| Prova | Eseguito oggi (2026-10-12) | Risultato |
| --- | --- | --- |
| `cargo test --locked` in `poc/m1` | sì | 6 passati, 0 falliti |
| `cargo test --locked` in `poc/m2` | sì | 7 passati, 0 falliti |
| `cargo test --locked` in `poc/m5_executor/keygen` | sì | 2 passati, 0 falliti |
| CI `checks` su `47bb466` | letto da GitHub | success |
| `verify-bridge` (build Docker + boot QEMU M5) | **no, non rieseguito oggi** | documentato PASS in `PHASE_2_3_REPORT.md` + `phase-2-3-boot.log` |
| Gate crash M1 10.000 kill | no | documentato in `poc/m1/VALIDATION.md` |
| Benchmark M2 iceoryx2 (62.222 msg/s, p99 17 µs) | no | documentato in `poc/m2/README.md` |
| Recall M0 186/200 | no | documentato in `poc/m0/GATE.md` |
| Qualsiasi hardware fisico | **no** | **non verificato** |

## 5. Stato per capacità

Legenda: **implementato** = codice + test eseguiti/CI; **prototipo** = codice PoC con evidenza solo documentata o parziale; **documentato** = solo specifica; **non verificato** = né codice né prova.

| Capacità | Stato | Evidenza |
| --- | --- | --- |
| LFS: oggetti cifrati, commit firmati, open via token Biscuit | implementato (host) | `poc/m1/src`, test m1 6/6 oggi |
| LFS: resistenza a crash di processo | prototipo | `poc/m1/VALIDATION.md`, `crash-report.json` (non rieseguito) |
| LFS: power-loss, anti-rollback, servizio persistente | non verificato | `docs/ROADMAP.md` (fuori scope M1) |
| Bus: authority Biscuit + handle opachi + revoca handle | implementato (host) | `poc/m2/src/authority.rs`, test m2 7/7 oggi |
| Bus: IPC iceoryx2 tra processi | prototipo | `poc/m2/bridge-results.json` |
| Bus: Wasmtime sandbox WIT senza WASI | prototipo | `poc/m2/src/sandbox.rs`, CI m2 |
| Bus: lane bulk streaming / crediti | documentato | `docs/ROADMAP.md` |
| M5: frame v2 (header 20 B, claims 32 B, firma Ed25519 64 B) e parsing | implementato (host) + prototipo (guest) | `app/src/bridge.rs`, keygen test 2/2 oggi |
| M5: pagina condivisa 4 KiB, protected call 0x701/0x702, ack | prototipo (QEMU, non rieseguito oggi) | `BRIDGE_CONTRACT.md`, `phase-2-3-boot.log` |
| M5: Wasmi fuel/memoria/import rifiutati, risultato 7 | prototipo (QEMU) | `PHASE_2_3_REPORT.md` |
| M5: replay entro la sessione | prototipo (QEMU) | idem |
| PCB: verifica Biscuit attenuato + admission strutturata | documentato | `BRIDGE_CONTRACT.md` §Required authorization boundary |
| Expiry con clock fidato, revoca, epoch, replay persistente | documentato | idem |
| Request → PCB → Executor → LFS con receipt | documentato | `BRIDGE_CONTRACT.md` §Required LFS and Bus flow |
| Supervisore / crash recovery dominii | documentato | idem |
| Shell UX / superfici effimere | non verificato | nessun codice nel branch |
| Driver display/input/storage su board | non verificato | nessun target hardware dichiarato |
| RAM idle < 1,5 GB, intent→execution < 250 ms | non verificato (ipotesi) | `docs/ROADMAP.md` |

## 6. Rischi noti dalla baseline

1. Seed di firma di test compilato dentro l'ELF PCB: non distribuibile (README M5).
2. Il token PCB attuale è Ed25519 a claims fissi: **non** è un verificatore Biscuit/Datalog.
3. M5 non è in CI: regressioni del bridge non vengono intercettate automaticamente.
4. `components/`, `build/`, `archive/` (distro Debian/KDE storica) coesistono nel branch: fuori scope, da non modificare.
5. KVM/hardware non disponibili per i gate prestazionali: le cifre QEMU non valgono come latenza.
