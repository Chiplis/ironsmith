#!/usr/bin/env bash
set -u -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_ROOT="$ROOT/scripts/ported-mage-tests"
REPORT_DIR="$ROOT/reports"
TIMESTAMP="$(date '+%Y%m%d-%H%M%S')"
LOG_FILE="${MAGE_TEST_LOG:-$REPORT_DIR/ported-mage-tests-$TIMESTAMP.tap}"
CONCURRENCY="${MAGE_TEST_CONCURRENCY:-${TEST_CONCURRENCY:-8}}"
BUILD_WASM="${MAGE_TEST_BUILD_WASM:-1}"

usage() {
  cat <<'USAGE'
Usage: ./run-mage-tests.sh [--no-build] [--concurrency N] [--log PATH] [--help]

Runs all ported MAGE .mjs tests with the release WASM build and prints a short
summary. The full Node test output is written to reports/ by default.

Environment:
  MAGE_TEST_CONCURRENCY=N   Same as --concurrency.
  TEST_CONCURRENCY=N        Fallback concurrency setting.
  MAGE_TEST_LOG=PATH        Same as --log.
  MAGE_TEST_BUILD_WASM=0    Same as --no-build.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-build)
      BUILD_WASM=0
      shift
      ;;
    --concurrency)
      if [[ $# -lt 2 ]]; then
        echo "error: --concurrency requires a value" >&2
        exit 2
      fi
      CONCURRENCY="$2"
      shift 2
      ;;
    --log)
      if [[ $# -lt 2 ]]; then
        echo "error: --log requires a path" >&2
        exit 2
      fi
      LOG_FILE="$2"
      shift 2
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! "$CONCURRENCY" =~ ^[0-9]+$ ]] || [[ "$CONCURRENCY" -lt 1 ]]; then
  echo "error: concurrency must be a positive integer, got: $CONCURRENCY" >&2
  exit 2
fi

if [[ ! -d "$TEST_ROOT" ]]; then
  echo "error: missing test directory: $TEST_ROOT" >&2
  exit 1
fi

mkdir -p "$(dirname "$LOG_FILE")"

cd "$ROOT" || exit 1

if [[ "$BUILD_WASM" != "0" ]]; then
  echo "Building release WASM..."
  if ! ./rebuild-wasm.sh; then
    echo "error: release WASM build failed" >&2
    exit 1
  fi
fi

TEST_ARGS=()
while IFS= read -r test_file; do
  TEST_ARGS+=("$test_file")
done < <(find "$TEST_ROOT" -name '*.test.mjs' | sort)

TEST_FILES="${#TEST_ARGS[@]}"
if [[ "$TEST_FILES" -eq 0 ]]; then
  echo "error: no .test.mjs files found under: $TEST_ROOT" >&2
  exit 1
fi

echo "Running ported MAGE tests..."
echo "  files:       $TEST_FILES"
echo "  concurrency: $CONCURRENCY"
echo "  log:         $LOG_FILE"
echo

START_SECONDS="$(date +%s)"

node --test \
  --test-isolation=none \
  --test-concurrency="$CONCURRENCY" \
  "${TEST_ARGS[@]}" 2>&1 | tee "$LOG_FILE"

STATUS="${PIPESTATUS[0]}"
END_SECONDS="$(date +%s)"
ELAPSED_SECONDS="$((END_SECONDS - START_SECONDS))"

summary_value() {
  local key="$1"
  awk -v key="$key" '
    $1 == "#" && $2 == key { value = $3 }
    $2 == key && $3 ~ /^[0-9.]+$/ { value = $3 }
    END { if (value != "") print value }
  ' "$LOG_FILE"
}

TESTS="$(summary_value tests)"
PASS="$(summary_value pass)"
FAIL="$(summary_value fail)"
SKIPPED="$(summary_value skipped)"
TODO="$(summary_value todo)"
DURATION_MS="$(summary_value duration_ms)"

echo
echo "Summary"
echo "  files:       $TEST_FILES"
echo "  tests:       ${TESTS:-unknown}"
echo "  passed:      ${PASS:-unknown}"
echo "  failed:      ${FAIL:-unknown}"
echo "  skipped:     ${SKIPPED:-0}"
if [[ -n "${TODO:-}" ]]; then
  echo "  todo:        $TODO"
fi
if [[ -n "${DURATION_MS:-}" ]]; then
  echo "  node time:   ${DURATION_MS}ms"
fi
echo "  wall time:   ${ELAPSED_SECONDS}s"
echo "  exit code:   $STATUS"
echo "  log:         $LOG_FILE"

if [[ "${FAIL:-0}" != "0" ]]; then
  echo
  echo "First failing tests:"
  awk '
    /^not ok [0-9]+ - / {
      sub(/^not ok [0-9]+ - /, "  - ")
      print
      count++
      if (count == 20) exit
    }
    /failing tests:/ {
      in_failures = 1
      next
    }
    in_failures && NF && $0 !~ /^ / {
      sub(/^[^[:space:]]+[[:space:]]+/, "  - ")
      print
      count++
      if (count == 20) exit
    }
  ' "$LOG_FILE"
fi

exit "$STATUS"
