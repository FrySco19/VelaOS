#!/usr/bin/env bash
# VelaOS verify-bridge-e2e: one-command QEMU end-to-end check.
#
# Wraps the existing, already-passing pipeline (no new verification logic):
#   1. policy/examples/e2e_bundle  -> host PDP: intent -> candidates -> UserSelection
#                                     -> Biscuit LFS_READ grant -> signed frames
#   2. keygen/bin/verify-bridge-e2e -> docker build Dockerfile.e2e (seL4 Microkit +
#                                     QEMU aarch64), boot, check 37 markers exactly once,
#                                     no panic/FAULT, expired grant never reaches LFS.
#
# Usage (from anywhere):   scripts/verify-bridge-e2e.sh      or   make e2e
# Needs: bash, git, cargo (Rust >= 1.90), Docker with BuildKit, ~6 GB disk, network
# for the first build. See packaging/e2e/README.md.
#
# Optional env:
#   VELA_PCB_SEED=path   existing Executor-admission seed matching the committed anchor
#                        (default .local/m5-executor.seed). If absent, an EPHEMERAL key
#                        is generated, the public anchor is patched only for this run
#                        and restored afterwards (works on any clean machine).
#   VELA_E2E_OUT=dir     output dir (default .local/e2e)
#   CARGO_FLAGS=...      extra cargo flags, e.g. --offline
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
EXEC=poc/m5_executor
ANCHOR="$EXEC/app/src/trusted_key.rs"
OUT="${VELA_E2E_OUT:-.local/e2e}"
LOG="$OUT/verify-bridge-e2e.log"          # full QEMU serial output
SUMMARY="$OUT/verify-bridge-e2e.summary.txt"  # runner verdict
CARGO_FLAGS="${CARGO_FLAGS:-}"

say()  { printf '\n==> %s\n' "$*"; }
die()  { printf '\nFAIL: %s\n' "$*" >&2; exit 1; }

say "Preflight"
command -v cargo >/dev/null || die "cargo not found: install Rust (https://rustup.rs), >= 1.90"
DOCKER=(docker)   # Windows host with Docker in WSL: use scripts/verify-bridge-e2e.ps1
command -v "${DOCKER[0]}" >/dev/null || die "docker not found: install Docker Engine/Desktop"
"${DOCKER[@]}" info >/dev/null 2>&1 || die "Docker daemon not reachable (start Docker; check permissions)"
"${DOCKER[@]}" buildx version >/dev/null 2>&1 || die "Docker BuildKit/buildx missing (needed for --secret mounts)"
echo "cargo: $(cargo --version)"
echo "docker: $("${DOCKER[@]}" version --format '{{.Server.Version}}' 2>/dev/null)"
mkdir -p "$OUT"
chmod 700 "$OUT" 2>/dev/null || true

# --- Keys -------------------------------------------------------------------
LFS_SEED="$OUT/m5-lfs.seed"   # created by e2e_bundle if missing; never committed
PCB_SEED="${VELA_PCB_SEED:-.local/m5-executor.seed}"
ANCHOR_BACKUP=""
restore_anchor() {
  if [[ -n "$ANCHOR_BACKUP" && -f "$ANCHOR_BACKUP" ]]; then
    cp "$ANCHOR_BACKUP" "$ANCHOR" && rm -f "$ANCHOR_BACKUP"
    echo "(restored committed trust anchor $ANCHOR)"
  fi
}
trap restore_anchor EXIT INT TERM

if [[ -f "$PCB_SEED" ]]; then
  say "Using existing PCB seed $PCB_SEED (must match committed anchor)"
else
  say "No PCB seed found -> ephemeral test key for this run only"
  PCB_SEED="$OUT/ephemeral-pcb.seed"
  rm -f "$PCB_SEED"
  ANCHOR_BACKUP="$OUT/trusted_key.rs.committed"
  cp "$ANCHOR" "$ANCHOR_BACKUP"
  # keygen creates the seed (OS entropy) and writes the matching public anchor.
  cargo run $CARGO_FLAGS --locked -q --manifest-path "$EXEC/keygen/Cargo.toml" --bin vela-executor-keygen -- \
    "$PCB_SEED" "$ANCHOR"
fi
chmod 600 "$PCB_SEED" 2>/dev/null || true

# --- 1. Host PDP bundle -------------------------------------------------------
say "Step 1/2: host PDP bundle (intent -> UserSelection -> grant -> signed frames)"
cargo run $CARGO_FLAGS --locked -q --manifest-path "$EXEC/policy/Cargo.toml" \
  --example e2e_bundle -- "$PCB_SEED" "$LFS_SEED" "$EXEC/app/generated"

# --- 2. QEMU e2e (docker build + boot + marker check) -------------------------
say "Step 2/2: docker build Dockerfile.e2e + QEMU boot + 37-marker check (first run: 15-40 min)"
set +e
cargo run $CARGO_FLAGS --locked -q --manifest-path "$EXEC/keygen/Cargo.toml" --bin verify-bridge-e2e -- \
  "$EXEC" "$PCB_SEED" "$LFS_SEED" "$LOG" 2>&1 | tee "$SUMMARY"
rc=${PIPESTATUS[0]}
set -e

echo
echo "Serial log : $ROOT/$LOG"
echo "Verdict    : $ROOT/$SUMMARY"
if [[ $rc -eq 0 ]] && grep -q '^PASS (e2e): 37 markers' "$SUMMARY"; then
  echo "RESULT: PASS (37 markers incl. POLICY_DENIED + MAX_FUEL, no panic/FAULT)"
else
  die "verify-bridge-e2e failed (exit $rc). Read the verdict file above, then the serial log."
fi
