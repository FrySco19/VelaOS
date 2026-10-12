# Dipendenze dalla piattaforma seL4 — cosa serve agli altri agenti

Owner: agente seL4/piattaforma · Branch `vela/sel4-platform` · Data: 2026-10-12.
Riferimenti: `docs/PLATFORM_SEL4.md`, `docs/SEL4_GATE1_SLICE.md`, `docs/CONTRACTS_v0.md` (C2, C3, C6).

## Cosa la piattaforma fornisce oggi (QEMU virt AArch64)

- Due PD (`pcb`, `executor`), una pagina condivisa 4 KiB (PCB RW / Executor R), protected call single-flight `0x701`/`0x702` — **verificato in QEMU il 2026-10-12 (PASS)**.
- Pin toolchain/SDK controllati da `check-pins.py`.
- Nient'altro: niente supervisore, epoch, clock, storage, display, input, rete.

## Bisogni per agente

| Agente | Ha bisogno dalla piattaforma | Stato | Blocca? |
| --- | --- | --- | --- |
| **PCB / authority** | PDP host (M-G1a) non richiede nulla dal guest | — | no |
| PCB | `epoch` autenticata per envelope v0 e chiave `(epoch, request_id)` | design (OWN-v3, supervisore) | Gate 1b |
| PCB | clock fidato per `not_after`/expiry e TTL Biscuit in guest | assente (nessun driver timer) | Gate 2 (expiry in guest) |
| PCB | provisioning chiave PCB fuori dall'ELF (oggi seed di test incorporato) | assente | Gate 2 / prima di qualunque preview |
| PCB | Biscuit in-guest: allocatore + `std`-like o verificatore `no_std` | non fattibile con `biscuit-auth =6.0.0` | no (PDP host in Gate 1) |
| **Executor / LFS** | regione dati in ingresso/uscita oltre la pagina 4 KiB (handle opachi, blocco risposta tipizzato C1) | assente; richiede modifica `executor.system` | Gate 1b |
| Executor / LFS | storage persistente in guest (virtio-blk su QEMU) per receipt e replay cross-boot | assente | Gate 1 E2E **in guest**; per Gate 1 si può tenere LFS host |
| Executor | supervisore che riavvia l'Executor dopo fault (OWN-v3) | design | Gate 2 |
| Executor | deadline wall-clock (timer) oltre il fuel | assente | Gate 2 |
| **UX** | nessun display/input nel guest seL4 oggi: la UI deve girare **su host** (prototipo HTML/mock) collegata a log/eventi | — | no per Gate 1 (UI host) |
| UX | canale diagnostico strutturato (stato richieste, epoch, errori C5) per mostrare provenienza/ambiguità | assente (solo seriale testuale) | Gate 1b se la UI deve leggere eventi reali dal guest |
| UX | display + input in guest | assente; dipende dalla board | Gate 3 |
| **QA** | `verify-bridge` riproducibile + log | **disponibile**, PASS oggi | no |
| QA | CI che compila/avvia il guest | non presente; servono runner Linux con Docker e un seed di test generato in CI (secret o seed effimero + `trusted_key.rs` generato al volo) | no per Gate 1 |
| QA | misure RAM/latenza | richiedono target dichiarato (non QEMU TCG) | Gate 4 |

## Proposte di sequenza (piattaforma)

1. Gate 1 (ora): pin check + CI pin; `verify-bridge` riprodotto — **fatto su questo branch**.
2. Gate 1b: PD `supervisor` + epoch boot-local (OWN-v3 S1–S6) — modifica `executor.system` e `app/` → richiede coordinamento con Executor/PCB e nuovi marker (ADR).
3. Gate 1b/2: regione risposta tipizzata (C1) + timer (ARM generic timer su QEMU virt) per expiry/deadline.
4. Gate 2: virtio-blk su QEMU per receipt/replay cross-boot; provisioning chiave PCB.
5. Gate 3: board fisica (decisione del proprietario), driver display/input/storage/clock reali.

## Domande aperte per Francesco

1. **Board fisica di riferimento per Gate 3.** Nessuna scelta fatta; nessuna affermazione hardware finché non decidi. Criteri utili: supporto Microkit/seL4 ufficiale per la board, documentazione driver (display, USB input, storage, timer), disponibilità e costo. Candidati tipici supportati da Microkit esistono (es. board i.MX8M, Odroid, Raspberry Pi 4 con limiti noti), ma **nessuno è stato verificato** in questo repo: l'elenco va controllato sulla versione di Microkit che sceglieremo. Va bene restare su QEMU virt AArch64 fino a tua decisione?
2. **Seed di test per CI guest**: accetti un seed di test effimero generato in CI (che rigenera `trusted_key.rs` solo nel runner, senza commit), oppure preferisci un secret GitHub? Nessuno dei due è un trust root di produzione.
3. **Supervisore**: ok ad aggiungere un terzo PD `supervisor` e ad annidare `pcb`/`executor` sotto di esso (cambia `executor.system` e richiede nuovi marker `verify-bridge` tramite ADR)?
4. **Storage in guest**: per Gate 1 basta l'LFS host (`poc/m1`) o vuoi la persistenza dei receipt già dentro seL4 (virtio-blk su QEMU)?
5. **Aggiornamento di Microkit**: restare su 2.3.0 (pin attuale, verificato) o pianificare un aggiornamento prima di Gate 3?