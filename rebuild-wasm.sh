#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$ROOT_DIR/pkg"
DEMO_PKG_DIR="$ROOT_DIR/web/wasm_demo/pkg"
FEATURES="wasm,generated-registry"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

usage() {
  cat <<'USAGE'
Usage: ./rebuild-wasm.sh [--features <csv>]

Examples:
  ./rebuild-wasm.sh
  ./rebuild-wasm.sh --features wasm

Notes:
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

wasm-pack build --release --target web --features "$FEATURES"

mkdir -p "$DEMO_PKG_DIR"
cp -f \
  "$PKG_DIR/ironsmith.js" \
  "$PKG_DIR/ironsmith_bg.wasm" \
  "$PKG_DIR/ironsmith.d.ts" \
  "$PKG_DIR/ironsmith_bg.wasm.d.ts" \
  "$PKG_DIR/package.json" \
  "$DEMO_PKG_DIR/"
