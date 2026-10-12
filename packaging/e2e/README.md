# VelaOS — verify-bridge-e2e packaging (QEMU) + demo Glass

One command reproduces the QEMU end-to-end check of the seL4/Microkit bridge
(intent -> candidates -> UserSelection -> Biscuit LFS_READ grant -> fuel-safe Wasm
-> LFS receipt notarised by PCB, plus the guest-clock TTL expiry), with no
knowledge of the repo internals. It wraps the existing, already-passing runner
(`poc/m5_executor/keygen/src/bin/verify-bridge-e2e.rs` + `Dockerfile.e2e`); it adds
no new verification logic.

## Prerequisites

| Need | Version / note |
|------|----------------|
| git | any (to clone) |
| Rust + cargo | >= 1.90 via https://rustup.rs (host std; the guest nightly-2025-10-20 toolchain is installed *inside* the image) |
| Docker with BuildKit (`docker buildx`) | Engine >= 23 or Docker Desktop. On Windows: Docker Engine inside WSL **or** Docker Desktop |
| Disk / time | ~6 GB free; first run 15–40 min (downloads Ubuntu, rustup nightly, Microkit SDK 2.3.0 with pinned SHA-256), later runs a few minutes |
| Network | first run only (crates.io, rustup, GitHub Microkit release) |
| bash + make | Linux / macOS / WSL only (`make e2e`) |

No key needs to exist. If `.local/m5-executor.seed` (the seed matching the committed
anchor) is missing, the script generates an **ephemeral** test key, patches
`poc/m5_executor/app/src/trusted_key.rs` for this run only and restores it on exit
(also on failure / Ctrl+C). Seeds stay in the gitignored `.local/`.

## ONE command

Linux / macOS / WSL (Docker + cargo in the same environment), from the repo root:

```bash
make e2e            # == bash scripts/verify-bridge-e2e.sh
```

Windows (cargo on Windows, Docker inside WSL — Francesco's setup), from the repo root:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\verify-bridge-e2e.ps1
```

Options: `-NativeDocker` (Docker Desktop instead of WSL docker), `-PcbSeed <path>`
(reuse an existing seed), `-Offline` (cargo `--offline`). Bash equivalents:
`VELA_PCB_SEED=<path>`, `CARGO_FLAGS=--offline`, `VELA_E2E_OUT=<dir>`.

## Where to read the result

| File | Content |
|------|---------|
| `.local/e2e/verify-bridge-e2e.log` | complete QEMU serial output (loader, seL4, every `VelaOS ...` line) |
| `.local/e2e/verify-bridge-e2e.summary.txt` | runner verdict: the `VelaOS` lines + `PASS (e2e): ...` or the failing assertion |

Last line on screen: `RESULT: PASS ...` (exit 0) or `FAIL: ...` (exit 1).

## PASS criteria (enforced by the runner, not by eye)

1. Image builds; QEMU runs until the 30 s idle timeout (exit 124 = clean idle, not a crash).
2. No `panicked` and no `FAULT` in the serial log.
3. **37 markers, each exactly once**, including:
   - `E2E [2]: AI-only request=11 POLICY_DENIED code=10 frames_signed=0, nothing published` (AI without UserSelection gets no authority)
   - `MAX_FUEL stack-safe status=6` and `fuel exhaustion and 64 KiB memory cap verified` (R1 fuel limits)
   - `E2E CLK [2]: request=13 grant ttl=1s expired in-guest ... -> EXPIRED code=7` (guest monotonic clock TTL)
   - final `ALL E2E CHECKS PASSED; intent -> selection -> grant -> Wasm 21 -> receipt notarized`
4. Forbidden: any `LFS committed epoch=1 request=13` / `LFS refused request=13` (expired grant reaching LFS).
5. Summary prints `PASS (e2e): 37 markers, no panic/FAULT, clean idle timeout, ...`.

Reference passing log: `docs/reports/m5-verify-bridge-e2e-lfs-clock-2026-10-12.log`.

## Troubleshooting

- `Docker daemon not reachable` — WSL: `wsl -e sudo service docker start`; Desktop: start it.
- `PCB seed does not match the committed Executor trust anchor` — you passed a seed that is
  not the one behind `trusted_key.rs`; omit `-PcbSeed`/move `.local/m5-executor.seed` away to
  use an ephemeral key.
- The image tag `velaos-executor-e2e` is shared: do not run two e2e jobs on one Docker daemon at once.

## Avvio rapido demo Glass (Windows)

Demo host della Glass UI (Avvia / Rivedi piano -> `UserSelection` -> PDP). Doppio clic su
`scripts\launch-glass.bat`, oppure dalla radice del repo:

```bat
scripts\launch-glass.bat "C:\Users\tella\Documents\Codex\2026-10-09\in-c-users-tella-documents-codex\outputs\VelaOS-public\poc\m0"
```

Poi apri **http://127.0.0.1:7878/**. Senza argomento lo script usa `poc\m0` solo se contiene
`.venv` (il repo non include `.venv`/`.model-cache`), altrimenti `%VELA_M0_DIR%`; se nessuno
esiste si ferma e mostra l'uso. `scripts\launch-glass.bat --no-m0` avvia solo la lane BM25
in-process (senza Python). Gira con `cargo --offline`: serve aver già scaricato i crate una
volta (`cargo fetch --manifest-path poc\product\glass\Cargo.toml`). In caso di errore la
finestra resta aperta (`pause`).

Collegamento sul Desktop (PowerShell, dalla radice del repo):

```powershell
$r=(Get-Location).Path; $s=(New-Object -ComObject WScript.Shell).CreateShortcut("$([Environment]::GetFolderPath('Desktop'))\VelaOS Glass.lnk"); $s.TargetPath="$r\scripts\launch-glass.bat"; $s.Arguments='"C:\Users\tella\Documents\Codex\2026-10-09\in-c-users-tella-documents-codex\outputs\VelaOS-public\poc\m0"'; $s.WorkingDirectory=$r; $s.Save()
```

La demo Glass è separata dall'e2e QEMU: quest'ultimo resta `make e2e` / `scripts\verify-bridge-e2e.ps1`.

## Limits

- Wraps a QEMU (cortex-a53) run, not physical hardware; LFS in the guest is still the
  simulated in-memory ledger (M-LFS-4 virtio-blk not implemented here).
- Ephemeral-key mode proves the pipeline, not the committed anchor; the PCB test ELF embeds its seed.
- Images are not pushed to any registry; every machine builds locally.
