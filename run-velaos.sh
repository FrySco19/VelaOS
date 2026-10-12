#!/usr/bin/env bash
# Build and run the native Microkit persistence proof, not the host Glass demo.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
for tool in cargo docker python3; do
  command -v "$tool" >/dev/null || { echo "Missing prerequisite: $tool" >&2; exit 1; }
done
docker info >/dev/null 2>&1 || { echo "Start Docker before launching VelaOS." >&2; exit 1; }
docker buildx version >/dev/null
umask 077
mkdir -p .local
OUT="$(mktemp -d "$ROOT/.local/boot-XXXXXXXX")"
ANCHOR=poc/m5_executor/app/src/trusted_key.rs
cp "$ANCHOR" "$OUT/anchor.backup"
cleanup() { cp "$OUT/anchor.backup" "$ANCHOR"; }
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
echo "VelaOS v0.1.0: native seL4/Microkit persistence milestone"
echo "First build downloads the SDK/toolchain and can take 15-40 minutes; cached runs are faster."
cargo run --locked -q --manifest-path poc/m5_executor/keygen/Cargo.toml --bin vela-executor-keygen -- "$OUT/pcb.seed" "$ANCHOR"
VELA_HARDWARE=1 cargo run --locked -q --manifest-path poc/m5_executor/policy/Cargo.toml --example e2e_bundle -- "$OUT/pcb.seed" "$OUT/lfs.seed" poc/m5_executor/app/generated
if ! docker image inspect velaos-executor-e2e:latest >/dev/null 2>&1; then
  echo "Building pinned Microkit SDK/toolchain foundation..."
  docker build --secret "id=pcb_seed,src=$OUT/pcb.seed" --secret "id=lfs_seed,src=$OUT/lfs.seed" \
    -f poc/m5_executor/Dockerfile.e2e -t velaos-executor-e2e poc/m5_executor
fi
CACHE=()
if docker image inspect velaos-lfs-hardware:latest >/dev/null 2>&1; then
  CACHE=(--cache-image velaos-lfs-hardware:latest)
fi
python3 scripts/verify-lfs-hardware.py --pcb-seed "$OUT/pcb.seed" --lfs-seed "$OUT/lfs.seed" --out "$OUT/proof" "${CACHE[@]}"
echo "Logs and dedicated test disk: $OUT/proof"
