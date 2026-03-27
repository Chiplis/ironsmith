#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage: scripts/generate_test_flamegraph.sh [options] [-- <test-binary-args>...]

Profiles the Rust lib test harness on macOS using `sample`, then converts the
captured stacks into a flamegraph SVG with `inferno`.

Options:
  -o, --output <path>      Output SVG path.
                           Default: reports/flamegraphs/lib_tests_flamegraph.svg
  -d, --duration <secs>    Sampling duration in seconds.
                           Default: 30
  -i, --interval <ms>      Sampling interval in milliseconds.
                           Default: 1
  -r, --repeat <count>     Repeat the selected test run and merge samples.
                           Default: 1
  -h, --help               Show this help.

Everything after `--` is passed directly to the compiled Rust test harness, so
you can narrow the profile to a specific test:

  scripts/generate_test_flamegraph.sh -- \
    test_exchange_of_words_swapped_myr_moonvessel_dies_trigger_stacks_when_ornithopter_is_sacrificed --exact
EOF
}

OUTPUT="reports/flamegraphs/lib_tests_flamegraph.svg"
DURATION=30
INTERVAL=1
REPEAT=1
TEST_ARGS=()

while (($# > 0)); do
  case "$1" in
    -o|--output)
      OUTPUT="${2:?missing value for $1}"
      shift 2
      ;;
    -d|--duration)
      DURATION="${2:?missing value for $1}"
      shift 2
      ;;
    -i|--interval)
      INTERVAL="${2:?missing value for $1}"
      shift 2
      ;;
    -r|--repeat)
      REPEAT="${2:?missing value for $1}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      TEST_ARGS=("$@")
      break
      ;;
    *)
      TEST_ARGS+=("$1")
      shift
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script currently supports macOS only (requires /usr/bin/sample)." >&2
  exit 1
fi

for cmd in cargo sample inferno-collapse-sample inferno-flamegraph python3; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
done

mkdir -p "$(dirname "$OUTPUT")"

TEST_BINARY="$(
  cargo test --no-run --lib --message-format=json \
    | python3 -c '
import json, sys
executables = []
for line in sys.stdin:
    line = line.strip()
    if not line or not line.startswith("{"):
        continue
    try:
        msg = json.loads(line)
    except json.JSONDecodeError:
        continue
    if msg.get("profile", {}).get("test") and msg.get("executable"):
        executables.append(msg["executable"])
if not executables:
    raise SystemExit("Could not locate compiled lib test harness executable")
print(executables[-1])
'
)"

SAMPLE_OUT="${OUTPUT%.svg}.sample.txt"
FOLDED_OUT="${OUTPUT%.svg}.folded.txt"
TEST_BASENAME="$(basename "$TEST_BINARY")"

echo "Profiling test harness: $TEST_BINARY"
if ((${#TEST_ARGS[@]} > 0)); then
  printf 'Harness args:'
  printf ' %q' "${TEST_ARGS[@]}"
  printf '\n'
fi
if ((REPEAT > 1)); then
  echo "Repeats: $REPEAT"
fi

rm -f "$SAMPLE_OUT" "$FOLDED_OUT" "$OUTPUT"
touch "$SAMPLE_OUT" "$FOLDED_OUT"

SAMPLE_PID=""
cleanup() {
  if [[ -n "${SAMPLE_PID:-}" ]] && kill -0 "$SAMPLE_PID" >/dev/null 2>&1; then
    kill "$SAMPLE_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup INT TERM

TEST_STATUS=0
for ((run = 1; run <= REPEAT; run++)); do
  RUN_SAMPLE="$(mktemp "${TMPDIR:-/tmp}/ironsmith-flamegraph.${run}.XXXXXX.sample.txt")"
  RUN_FOLDED="$(mktemp "${TMPDIR:-/tmp}/ironsmith-flamegraph.${run}.XXXXXX.folded.txt")"

  sample "$TEST_BASENAME" "$DURATION" "$INTERVAL" -wait -mayDie -file "$RUN_SAMPLE" >/dev/null 2>&1 &
  SAMPLE_PID=$!

  RUN_STATUS=0
  if ((${#TEST_ARGS[@]} > 0)); then
    cargo test --lib -- "${TEST_ARGS[@]}" >/dev/null 2>&1 || RUN_STATUS=$?
  else
    cargo test --lib >/dev/null 2>&1 || RUN_STATUS=$?
  fi

  wait "$SAMPLE_PID" || true
  SAMPLE_PID=""

  if [[ -s "$RUN_SAMPLE" ]]; then
    cat "$RUN_SAMPLE" >> "$SAMPLE_OUT"
    printf '\n' >> "$SAMPLE_OUT"
    inferno-collapse-sample "$RUN_SAMPLE" > "$RUN_FOLDED" || true
    if [[ -s "$RUN_FOLDED" ]]; then
      cat "$RUN_FOLDED" >> "$FOLDED_OUT"
    fi
  fi

  rm -f "$RUN_SAMPLE" "$RUN_FOLDED"

  if ((RUN_STATUS != 0)); then
    TEST_STATUS=$RUN_STATUS
    break
  fi
done

trap - INT TERM

if [[ ! -s "$SAMPLE_OUT" ]]; then
  echo "sample did not produce any output at $SAMPLE_OUT" >&2
  exit 1
fi
if [[ ! -s "$FOLDED_OUT" ]]; then
  echo "sample did not produce any stack counts at $FOLDED_OUT" >&2
  exit 1
fi
inferno-flamegraph \
  --title "Rust lib test flamegraph" \
  --subtitle "$(basename "$TEST_BINARY")" \
  "$FOLDED_OUT" > "$OUTPUT"

echo "Wrote flamegraph SVG to $OUTPUT"
echo "Wrote raw sample to $SAMPLE_OUT"
echo "Wrote folded stacks to $FOLDED_OUT"

exit "$TEST_STATUS"
