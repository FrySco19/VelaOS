# Vela Soft development launcher

Double-click `vela-up.bat` or the local **Vela Soft DEV** desktop shortcut.
A terminal stays open for approvals. Press Enter when prompted to hydrate the
catalog, open a version or approve a save. Any non-empty line or EOF denies.
Only `--dev-virtual-fido` enables this sensor replacement. The default broker
without this flag still requires an enrolled USB security key.

The development mode is deliberately NOT FIDO hardware authentication. Its
red startup warning and `DevelopmentVirtualFido` receipt metadata distinguish
it from the real USB mode. Final Ed25519 signatures, guest checks, Wasm,
immutable disk writes and receipt verification remain real. Redirected input
is rejected even with the flag, so the launcher must expose a real terminal.

## Current prerequisites

Windows, WSL Debian with working Docker, the locally built `velaos-v1` image,
the enrolled local keys and the two Rust Windows executables. Missing
executables are built when Cargo is installed. Missing SDK image/enrollment
is reported; this is not a self-contained installer for a fresh PC.
No NVIDIA/CUDA dependency is introduced by the launcher.

The PowerShell launcher uses process-scoped RemoteSigned, preserves the system
policy and honors higher-priority organization policies. It will not bypass
restrictions on downloaded unsigned scripts.

`vela-start.ps1 -Hardware` requires the USB enrollment instead of the explicit
development flag. The ordinary double-click entry is DEV, as requested.
`-DataDirectory` selects a dedicated directory under this workspace `.local`
for isolated testing. Persistent notes survive stopping the launcher.

## Lifecycle

QEMU runs headless in an individually labelled container. Glass runs locally
and the broker inherits the launcher's console. The default browser opens
`http://127.0.0.1:7882/studio` after listeners are ready.

Ctrl+C/normal termination runs cleanup. A separate hidden watchdog also
handles closing/killing the launcher process, matching process executable,
start time and the exact container run label. It stops owned processes only;
it does not remove the disk or delete source files.

## Verified

- Rust hardware-verification tests still pass.
- The development happy path ran in an interactive terminal against a separate
  disk copy: hydration, two distinct notes, immutable update, old/new reads and
  page reload passed with `DevelopmentVirtualFido` and real signed receipts.
- Forced launcher termination stopped QEMU, broker and Glass; ports 7990/7882
  were no longer listening.
- Hardware positive verification remains pending an actual USB security key.
