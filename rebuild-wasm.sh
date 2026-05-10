#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WASM_CRATE_DIR="$ROOT_DIR/crates/ironsmith-wasm"
PKG_DIR="$ROOT_DIR/pkg"
DEMO_PKG_DIR="$ROOT_DIR/web/wasm_demo/pkg"
DEFAULT_DB_PATH="$ROOT_DIR/reports/engine-status.sqlite3"
DEFAULT_FRONTEND_SCORES_FILE="$ROOT_DIR/web/ui/public/ironsmith_semantic_scores.json"

FEATURES="wasm,generated-registry"
OPTIMIZE_WASM=0
DB_PATH="${IRONSMITH_REGISTRY_DB_PATH:-$DEFAULT_DB_PATH}"
FRONTEND_SCORES_FILE="${IRONSMITH_FRONTEND_SEMANTIC_SCORES_FILE:-$DEFAULT_FRONTEND_SCORES_FILE}"

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
Usage: ./rebuild-wasm.sh [--features <csv>] [--frontend-scores-file <path>]

Examples:
  ./rebuild-wasm.sh
  ./rebuild-wasm.sh --release
  ./rebuild-wasm.sh --frontend-scores-file web/ui/public/ironsmith_semantic_scores.json

Notes:
  - Cargo always builds the WASM crate in release mode.
  - wasm-opt is skipped by default for faster iteration; pass --release to enable it.
  - Canonical card data and per-card semantic scores are loaded from the registry SQLite DB (default: $DEFAULT_DB_PATH).
  - Run sync_card_status_db separately when latest_card_compilation needs to be refreshed.
  - Frontend cache file defaults to $DEFAULT_FRONTEND_SCORES_FILE and stores only compact threshold stats.
  - Default features are "wasm,generated-registry".
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --features)
      [[ $# -ge 2 ]] || { echo "missing value for --features" >&2; exit 1; }
      FEATURES="$2"
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
    --frontend-scores-file)
      [[ $# -ge 2 ]] || { echo "missing value for --frontend-scores-file" >&2; exit 1; }
      FRONTEND_SCORES_FILE="$2"
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

if feature_enabled "generated-registry"; then
  if [[ ! -f "$DB_PATH" ]]; then
    cat >&2 <<EOF
[ERROR] registry DB not found: $DB_PATH

Run the registry sync first, for example:
  cargo run --release -p ironsmith-tools --bin sync_registry_db -- --cards cards.json --db-path $DB_PATH
EOF
    exit 1
  fi

  echo "[INFO] using latest strict card compilation snapshots from DB..."

  mkdir -p "$(dirname "$FRONTEND_SCORES_FILE")"
  python3 - "$DB_PATH" "$FRONTEND_SCORES_FILE" <<'PY'
import json
import sqlite3
import sys
from pathlib import Path

db_path = Path(sys.argv[1])
target = Path(sys.argv[2])

conn = sqlite3.connect(db_path)
try:
    rows = conn.execute(
        """
        SELECT card_name, similarity_score
        FROM latest_card_compilation
        WHERE parse_status = 'strict_compiled'
          AND parse_error IS NULL
          AND has_unimplemented = 0
          AND normalized_oracle_text IS NOT NULL
          AND compiled_text IS NOT NULL
        """
    )
    scores_by_name = {}
    for raw_name, raw_score in rows:
        name = str(raw_name).strip().lower()
        if not name:
            continue
        score = max(0.0, min(1.0, float(raw_score)))
        previous = scores_by_name.get(name)
        if previous is None or score > previous:
            scores_by_name[name] = score
finally:
    conn.close()

threshold_counts = [0] * 100
for score in scores_by_name.values():
    for idx in range(100):
        threshold = (idx + 1) / 100.0
        if score >= threshold:
            threshold_counts[idx] += 1

summary = {
    "scoredCount": len(scores_by_name),
    "thresholdCounts": threshold_counts,
}

target.write_text(json.dumps(summary, separators=(",", ":")), encoding="utf-8")
PY
  echo "[INFO] synced semantic threshold cache for frontend: $FRONTEND_SCORES_FILE"

  export IRONSMITH_REGISTRY_DB_PATH="$DB_PATH"
  echo "[INFO] registry DB source: $IRONSMITH_REGISTRY_DB_PATH"
else
  echo "[INFO] generated registry disabled; skipping DB-backed semantic score cache"
fi
echo "[INFO] wasm build profile: release"
if [[ "$OPTIMIZE_WASM" -eq 1 ]]; then
  echo "[INFO] wasm-opt: enabled"
else
  echo "[INFO] wasm-opt: disabled (--no-opt)"
fi

WASM_OUT_DIR="$(
  python3 - "$PKG_DIR" "$WASM_CRATE_DIR" <<'PY'
import os
import sys

print(os.path.relpath(sys.argv[1], sys.argv[2]))
PY
)"

WASM_PACK_ARGS=(
  build "$WASM_CRATE_DIR"
  --target web
  --release
  --out-dir "$WASM_OUT_DIR"
  --out-name ironsmith
)
if [[ "$OPTIMIZE_WASM" -eq 0 ]]; then
  WASM_PACK_ARGS+=(--no-opt)
fi
WASM_PACK_ARGS+=(--features "$FEATURES")

find_cached_wasm_bindgen() {
  local cache_root="${WASM_PACK_CACHE:-$HOME/Library/Caches/.wasm-pack}"
  local candidate
  local required_version
  if [[ ! -d "$cache_root" ]]; then
    return 1
  fi
  required_version="$(
    awk '
      /^name = "wasm-bindgen"$/ { found = 1; next }
      found && /^version = / {
        gsub(/"/, "", $3)
        print $3
        exit
      }
    ' "$ROOT_DIR/Cargo.lock"
  )"
  while IFS= read -r candidate; do
    if [[ -x "$candidate" && -n "$required_version" ]] \
      && "$candidate" --version 2>/dev/null | grep -q "wasm-bindgen $required_version"; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done < <(find "$cache_root" -maxdepth 2 -type f -name wasm-bindgen | sort -r)
  while IFS= read -r candidate; do
    if [[ -x "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done < <(find "$cache_root" -maxdepth 2 -type f -name wasm-bindgen | sort -r)
  return 1
}

find_cached_wasm_opt() {
  local cache_root="${WASM_PACK_CACHE:-$HOME/Library/Caches/.wasm-pack}"
  local candidate
  if [[ ! -d "$cache_root" ]]; then
    return 1
  fi
  while IFS= read -r candidate; do
    if [[ -x "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done < <(find "$cache_root" -maxdepth 4 -type f -path '*/bin/wasm-opt' | sort -r)
  return 1
}

build_wasm_with_cached_bindgen() {
  local bindgen
  local wasm_opt
  bindgen="$(find_cached_wasm_bindgen)" || {
    echo "[ERROR] wasm-pack failed and no cached wasm-bindgen binary was found" >&2
    return 1
  }

  echo "[WARN] wasm-pack failed; falling back to cargo build with cached wasm-bindgen: $bindgen" >&2
  cargo build \
    -p ironsmith-wasm \
    --target wasm32-unknown-unknown \
    --release \
    --features "$FEATURES"

  mkdir -p "$PKG_DIR"
  "$bindgen" \
    "$ROOT_DIR/target/wasm32-unknown-unknown/release/ironsmith_wasm.wasm" \
    --target web \
    --out-dir "$PKG_DIR" \
    --out-name ironsmith

  if [[ "$OPTIMIZE_WASM" -eq 1 ]]; then
    if wasm_opt="$(find_cached_wasm_opt)"; then
      echo "[INFO] optimizing fallback WASM with cached wasm-opt: $wasm_opt"
      "$wasm_opt" -Oz "$PKG_DIR/ironsmith_bg.wasm" -o "$PKG_DIR/ironsmith_bg.wasm"
    else
      echo "[WARN] wasm-pack failed and no cached wasm-opt binary was found; fallback WASM is unoptimized" >&2
    fi
  fi

  if [[ -f "$DEMO_PKG_DIR/package.json" ]]; then
    cp -f "$DEMO_PKG_DIR/package.json" "$PKG_DIR/package.json"
  elif [[ ! -f "$PKG_DIR/package.json" ]]; then
    cat > "$PKG_DIR/package.json" <<'JSON'
{"name":"ironsmith","type":"module","files":["ironsmith.js","ironsmith_bg.wasm","ironsmith.d.ts","ironsmith_bg.wasm.d.ts"]}
JSON
  fi
}

if ! wasm-pack "${WASM_PACK_ARGS[@]}"; then
  build_wasm_with_cached_bindgen
fi

mkdir -p "$DEMO_PKG_DIR"
cp -f \
  "$PKG_DIR/ironsmith.js" \
  "$PKG_DIR/ironsmith_bg.wasm" \
  "$PKG_DIR/ironsmith.d.ts" \
  "$PKG_DIR/ironsmith_bg.wasm.d.ts" \
  "$PKG_DIR/package.json" \
  "$DEMO_PKG_DIR/"
