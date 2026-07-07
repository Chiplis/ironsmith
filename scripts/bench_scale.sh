#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_WASM="${BENCH_SCALE_BUILD_WASM:-1}"

cd "$ROOT"
mkdir -p reports/bench

echo "Running native scale criterion benches..."
cargo bench -p ironsmith-runtime --features bench-support --bench scale

if [[ "$BUILD_WASM" != "0" ]]; then
  echo "Building release WASM for scale snapshot bench..."
  ./rebuild-wasm.sh
fi

echo "Running WASM scale snapshot bench..."
node scripts/bench_wasm_scale.mjs
