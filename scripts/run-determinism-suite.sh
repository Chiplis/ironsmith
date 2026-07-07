#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_WASM="${DETERMINISM_BUILD_WASM:-1}"

cd "$ROOT"

echo "Running native same-seed determinism test..."
cargo test -p ironsmith-wasm same_seed_double_run_sync_checkpoint_is_byte_identical --lib

if [[ "$BUILD_WASM" != "0" ]]; then
  echo "Building release WASM for lockstep simulation..."
  ./rebuild-wasm.sh
fi

echo "Running two-peer lockstep simulation..."
node scripts/lockstep-sim.mjs

echo "determinism suite passed"
