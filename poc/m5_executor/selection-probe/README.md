# UserSelection Microkit IPC integration probe

Test-only addition to the existing complete three-domain E2E image. Three extra
domains exercise actual shared-buffer notifications: input fixture -> PCB ->
authorization sink, with explicit ownership and acknowledgement states.

The PCB uses `vela-selection-contract` / `ed25519-dalek` without std. The fixture
publishes a signed selection, then a corrupted signature and an altered hash.
The sink asserts it receives exactly one mock authorization with the expected
hash; denied requests clear payloads and emit no authorization.

The signing seed is local and supplied through a BuildKit secret, but is embedded
in the TEST sender ELF. Never ship that ELF or this image as production. The PCB
contains only the public key. Session, revision and plan are fixed fixture values.
The probe does not establish human input, trusted rendering, real Biscuit policy,
expiry/revocation, durable replay protection or production page ownership against
a malicious peer. Mock tokens never enter the production Wasm Executor.

## Run

1. Build and verify the baseline with `scripts/verify-bridge-e2e.ps1` on Windows
   or `scripts/verify-bridge-e2e.sh` on a host with Cargo and Docker.
2. Generate an independent fixture seed and public anchor using
   `vela-executor-keygen`, placing the public Rust constant in
   `selection-probe/generated/trusted_input_public.rs`.
3. Run `scripts/verify-selection-ipc.py --input-seed <seed> --out <output>`
   inside WSL. The runner builds the combined six-domain image and verifies
   baseline markers plus the selection-specific markers from real QEMU output.

The 30-second QEMU timeout is intentional after all checks; exit 124 is accepted
only when every required marker appears exactly once and no panic/fault appears.
Build and complete serial logs are retained in the chosen output directory.
