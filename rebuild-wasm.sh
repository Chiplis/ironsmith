#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$ROOT_DIR/pkg"
DEMO_PKG_DIR="$ROOT_DIR/web/wasm_demo/pkg"
DEFAULT_FRONTEND_SCORES_FILE="$ROOT_DIR/web/ui/public/ironsmith_semantic_scores.json"

DIMS="${IRONSMITH_WASM_SEMANTIC_DIMS:-384}"
FEATURES="wasm,generated-registry"
BUILD_PROFILE="release"
THRESHOLD="${IRONSMITH_WASM_SEMANTIC_THRESHOLD:-}"
FRONTEND_SCORES_FILE="${IRONSMITH_FRONTEND_SEMANTIC_SCORES_FILE:-$DEFAULT_FRONTEND_SCORES_FILE}"
FRONTEND_SCORES_FILE_EXPLICIT=0
SCORES_FILE="${IRONSMITH_GENERATED_REGISTRY_SCORES_FILE:-}"
SCORES_FILE_EXPLICIT=0

ROOT_FALSE_POSITIVES_FILE="$ROOT_DIR/semantic_false_positives.txt"
LEGACY_FALSE_POSITIVES_FILE="$ROOT_DIR/scripts/semantic_false_positives.txt"
FALSE_POSITIVES_FILE="$ROOT_FALSE_POSITIVES_FILE"
if [[ ! -f "$FALSE_POSITIVES_FILE" && -f "$LEGACY_FALSE_POSITIVES_FILE" ]]; then
  FALSE_POSITIVES_FILE="$LEGACY_FALSE_POSITIVES_FILE"
fi

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

usage() {
  cat <<USAGE
Usage: ./rebuild-wasm.sh [--dev|--release] [--threshold <float>] [--dims <int>] [--features <csv>] [--scores-file <path>] [--frontend-scores-file <path>]

Examples:
  ./rebuild-wasm.sh --dev
  ./rebuild-wasm.sh --threshold 0.99
  ./rebuild-wasm.sh --dims 384
  ./rebuild-wasm.sh --scores-file /tmp/ironsmith_semantic_scores.json
  ./rebuild-wasm.sh --frontend-scores-file web/ui/public/ironsmith_semantic_scores.json

Notes:
  - Per-card semantic scores are loaded from --scores-file (default: --frontend-scores-file).
  - Frontend cache file defaults to $DEFAULT_FRONTEND_SCORES_FILE.
  - The script recomputes scores only when --threshold is provided.
  - If --threshold is omitted and the scores file is missing, the build fails.
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
    --threshold)
      [[ $# -ge 2 ]] || { echo "missing value for --threshold" >&2; exit 1; }
      THRESHOLD="$2"
      shift 2
      ;;
    --dev)
      BUILD_PROFILE="dev"
      shift
      ;;
    --release)
      BUILD_PROFILE="release"
      shift
      ;;
    --scores-file)
      [[ $# -ge 2 ]] || { echo "missing value for --scores-file" >&2; exit 1; }
      SCORES_FILE="$2"
      SCORES_FILE_EXPLICIT=1
      shift 2
      ;;
    --frontend-scores-file)
      [[ $# -ge 2 ]] || { echo "missing value for --frontend-scores-file" >&2; exit 1; }
      FRONTEND_SCORES_FILE="$2"
      FRONTEND_SCORES_FILE_EXPLICIT=1
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

if [[ "$SCORES_FILE_EXPLICIT" -eq 1 ]]; then
  :
elif [[ "$FRONTEND_SCORES_FILE_EXPLICIT" -eq 1 ]]; then
  SCORES_FILE="$FRONTEND_SCORES_FILE"
elif [[ -z "$SCORES_FILE" ]]; then
  SCORES_FILE="$FRONTEND_SCORES_FILE"
fi

if [[ -n "$THRESHOLD" ]]; then
  mkdir -p "$(dirname "$SCORES_FILE")"
  echo "[INFO] computing semantic audits report (dims=${DIMS}, threshold=${THRESHOLD})..."
  AUDIT_CMD=(
    cargo run --quiet -p ironsmith-tools --bin audit_oracle_clusters --
    --cards "$ROOT_DIR/cards.json"
    --use-embeddings
    --embedding-dims "$DIMS"
    --embedding-threshold "$THRESHOLD"
    --min-cluster-size 1
    --top-clusters 0
    --examples 1
    --audits-out "$SCORES_FILE"
  )
  if [[ -f "$FALSE_POSITIVES_FILE" ]]; then
    AUDIT_CMD+=(--false-positive-names "$FALSE_POSITIVES_FILE")
  fi
  "${AUDIT_CMD[@]}"
else
  if [[ ! -f "$SCORES_FILE" ]]; then
    cat >&2 <<EOF
[ERROR] semantic scores file not found: $SCORES_FILE

Run once with --threshold to generate it, for example:
  ./rebuild-wasm.sh --threshold 0.99

Or pass an existing file:
  ./rebuild-wasm.sh --scores-file /path/to/ironsmith_semantic_scores.json
EOF
    exit 1
  fi
  echo "[INFO] reusing semantic scores file: $SCORES_FILE"
fi

if [[ "$SCORES_FILE" != "$FRONTEND_SCORES_FILE" ]]; then
  mkdir -p "$(dirname "$FRONTEND_SCORES_FILE")"
  cp -f "$SCORES_FILE" "$FRONTEND_SCORES_FILE"
  echo "[INFO] synced semantic scores cache for frontend: $FRONTEND_SCORES_FILE"
fi

export IRONSMITH_GENERATED_REGISTRY_SCORES_FILE="$SCORES_FILE"
echo "[INFO] semantic scores source: $IRONSMITH_GENERATED_REGISTRY_SCORES_FILE"
echo "[INFO] wasm build profile: $BUILD_PROFILE"

WASM_PACK_ARGS=(build --target web)
if [[ "$BUILD_PROFILE" == "release" ]]; then
  WASM_PACK_ARGS+=(--release)
else
  WASM_PACK_ARGS+=(--dev)
fi
WASM_PACK_ARGS+=(--features "$FEATURES")

wasm-pack "${WASM_PACK_ARGS[@]}"

mkdir -p "$DEMO_PKG_DIR"
cp -f \
  "$PKG_DIR/ironsmith.js" \
  "$PKG_DIR/ironsmith_bg.wasm" \
  "$PKG_DIR/ironsmith.d.ts" \
  "$PKG_DIR/ironsmith_bg.wasm.d.ts" \
  "$PKG_DIR/package.json" \
  "$DEMO_PKG_DIR/"
