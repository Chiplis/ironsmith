#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$ROOT_DIR/pkg"
DEMO_PKG_DIR="$ROOT_DIR/web/wasm_demo/pkg"
DEFAULT_CARDS_FILE="$ROOT_DIR/cards.json"
DEFAULT_DB_PATH="$ROOT_DIR/reports/engine-status.sqlite3"
DEFAULT_FRONTEND_SCORES_FILE="$ROOT_DIR/web/ui/public/ironsmith_semantic_scores.json"
DEFAULT_FRONTEND_CARDS_DIR="$ROOT_DIR/web/ui/public/cards"
FRONTEND_CARDS_CACHE_MANIFEST_NAME=".ironsmith_frontend_cards_checksum"

FEATURES="wasm-lean"
OPTIMIZE_WASM=0
CARDS_FILE="${IRONSMITH_CARDS_FILE:-$DEFAULT_CARDS_FILE}"
if [[ -n "${IRONSMITH_SCRYFALL_METADATA_FILE:-}" ]]; then
  SCRYFALL_METADATA_FILE="$IRONSMITH_SCRYFALL_METADATA_FILE"
  SCRYFALL_METADATA_FILE_SET=1
else
  SCRYFALL_METADATA_FILE="${CARDS_FILE}.scryfall-bulk-data.json"
  SCRYFALL_METADATA_FILE_SET=0
fi
DB_PATH="${IRONSMITH_REGISTRY_DB_PATH:-$DEFAULT_DB_PATH}"
FRONTEND_SCORES_FILE="${IRONSMITH_FRONTEND_SEMANTIC_SCORES_FILE:-$DEFAULT_FRONTEND_SCORES_FILE}"
FRONTEND_CARDS_DIR="${IRONSMITH_FRONTEND_CARDS_DIR:-$DEFAULT_FRONTEND_CARDS_DIR}"
SYNC_SCRYFALL_CARDS="${IRONSMITH_SYNC_SCRYFALL_CARDS:-1}"
NO_DEFAULT_FEATURES=1
WASM_OPT_LEVEL="${IRONSMITH_WASM_OPT_LEVEL:--O1}"

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
Usage: ./rebuild-wasm.sh [--features <csv>] [--cards-file <path>] [--scryfall-metadata-file <path>] [--frontend-scores-file <path>] [--frontend-cards-dir <path>] [--skip-scryfall-sync]

Examples:
  ./rebuild-wasm.sh
  ./rebuild-wasm.sh --release
  ./rebuild-wasm.sh --skip-scryfall-sync
  ./rebuild-wasm.sh --frontend-scores-file web/ui/public/ironsmith_semantic_scores.json
  ./rebuild-wasm.sh --features wasm,generated-registry --default-features

Notes:
  - Cargo always builds the WASM crate in release mode.
  - wasm-opt is skipped by default for faster iteration; pass --release to enable it.
  - Scryfall Default Cards are downloaded to $DEFAULT_CARDS_FILE when Scryfall publishes a newer bulk-data updated_at.
  - Registry DB rows are inserted only for cards not already present; existing registry cards are not updated or pruned during this rebuild preflight.
  - Cards without compilation status rows are compiled before frontend assets are generated.
  - Canonical card data and per-card semantic scores are loaded from the registry SQLite DB (default: $DEFAULT_DB_PATH).
  - Frontend cache file defaults to $DEFAULT_FRONTEND_SCORES_FILE and stores only compact threshold stats.
  - Frontend card assets default to $DEFAULT_FRONTEND_CARDS_DIR and are copied by Vite into dist/cards/.
  - Frontend JSON assets are skipped when their checksum manifest matches the registry DB and generator inputs.
  - Default features are "wasm-lean" with crate default features disabled, so card source data is loaded from dist/cards/ instead of being embedded in engine_bg.wasm.
  - The package contains separate engine, compiler, and verifier modules behind one JavaScript facade.
  - IRONSMITH_WASM_OPT_LEVEL selects the shipped optimizer level (-O1, -O2, -Os, or -Oz; default -O1).
USAGE
}

write_frontend_cards_cache_manifest() {
  local target="$1"
  python3 - "$ROOT_DIR" "$DB_PATH" "$target" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
db_path = Path(sys.argv[2]).resolve()
target = Path(sys.argv[3])


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


inputs = [
    ("rebuild-wasm.sh", root / "rebuild-wasm.sh"),
    ("scripts/generate_baked_registry.py", root / "scripts" / "generate_baked_registry.py"),
    ("scripts/stream_scryfall_blocks.py", root / "scripts" / "stream_scryfall_blocks.py"),
    ("artifact-baker", root / "crates" / "ironsmith-artifact-baker" / "src" / "main.rs"),
    ("registry-db", db_path),
]

payload = {
    "schemaVersion": 1,
    "inputs": [
        {
            "label": label,
            "path": str(path.relative_to(root) if path.is_relative_to(root) else path),
            "sha256": file_sha256(path),
        }
        for label, path in inputs
    ],
}
checksum_payload = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
payload["checksum"] = hashlib.sha256(checksum_payload).hexdigest()
target.write_text(json.dumps(payload, sort_keys=True, separators=(",", ":")), encoding="utf-8")
PY
}

frontend_cards_cache_matches() {
  local expected_manifest="$1"
  local existing_manifest="$2"
  local cards_dir="$3"
  local scores_file="$4"
  python3 - "$expected_manifest" "$existing_manifest" "$cards_dir" "$scores_file" <<'PY'
import json
import sys
from pathlib import Path

expected_path = Path(sys.argv[1])
existing_path = Path(sys.argv[2])
cards_dir = Path(sys.argv[3])
scores_file = Path(sys.argv[4])

try:
    expected = json.loads(expected_path.read_text(encoding="utf-8"))
    existing = json.loads(existing_path.read_text(encoding="utf-8"))
except (FileNotFoundError, json.JSONDecodeError):
    sys.exit(1)

if expected.get("checksum") != existing.get("checksum"):
    sys.exit(1)

index_path = cards_dir / "index.json"
if not index_path.is_file():
    sys.exit(1)
if not scores_file.is_file():
    sys.exit(1)

try:
    index = json.loads(index_path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError):
    sys.exit(1)

cards = index.get("cards")
if not isinstance(cards, list):
    sys.exit(1)

expected_route_count = index.get("routeCount")
if not isinstance(expected_route_count, int) or expected_route_count < 0:
    sys.exit(1)

route_files = [
    path
    for path in cards_dir.glob("*.json")
    if path.name != "index.json"
]

reserved_manifest_routes = {"index"}
reserved_route_count = sum(
    1
    for card in cards
    if isinstance(card, dict)
    and str(card.get("route") or "").strip() in reserved_manifest_routes
)
if len(route_files) != expected_route_count - reserved_route_count:
    sys.exit(1)

for card in cards:
    if not isinstance(card, dict):
        sys.exit(1)
    route = str(card.get("route") or "").strip()
    if not route:
        sys.exit(1)
    if route in reserved_manifest_routes:
        continue
    if not (cards_dir / f"{route}.json").is_file():
        sys.exit(1)

sys.exit(0)
PY
}

scryfall_sync_enabled() {
  case "$SYNC_SCRYFALL_CARDS" in
    1|true|TRUE|yes|YES|on|ON)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

count_lines() {
  local path="$1"
  python3 - "$path" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
try:
    print(sum(1 for line in path.read_text(encoding="utf-8").splitlines() if line.strip()))
except FileNotFoundError:
    print(0)
PY
}

write_registry_cards_missing_status_rows() {
  local target="$1"
  python3 - "$DB_PATH" "$target" <<'PY'
import sqlite3
import sys
from pathlib import Path

db_path = Path(sys.argv[1])
target = Path(sys.argv[2])

conn = sqlite3.connect(db_path)
try:
    names = [
        row[0]
        for row in conn.execute(
            """
            SELECT registry.card_name
            FROM registry_card registry
            LEFT JOIN latest_card_compilation latest
              ON latest.card_name = registry.card_name
            WHERE latest.card_name IS NULL
            ORDER BY registry.card_name COLLATE NOCASE ASC
            """
        )
    ]
finally:
    conn.close()

target.write_text("".join(f"{name}\n" for name in names), encoding="utf-8")
PY
}

sync_scryfall_cards_for_rebuild() {
  local download_status_file
  local inserted_names_file
  local missing_status_names_file
  local download_status
  local inserted_count
  local missing_status_count

  if ! scryfall_sync_enabled; then
    echo "[INFO] skipped Scryfall card sync by request"
    return 0
  fi

  download_status_file="$(mktemp "$ROOT_DIR/target/scryfall-download-status.XXXXXX")"
  inserted_names_file="$(mktemp "$ROOT_DIR/target/scryfall-inserted-card-names.XXXXXX")"
  missing_status_names_file="$(mktemp "$ROOT_DIR/target/registry-cards-missing-status.XXXXXX")"

  echo "[INFO] checking Scryfall Default Cards freshness..."
  python3 "$ROOT_DIR/scripts/download_scryfall_cards.py" \
    --out "$CARDS_FILE" \
    --metadata-out "$SCRYFALL_METADATA_FILE" \
    --status-out "$download_status_file"

  download_status="unknown"
  if [[ -s "$download_status_file" ]]; then
    IFS= read -r download_status < "$download_status_file"
  fi
  case "$download_status" in
    downloaded)
      echo "[INFO] downloaded latest Scryfall card list: $CARDS_FILE"
      ;;
    skipped)
      echo "[INFO] Scryfall card list already current: $CARDS_FILE"
      ;;
    *)
      echo "[INFO] Scryfall card list status: $download_status"
      ;;
  esac

  echo "[INFO] syncing registry DB with cards not already present..."
  cargo run --release -p ironsmith-registry-sync --bin sync_registry_db -- \
    --cards "$CARDS_FILE" \
    --db-path "$DB_PATH" \
    --insert-missing-only \
    --inserted-names-out "$inserted_names_file"

  inserted_count="$(count_lines "$inserted_names_file")"
  if [[ "$inserted_count" -gt 0 ]]; then
    echo "[INFO] registered $inserted_count new card(s) in the DB"
  else
    echo "[INFO] no new registry cards found"
  fi

  write_registry_cards_missing_status_rows "$missing_status_names_file"
  missing_status_count="$(count_lines "$missing_status_names_file")"
  if [[ "$missing_status_count" -gt 0 ]]; then
    echo "[INFO] compiling semantic snapshots for $missing_status_count card(s) without status rows..."
    cargo run --release -p ironsmith-tools --bin sync_card_status_db -- \
      --db-path "$DB_PATH" \
      --names-file "$missing_status_names_file"
  else
    echo "[INFO] all registry cards already have compilation status rows"
  fi

  rm -f "$download_status_file" "$inserted_names_file" "$missing_status_names_file"
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
    --cards-file)
      [[ $# -ge 2 ]] || { echo "missing value for --cards-file" >&2; exit 1; }
      CARDS_FILE="$2"
      if [[ "$SCRYFALL_METADATA_FILE_SET" -eq 0 ]]; then
        SCRYFALL_METADATA_FILE="${CARDS_FILE}.scryfall-bulk-data.json"
      fi
      shift 2
      ;;
    --scryfall-metadata-file)
      [[ $# -ge 2 ]] || { echo "missing value for --scryfall-metadata-file" >&2; exit 1; }
      SCRYFALL_METADATA_FILE="$2"
      SCRYFALL_METADATA_FILE_SET=1
      shift 2
      ;;
    --frontend-scores-file)
      [[ $# -ge 2 ]] || { echo "missing value for --frontend-scores-file" >&2; exit 1; }
      FRONTEND_SCORES_FILE="$2"
      shift 2
      ;;
    --frontend-cards-dir)
      [[ $# -ge 2 ]] || { echo "missing value for --frontend-cards-dir" >&2; exit 1; }
      FRONTEND_CARDS_DIR="$2"
      shift 2
      ;;
    --default-features)
      NO_DEFAULT_FEATURES=0
      shift
      ;;
    --no-default-features)
      NO_DEFAULT_FEATURES=1
      shift
      ;;
    --skip-scryfall-sync)
      SYNC_SCRYFALL_CARDS=0
      shift
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

case "$WASM_OPT_LEVEL" in
  -O1|-O2|-Os|-Oz) ;;
  *) echo "IRONSMITH_WASM_OPT_LEVEL must be -O1, -O2, -Os, or -Oz" >&2; exit 1 ;;
esac

cd "$ROOT_DIR"
require_cmd cargo
require_cmd wasm-bindgen
require_cmd python3

mkdir -p "$ROOT_DIR/target"
sync_scryfall_cards_for_rebuild

if [[ ! -f "$DB_PATH" ]]; then
  cat >&2 <<EOF
[ERROR] registry DB not found: $DB_PATH

Run the registry sync first, for example:
  cargo run --release -p ironsmith-registry-sync --bin sync_registry_db -- --cards cards.json --db-path "$DB_PATH"

Or let this script do the missing-card preflight by omitting --skip-scryfall-sync.
EOF
  exit 1
fi

FRONTEND_CARDS_CACHE_MANIFEST="$FRONTEND_CARDS_DIR/$FRONTEND_CARDS_CACHE_MANIFEST_NAME"
PENDING_FRONTEND_CARDS_CACHE_MANIFEST="$(mktemp "$ROOT_DIR/target/frontend-card-assets-cache.json.XXXXXX")"
write_frontend_cards_cache_manifest "$PENDING_FRONTEND_CARDS_CACHE_MANIFEST"

FRONTEND_JSON_CACHE_CURRENT=0
if frontend_cards_cache_matches \
  "$PENDING_FRONTEND_CARDS_CACHE_MANIFEST" \
  "$FRONTEND_CARDS_CACHE_MANIFEST" \
  "$FRONTEND_CARDS_DIR" \
  "$FRONTEND_SCORES_FILE"; then
  FRONTEND_JSON_CACHE_CURRENT=1
fi

if [[ "$FRONTEND_JSON_CACHE_CURRENT" -eq 1 ]]; then
  rm -f "$PENDING_FRONTEND_CARDS_CACHE_MANIFEST"
  echo "[INFO] skipped frontend JSON assets; checksum cache is current: $FRONTEND_CARDS_DIR"
else
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

  python3 "$ROOT_DIR/scripts/generate_baked_registry.py" \
    --db-path "$DB_PATH" \
    --out "$ROOT_DIR/target/generated_registry_for_frontend_assets.rs" \
    --frontend-cards-dir "$FRONTEND_CARDS_DIR"
  cargo run --release -p ironsmith-artifact-baker --bin bake_card_artifacts -- \
    --cards-dir "$FRONTEND_CARDS_DIR"
  mkdir -p "$FRONTEND_CARDS_DIR"
  mv -f "$PENDING_FRONTEND_CARDS_CACHE_MANIFEST" "$FRONTEND_CARDS_CACHE_MANIFEST"
  echo "[INFO] synced frontend card compilation assets: $FRONTEND_CARDS_DIR"
fi

if feature_enabled "generated-registry"; then
  export IRONSMITH_REGISTRY_DB_PATH="$DB_PATH"
  echo "[INFO] registry DB source: $IRONSMITH_REGISTRY_DB_PATH"
else
  echo "[INFO] generated registry disabled; WASM will load card compilation assets from frontend cards/"
fi
echo "[INFO] wasm build profile: release"
if [[ "$NO_DEFAULT_FEATURES" -eq 1 ]]; then
  echo "[INFO] cargo default features: disabled"
else
  echo "[INFO] cargo default features: enabled"
fi
if [[ "$OPTIMIZE_WASM" -eq 1 ]]; then
  echo "[INFO] wasm-opt: enabled"
else
  echo "[INFO] wasm-opt: disabled (--no-opt)"
fi

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

build_split_wasm_package() {
  local engine_features
  local wasm_opt
  local cargo_args
  local cargo_packages
  local raw_wasm_files
  local artifact_names
  local generated_wasm
  local artifact
  local artifact_name
  local index
  local engine_opt_pid
  local compiler_opt_pid
  local verifier_opt_pid

  engine_features="$(printf '%s' "$FEATURES" | sed 's/[[:space:]]//g; s/,/,ironsmith-engine-wasm\//g; s/^/ironsmith-engine-wasm\//')"
  cargo_packages=(
    -p ironsmith-engine-wasm
    -p ironsmith-compiler-wasm
    -p ironsmith-verifier-wasm
  )
  cargo_args=(
    build
    "${cargo_packages[@]}"
    --target wasm32-unknown-unknown
    --release
  )
  if [[ "$NO_DEFAULT_FEATURES" -eq 1 ]]; then
    cargo_args+=(--no-default-features)
  fi
  cargo_args+=(--features "$engine_features")
  cargo "${cargo_args[@]}"

  raw_wasm_files=(
    "$ROOT_DIR/target/wasm32-unknown-unknown/release/ironsmith_engine_wasm.wasm"
    "$ROOT_DIR/target/wasm32-unknown-unknown/release/ironsmith_compiler_wasm.wasm"
    "$ROOT_DIR/target/wasm32-unknown-unknown/release/ironsmith_verifier_wasm.wasm"
  )
  artifact_names=(engine compiler verifier)
  for artifact in "${raw_wasm_files[@]}"; do
    [[ -f "$artifact" ]] || { echo "[ERROR] missing split wasm artifact: $artifact" >&2; return 1; }
  done

  rm -rf -- "$PKG_DIR"
  mkdir -p "$PKG_DIR"
  for ((index = 0; index < ${#raw_wasm_files[@]}; index += 1)); do
    wasm-bindgen "${raw_wasm_files[$index]}" \
      --target web \
      --out-dir "$PKG_DIR" \
      --out-name "${artifact_names[$index]}"
  done

  generated_wasm=()
  for artifact_name in "${artifact_names[@]}"; do
    generated_wasm+=("$PKG_DIR/${artifact_name}_bg.wasm")
  done

  if [[ "$OPTIMIZE_WASM" -eq 1 ]]; then
    wasm_opt="$(find_cached_wasm_opt)" || {
      echo "[ERROR] release packaging requires wasm-opt" >&2
      return 1
    }
    echo "[INFO] optimizing split artifacts with $WASM_OPT_LEVEL and at most two wasm-opt processes"
    "$wasm_opt" "$WASM_OPT_LEVEL" "${generated_wasm[0]}" -o "${generated_wasm[0]}.optimized" &
    engine_opt_pid="$!"
    "$wasm_opt" "$WASM_OPT_LEVEL" "${generated_wasm[1]}" -o "${generated_wasm[1]}.optimized" &
    compiler_opt_pid="$!"
    wait "$compiler_opt_pid"
    "$wasm_opt" "$WASM_OPT_LEVEL" "${generated_wasm[2]}" -o "${generated_wasm[2]}.optimized" &
    verifier_opt_pid="$!"
    wait "$engine_opt_pid"
    wait "$verifier_opt_pid"
    for artifact in "${generated_wasm[@]}"; do
      mv -f "$artifact.optimized" "$artifact"
    done
  fi

  cp -f "$ROOT_DIR/npm/ironsmith-wasm/split-facade.js" "$PKG_DIR/ironsmith.js"
  cp -f "$ROOT_DIR/npm/ironsmith-wasm/split-facade.d.ts" "$PKG_DIR/ironsmith.d.ts"
  cp -f "$ROOT_DIR/npm/ironsmith-wasm/package.template.json" "$PKG_DIR/package.json"
  if [[ -f "$ROOT_DIR/npm/ironsmith-wasm/README.md" ]]; then
    cp -f "$ROOT_DIR/npm/ironsmith-wasm/README.md" "$PKG_DIR/README.md"
  fi
}

build_split_wasm_package

rm -rf -- "$DEMO_PKG_DIR"
mkdir -p "$DEMO_PKG_DIR"
cp -Rf "$PKG_DIR/." "$DEMO_PKG_DIR/"
