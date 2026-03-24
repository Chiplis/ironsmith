#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$ROOT_DIR/pkg"
DEMO_PKG_DIR="$ROOT_DIR/web/wasm_demo/pkg"
DEFAULT_DB_PATH="$ROOT_DIR/reports/engine-status.sqlite3"
DEFAULT_INTERNAL_SCORES_FILE="$ROOT_DIR/reports/ironsmith_semantic_scores.json"
DEFAULT_FRONTEND_SCORES_FILE="$ROOT_DIR/web/ui/public/ironsmith_semantic_scores.json"
DEFAULT_CLUSTER_CSV_FILE="$ROOT_DIR/reports/ironsmith_parse_failure_clusters.csv"
DEFAULT_PARSE_ERRORS_CSV_FILE="$ROOT_DIR/reports/ironsmith_parse_errors.csv"
DEFAULT_PARSE_ERROR_SUMMARY_CSV_FILE="$ROOT_DIR/reports/ironsmith_parse_error_summary.csv"

DIMS="${IRONSMITH_WASM_SEMANTIC_DIMS:-384}"
FEATURES="wasm,generated-registry"
THRESHOLD="${IRONSMITH_WASM_SEMANTIC_THRESHOLD:-}"
OPTIMIZE_WASM=0
DB_PATH="${IRONSMITH_REGISTRY_DB_PATH:-$DEFAULT_DB_PATH}"
FRONTEND_SCORES_FILE="${IRONSMITH_FRONTEND_SEMANTIC_SCORES_FILE:-$DEFAULT_FRONTEND_SCORES_FILE}"
FRONTEND_SCORES_FILE_EXPLICIT=0
SCORES_FILE="${IRONSMITH_GENERATED_REGISTRY_SCORES_FILE:-$DEFAULT_INTERNAL_SCORES_FILE}"
SCORES_FILE_EXPLICIT=0
CLUSTER_CSV_FILE="${IRONSMITH_CLUSTER_CSV_FILE:-$DEFAULT_CLUSTER_CSV_FILE}"
PARSE_ERRORS_CSV_FILE="${IRONSMITH_PARSE_ERRORS_CSV_FILE:-$DEFAULT_PARSE_ERRORS_CSV_FILE}"
PARSE_ERROR_SUMMARY_CSV_FILE="${IRONSMITH_PARSE_ERROR_SUMMARY_CSV_FILE:-$DEFAULT_PARSE_ERROR_SUMMARY_CSV_FILE}"

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

feature_enabled() {
  local normalized
  normalized="$(printf '%s' "$FEATURES" | tr -d '[:space:]')"
  [[ ",$normalized," == *",$1,"* ]]
}

usage() {
  cat <<USAGE
Usage: ./rebuild-wasm.sh [--threshold <float>] [--dims <int>] [--features <csv>] [--scores-file <path>] [--frontend-scores-file <path>] [--cluster-csv-file <path>] [--parse-errors-csv-file <path>] [--parse-error-summary-csv-file <path>]

Examples:
  ./rebuild-wasm.sh
  ./rebuild-wasm.sh --release
  ./rebuild-wasm.sh --threshold 0.99
  ./rebuild-wasm.sh --dims 384
  ./rebuild-wasm.sh --scores-file /tmp/ironsmith_semantic_scores.json
  ./rebuild-wasm.sh --frontend-scores-file web/ui/public/ironsmith_semantic_scores.json
  ./rebuild-wasm.sh --cluster-csv-file reports/ironsmith_parse_failure_clusters.csv
  ./rebuild-wasm.sh --parse-errors-csv-file reports/ironsmith_parse_errors.csv
  ./rebuild-wasm.sh --parse-error-summary-csv-file reports/ironsmith_parse_error_summary.csv

Notes:
  - Cargo always builds the WASM crate in release mode.
  - wasm-opt is skipped by default for faster iteration; pass --release to enable it.
  - Canonical card data is loaded from the registry SQLite DB (default: $DEFAULT_DB_PATH).
  - Per-card semantic scores for the generated registry are loaded from --scores-file.
  - Frontend cache file defaults to $DEFAULT_FRONTEND_SCORES_FILE and stores only compact threshold stats.
  - Cluster and parse-error CSVs are refreshed only when --threshold is provided.
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
      OPTIMIZE_WASM=0
      shift
      ;;
    --release)
      OPTIMIZE_WASM=1
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
    --cluster-csv-file)
      [[ $# -ge 2 ]] || { echo "missing value for --cluster-csv-file" >&2; exit 1; }
      CLUSTER_CSV_FILE="$2"
      shift 2
      ;;
    --parse-errors-csv-file)
      [[ $# -ge 2 ]] || { echo "missing value for --parse-errors-csv-file" >&2; exit 1; }
      PARSE_ERRORS_CSV_FILE="$2"
      shift 2
      ;;
    --parse-error-summary-csv-file)
      [[ $# -ge 2 ]] || { echo "missing value for --parse-error-summary-csv-file" >&2; exit 1; }
      PARSE_ERROR_SUMMARY_CSV_FILE="$2"
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

if [[ -n "$THRESHOLD" ]] || feature_enabled "generated-registry"; then
  if [[ ! -f "$DB_PATH" ]]; then
    cat >&2 <<EOF
[ERROR] registry DB not found: $DB_PATH

Run the registry sync first, for example:
  cargo run --release -p ironsmith-tools --bin sync_registry_db -- --cards cards.json --db-path $DB_PATH
EOF
    exit 1
  fi
fi

if [[ -n "$THRESHOLD" ]]; then
  mkdir -p "$(dirname "$SCORES_FILE")"
  mkdir -p "$(dirname "$CLUSTER_CSV_FILE")"
  mkdir -p "$(dirname "$PARSE_ERRORS_CSV_FILE")"
  mkdir -p "$(dirname "$PARSE_ERROR_SUMMARY_CSV_FILE")"
  echo "[INFO] computing semantic audits report (dims=${DIMS}, threshold=${THRESHOLD})..."
  AUDIT_CMD=(
    cargo run --quiet --release -p ironsmith-tools --bin audit_oracle_clusters --
    --db-path "$DB_PATH"
    --use-embeddings
    --embedding-dims "$DIMS"
    --embedding-threshold "$THRESHOLD"
    --min-cluster-size 1
    --top-clusters 0
    --examples 1
    --audits-out "$SCORES_FILE"
    --cluster-csv-out "$CLUSTER_CSV_FILE"
    --parse-errors-csv-out "$PARSE_ERRORS_CSV_FILE"
    --parse-error-summary-csv-out "$PARSE_ERROR_SUMMARY_CSV_FILE"
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

mkdir -p "$(dirname "$FRONTEND_SCORES_FILE")"
python3 - "$SCORES_FILE" "$FRONTEND_SCORES_FILE" <<'PY'
import json
import math
import sys
from pathlib import Path

source = Path(sys.argv[1])
target = Path(sys.argv[2])
payload = json.loads(source.read_text(encoding="utf-8"))

if isinstance(payload, dict) and isinstance(payload.get("thresholdCounts"), list) and "scoredCount" in payload:
    summary = {
        "scoredCount": int(payload["scoredCount"]),
        "thresholdCounts": [int(v) for v in payload["thresholdCounts"]],
    }
else:
    scores_by_name = {}

    def maybe_insert(raw_name, raw_score):
        if not isinstance(raw_name, str):
            return
        try:
            score = float(raw_score)
        except (TypeError, ValueError):
            return
        score = max(0.0, min(1.0, score))
        name = raw_name.strip().lower()
        if not name:
            return
        prev = scores_by_name.get(name)
        if prev is None or score > prev:
            scores_by_name[name] = score

    if isinstance(payload, dict) and isinstance(payload.get("entries"), list):
        for entry in payload["entries"]:
            if not isinstance(entry, dict):
                continue
            if entry.get("parse_error") is not None:
                continue
            if bool(entry.get("has_unimplemented", False)):
                continue
            maybe_insert(entry.get("name"), entry.get("similarity_score"))
    elif isinstance(payload, dict):
        for name, score in payload.items():
            maybe_insert(name, score)
    elif isinstance(payload, list):
        for entry in payload:
            if not isinstance(entry, dict):
                continue
            maybe_insert(entry.get("name"), entry.get("similarity_score"))

    threshold_counts = [0] * 100
    for score in scores_by_name.values():
        thresholds_met = int(math.floor(score * 100))
        for idx in range(thresholds_met):
            threshold_counts[idx] += 1

    summary = {
        "scoredCount": len(scores_by_name),
        "thresholdCounts": threshold_counts,
    }

target.write_text(json.dumps(summary, separators=(",", ":")), encoding="utf-8")
PY
echo "[INFO] synced semantic threshold cache for frontend: $FRONTEND_SCORES_FILE"

export IRONSMITH_GENERATED_REGISTRY_SCORES_FILE="$SCORES_FILE"
export IRONSMITH_REGISTRY_DB_PATH="$DB_PATH"
echo "[INFO] semantic scores source: $IRONSMITH_GENERATED_REGISTRY_SCORES_FILE"
echo "[INFO] registry DB source: $IRONSMITH_REGISTRY_DB_PATH"
echo "[INFO] wasm build profile: release"
if [[ "$OPTIMIZE_WASM" -eq 1 ]]; then
  echo "[INFO] wasm-opt: enabled"
else
  echo "[INFO] wasm-opt: disabled (--no-opt)"
fi

WASM_PACK_ARGS=(build --target web --release)
if [[ "$OPTIMIZE_WASM" -eq 0 ]]; then
  WASM_PACK_ARGS+=(--no-opt)
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
