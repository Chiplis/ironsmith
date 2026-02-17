#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$ROOT_DIR/pkg"
DEMO_PKG_DIR="$ROOT_DIR/web/wasm_demo/pkg"
FALSE_POSITIVES_FILE="$ROOT_DIR/scripts/semantic_false_positives.txt"
PREFERRED_REPORTS_DIR="/reports"
REPORTS_DIR="$PREFERRED_REPORTS_DIR"

# Prefer the parser threshold env var used by the audit tooling, then fall back
# to the old wasm-specific var name.
THRESHOLD="${IRONSMITH_PARSER_SEMANTIC_THRESHOLD:-${IRONSMITH_WASM_SEMANTIC_THRESHOLD:-}}"
DIMS="${IRONSMITH_WASM_SEMANTIC_DIMS:-384}"
MIN_CLUSTER_SIZE="${IRONSMITH_WASM_MIN_CLUSTER_SIZE:-1}"
FEATURES="wasm,generated-registry"

usage() {
  cat <<'USAGE'
Usage: ./rebuild-wasm.sh [--threshold <float>] [--dims <int>] [--min-cluster-size <int>] [--features <csv>]

Examples:
  ./rebuild-wasm.sh
  ./rebuild-wasm.sh --threshold 0.90
  IRONSMITH_PARSER_SEMANTIC_THRESHOLD=0.85 ./rebuild-wasm.sh
  ./rebuild-wasm.sh --min-cluster-size 1

Notes:
  - --threshold enables semantic gating for generated-registry builds.
    Cards below the threshold are excluded from the generated registry.
  - Parse failures are still excluded independently of threshold gating.
  - Use the same threshold in audit_oracle_clusters to compare counts.
  - Default features are "wasm,generated-registry".
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --threshold)
      [[ $# -ge 2 ]] || { echo "missing value for --threshold" >&2; exit 1; }
      THRESHOLD="$2"
      shift 2
      ;;
    --dims)
      [[ $# -ge 2 ]] || { echo "missing value for --dims" >&2; exit 1; }
      DIMS="$2"
      shift 2
      ;;
    --min-cluster-size)
      [[ $# -ge 2 ]] || { echo "missing value for --min-cluster-size" >&2; exit 1; }
      MIN_CLUSTER_SIZE="$2"
      shift 2
      ;;
    --features)
      [[ $# -ge 2 ]] || { echo "missing value for --features" >&2; exit 1; }
      FEATURES="$2"
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

if [[ -n "$THRESHOLD" ]]; then
  if ! mkdir -p "$REPORTS_DIR" 2>/dev/null; then
    REPORTS_DIR="$ROOT_DIR/reports"
    mkdir -p "$REPORTS_DIR"
    echo "[WARN] fallback to $REPORTS_DIR (could not create $PREFERRED_REPORTS_DIR)"
  fi

  TIMESTAMP="$(date -u +'%Y%m%dT%H%M%SZ')"
  SAFE_THRESHOLD="${THRESHOLD//./_}"
  RUN_ID="${SAFE_THRESHOLD}_${DIMS}_${TIMESTAMP}"
  MISMATCH_NAMES_FILE="${TMPDIR:-/tmp}/ironsmith_wasm_mismatch_names_${RUN_ID}.txt"
  FAILURES_REPORT="${TMPDIR:-/tmp}/ironsmith_wasm_threshold_failures_${RUN_ID}.json"
  CLUSTER_REPORT="${TMPDIR:-/tmp}/ironsmith_wasm_cluster_report_${RUN_ID}.json"
  MISMATCH_REPORT="$REPORTS_DIR/ironsmith_wasm_semantic_mismatch_report_${RUN_ID}.json"
  UNPARSABLE_REPORT="$REPORTS_DIR/ironsmith_wasm_unparsable_clusters_${RUN_ID}.json"

  echo "[INFO] computing semantic threshold failures (threshold=${THRESHOLD}, dims=${DIMS})..."
  AUDIT_CMD=(
    cargo run --quiet --bin audit_oracle_clusters --
    --cards "$ROOT_DIR/cards.json"
    --use-embeddings
    --embedding-dims "$DIMS"
    --embedding-threshold "$THRESHOLD"
    --min-cluster-size "$MIN_CLUSTER_SIZE"
    --top-clusters 20000
    --examples 1
    --mismatch-names-out "$MISMATCH_NAMES_FILE"
    --failures-out "$FAILURES_REPORT"
    --json-out "$CLUSTER_REPORT"
  )
  if [[ -f "$FALSE_POSITIVES_FILE" ]]; then
    AUDIT_CMD+=(--false-positive-names "$FALSE_POSITIVES_FILE")
  fi
  "${AUDIT_CMD[@]}"

  EXCLUDED_COUNT="$(rg -cve '^\\s*$' "$MISMATCH_NAMES_FILE" 2>/dev/null || true)"
  export IRONSMITH_GENERATED_REGISTRY_SKIP_NAMES_FILE="$MISMATCH_NAMES_FILE"
  echo "[INFO] semantic gating active: excluding ${EXCLUDED_COUNT} below-threshold card(s)"
  cp -f "$FAILURES_REPORT" "$MISMATCH_REPORT"
  jq --arg ts "$TIMESTAMP" --arg threshold "$THRESHOLD" --arg dims "$DIMS" '
    {
      generated_at: $ts,
      threshold: $threshold,
      embedding_dims: $dims,
      cards_processed: .cards_processed,
      semantic_failures_report: .failures,
      parse_failures: .parse_failures,
      unparsable_clusters_by_error: (
        .clusters
        | map(
          select(.parse_failures > 0)
          | {
              error: (.top_errors[0].error // "unclassified"),
              cluster: {
                signature: .signature,
                size: .size,
                parse_failures: .parse_failures,
                parse_failure_rate: .parse_failure_rate,
                top_errors: .top_errors,
                examples: .examples
              }
            }
        )
        | if length == 0 then
            {}
          else
            sort_by(.error)
            | group_by(.error)
            | map(
                {
                  (.[0].error): {
                    cluster_count: length,
                    total_parse_failures: (map(.cluster.parse_failures) | add),
                    clusters: (map(.cluster))
                  }
                }
              )
            | add
          end
      )
    }
  ' "$CLUSTER_REPORT" > "$UNPARSABLE_REPORT"
  echo "[INFO] semantic mismatch report: $MISMATCH_REPORT"
  echo "[INFO] unparsable cluster report: $UNPARSABLE_REPORT"
else
  unset IRONSMITH_GENERATED_REGISTRY_SKIP_NAMES_FILE
fi

wasm-pack build --release --target web --features "$FEATURES"

mkdir -p "$DEMO_PKG_DIR"
cp -f \
  "$PKG_DIR/ironsmith.js" \
  "$PKG_DIR/ironsmith_bg.wasm" \
  "$PKG_DIR/ironsmith.d.ts" \
  "$PKG_DIR/ironsmith_bg.wasm.d.ts" \
  "$PKG_DIR/package.json" \
  "$DEMO_PKG_DIR/"
