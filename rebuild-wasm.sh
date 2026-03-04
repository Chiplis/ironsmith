#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$ROOT_DIR/pkg"
DEMO_PKG_DIR="$ROOT_DIR/web/wasm_demo/pkg"
FALSE_POSITIVES_FILE="$ROOT_DIR/scripts/semantic_false_positives.txt"

DIMS="${IRONSMITH_WASM_SEMANTIC_DIMS:-384}"
FEATURES="wasm,generated-registry"
SCORES_FILE="${IRONSMITH_GENERATED_REGISTRY_SCORES_FILE:-}"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

usage() {
  cat <<'USAGE'
Usage: ./rebuild-wasm.sh [--dims <int>] [--features <csv>] [--scores-file <path>]

Examples:
  ./rebuild-wasm.sh
  ./rebuild-wasm.sh --dims 384
  ./rebuild-wasm.sh --scores-file /tmp/ironsmith_wasm_semantic_audits_384.json

Notes:
  - Threshold-based registry filtering is removed.
  - A per-card semantic score report is used to bake fidelity scores into WASM.
  - If --scores-file is omitted, the script computes a fresh audits report.
  - Default features are "wasm,generated-registry".
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dims)
      [[ $# -ge 2 ]] || { echo "missing value for --dims" >&2; exit 1; }
      DIMS="$2"
      shift 2
      ;;
    --features)
      [[ $# -ge 2 ]] || { echo "missing value for --features" >&2; exit 1; }
      FEATURES="$2"
      shift 2
      ;;
    --scores-file)
      [[ $# -ge 2 ]] || { echo "missing value for --scores-file" >&2; exit 1; }
      SCORES_FILE="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

cd "$ROOT_DIR"
require_cmd cargo
require_cmd wasm-pack

if [[ -z "$SCORES_FILE" ]]; then
  SCORES_FILE="${TMPDIR:-/tmp}/ironsmith_wasm_semantic_audits_${DIMS}.json"
  echo "[INFO] computing semantic audits report (dims=${DIMS})..."
  AUDIT_CMD=(
    cargo run --quiet --no-default-features --bin audit_oracle_clusters --
    --cards "$ROOT_DIR/cards.json"
    --use-embeddings
    --embedding-dims "$DIMS"
    --min-cluster-size 1
    --top-clusters 0
    --examples 1
    --audits-out "$SCORES_FILE"
  )
  if [[ -f "$FALSE_POSITIVES_FILE" ]]; then
    AUDIT_CMD+=(--false-positive-names "$FALSE_POSITIVES_FILE")
  fi
  "${AUDIT_CMD[@]}"
fi

export IRONSMITH_GENERATED_REGISTRY_SCORES_FILE="$SCORES_FILE"
echo "[INFO] semantic scores source: $IRONSMITH_GENERATED_REGISTRY_SCORES_FILE"

wasm-pack build --release --target web --features "$FEATURES"

mkdir -p "$DEMO_PKG_DIR"
cp -f \
  "$PKG_DIR/ironsmith.js" \
  "$PKG_DIR/ironsmith_bg.wasm" \
  "$PKG_DIR/ironsmith.d.ts" \
  "$PKG_DIR/ironsmith_bg.wasm.d.ts" \
  "$PKG_DIR/package.json" \
  "$DEMO_PKG_DIR/"
