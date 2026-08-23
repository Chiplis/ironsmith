#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_ROOT="${IRONSMITH_COLD_TARGET_ROOT:-${TMPDIR:-/tmp}/ironsmith-cold-build-target}"
OUTPUT_ROOT="$ROOT_DIR/reports/bench/cold-build"
FIXTURE_CARDS="$ROOT_DIR/fixtures/cold-build/cards.json"
RUNS=1
OPTIMIZE=1
KEEP_TARGET=0
BUILD_PREFLIGHT=1
CARGO_JOBS="${IRONSMITH_CARGO_JOBS:-8}"
WASM_OPT_LEVEL="${IRONSMITH_WASM_OPT_LEVEL:--O1}"
WASM_CARGO_PROFILE="wasm-release"
EXPECTED_RUSTC_PREFIX="rustc 1.90.0 "
EXPECTED_WASM_BINDGEN="wasm-bindgen 0.2.120"
EXPECTED_WASM_OPT="wasm-opt version 117 (version_117)"

usage() {
  cat <<'USAGE'
Usage: scripts/bench-cold-build.sh [options]

Options:
  --runs <n>          Number of fully cold runs (default: 1).
  --output-dir <dir>  Persistent timing/result directory.
  --no-opt            Skip wasm-opt (developer diagnostic only; not acceptance).
  --wasm-opt-level <level>
                      Optimizer level shipped by the package: -O1, -O2, -Os, or -Oz.
  --skip-preflight    Skip the native preflight build (diagnostic only).
  --keep-target       Keep the final run target for investigation.
  -h, --help          Show this help.

The dedicated CARGO_TARGET_DIR is deleted before every run. Cargo registry,
git, Rust toolchain, wasm-bindgen, and wasm-opt installations remain warm.
The acceptance measurement includes native preflight compilation, wasm Cargo
compilation, wasm-bindgen, production wasm optimization, package assembly, and
a JavaScript smoke import.
USAGE
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

resolve_wasm_opt() {
  if [[ -n "${WASM_OPT:-}" && -x "${WASM_OPT}" ]]; then
    printf '%s\n' "$WASM_OPT"
    return 0
  fi
  if command -v wasm-opt >/dev/null 2>&1; then
    command -v wasm-opt
    return 0
  fi
  local cache_root="${WASM_PACK_CACHE:-$HOME/Library/Caches/.wasm-pack}"
  local candidate
  while IFS= read -r candidate; do
    if [[ -x "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done < <(find "$cache_root" -maxdepth 4 -type f -path '*/bin/wasm-opt' 2>/dev/null | sort -r)
  return 1
}

validate_target_root() {
  case "$TARGET_ROOT" in
    /|"$HOME"|"$ROOT_DIR"|"$ROOT_DIR/target"|"")
      echo "refusing unsafe cold target root: $TARGET_ROOT" >&2
      exit 1
      ;;
  esac
  if [[ "$(basename -- "$TARGET_ROOT")" != "ironsmith-cold-build-target" ]]; then
    echo "cold target basename must be ironsmith-cold-build-target: $TARGET_ROOT" >&2
    exit 1
  fi
}

clear_run_target() {
  local run_target="$1"
  case "$run_target" in
    "$TARGET_ROOT"/run-*) rm -rf -- "$run_target" ;;
    *) echo "refusing unsafe run target cleanup: $run_target" >&2; exit 1 ;;
  esac
}

time_command() {
  local timing_file="$1"
  shift
  if [[ "$(uname -s)" == "Darwin" ]]; then
    /usr/bin/time -lp "$@" 2> >(tee "$timing_file" >&2)
  else
    /usr/bin/time -v "$@" 2> >(tee "$timing_file" >&2)
  fi
}

latest_cargo_timing() {
  local run_target="$1"
  find "$run_target/cargo-timings" -maxdepth 1 -type f -name 'cargo-timing-*.html' -print \
    | sort \
    | tail -n 1
}

write_phase_json() {
  local run_dir="$1"
  local run_number="$2"
  local started_epoch="$3"
  local finished_epoch="$4"
  local artifact_sizes="$5"
  local cargo_summary="$6"
  local bindgen_seconds="$7"
  local optimize_seconds="$8"
  local smoke_seconds="$9"

  python3 - \
    "$run_dir/result.json" \
    "$run_number" \
    "$started_epoch" \
    "$finished_epoch" \
    "$artifact_sizes" \
    "$cargo_summary" \
    "$bindgen_seconds" \
    "$optimize_seconds" \
    "$smoke_seconds" \
    "$OPTIMIZE" \
    "$BUILD_PREFLIGHT" \
    "$WASM_OPT_LEVEL" <<'PY'
import json
import sys
from pathlib import Path

(
    output,
    run_number,
    started,
    finished,
    artifact_sizes,
    cargo_summary,
    bindgen_seconds,
    optimize_seconds,
    smoke_seconds,
    optimize_enabled,
    preflight_enabled,
    optimizer_level,
) = sys.argv[1:]

cargo = json.loads(Path(cargo_summary).read_text(encoding="utf-8"))
artifacts = json.loads(Path(artifact_sizes).read_text(encoding="utf-8"))
payload = {
    "schemaVersion": 2,
    "run": int(run_number),
    "coldTargetDeletedBeforeRun": True,
    "preflightIncluded": preflight_enabled == "1",
    "optimizerIncluded": optimize_enabled == "1",
    "optimizerLevel": optimizer_level,
    "totalWallSeconds": int(finished) - int(started),
    "cargo": cargo,
    "bindgenWallSeconds": int(bindgen_seconds),
    "optimizerWallSeconds": int(optimize_seconds),
    "smokeWallSeconds": int(smoke_seconds),
    "artifacts": artifacts,
    "rawWasmBytes": artifacts["engine"]["rawBytes"],
    "optimizedWasmBytes": artifacts["engine"]["optimizedBytes"],
}

def read_time_log(name):
    path = Path(output).parent / name
    if not path.is_file():
        return None
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    metrics = {}
    for line in lines:
        stripped = line.strip()
        parts = stripped.split()
        if len(parts) == 2 and parts[0] in {"real", "user", "sys"}:
            try:
                metrics[parts[0] + "Seconds"] = float(parts[1])
            except ValueError:
                pass
        elif stripped.endswith("maximum resident set size"):
            try:
                metrics["peakRssBytes"] = int(parts[0])
            except (ValueError, IndexError):
                pass
        elif "Maximum resident set size (kbytes):" in stripped:
            try:
                metrics["peakRssBytes"] = int(stripped.rsplit(":", 1)[1].strip()) * 1024
            except ValueError:
                pass
    return metrics or None

payload["resourceUsage"] = {
    path.stem.removesuffix("-time"): read_time_log(path.name)
    for path in sorted(Path(output).parent.glob("*-time.txt"))
}
Path(output).write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --runs)
      [[ $# -ge 2 ]] || { echo "--runs requires a value" >&2; exit 1; }
      RUNS="$2"
      shift 2
      ;;
    --output-dir)
      [[ $# -ge 2 ]] || { echo "--output-dir requires a value" >&2; exit 1; }
      OUTPUT_ROOT="$2"
      shift 2
      ;;
    --no-opt)
      OPTIMIZE=0
      shift
      ;;
    --wasm-opt-level)
      [[ $# -ge 2 ]] || { echo "--wasm-opt-level requires a value" >&2; exit 1; }
      WASM_OPT_LEVEL="$2"
      shift 2
      ;;
    --skip-preflight)
      BUILD_PREFLIGHT=0
      shift
      ;;
    --keep-target)
      KEEP_TARGET=1
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

[[ "$RUNS" =~ ^[1-9][0-9]*$ ]] || { echo "--runs must be a positive integer" >&2; exit 1; }
case "$WASM_OPT_LEVEL" in
  -O1|-O2|-Os|-Oz) ;;
  *) echo "--wasm-opt-level must be -O1, -O2, -Os, or -Oz" >&2; exit 1 ;;
esac

require_cmd cargo
require_cmd node
require_cmd python3
require_cmd rustc
require_cmd rustup
require_cmd wasm-bindgen
validate_target_root

WASM_OPT_BIN=""
if [[ "$OPTIMIZE" -eq 1 ]]; then
  WASM_OPT_BIN="$(resolve_wasm_opt)" || {
    echo "missing wasm-opt; set WASM_OPT to the locked binary" >&2
    exit 1
  }
fi

rustup target list --installed | grep -qx 'wasm32-unknown-unknown' || {
  echo "wasm32-unknown-unknown target is not installed" >&2
  exit 1
}

[[ "$(rustc --version)" == "$EXPECTED_RUSTC_PREFIX"* ]] || {
  echo "rustc version mismatch: expected $EXPECTED_RUSTC_PREFIX..., got $(rustc --version)" >&2
  exit 1
}
[[ "$(wasm-bindgen --version)" == "$EXPECTED_WASM_BINDGEN" ]] || {
  echo "wasm-bindgen version mismatch: expected $EXPECTED_WASM_BINDGEN, got $(wasm-bindgen --version)" >&2
  exit 1
}
if [[ "$OPTIMIZE" -eq 1 && "$($WASM_OPT_BIN --version)" != "$EXPECTED_WASM_OPT" ]]; then
  echo "wasm-opt version mismatch: expected $EXPECTED_WASM_OPT, got $($WASM_OPT_BIN --version)" >&2
  exit 1
fi

mkdir -p "$OUTPUT_ROOT" "$TARGET_ROOT"
[[ -f "$FIXTURE_CARDS" ]] || { echo "missing pinned cards fixture: $FIXTURE_CARDS" >&2; exit 1; }
RUN_SET="$(date -u +%Y%m%dT%H%M%SZ)"
SET_DIR="$OUTPUT_ROOT/$RUN_SET"
mkdir -p "$SET_DIR"

{
  printf 'rustc=%s\n' "$(rustc --version)"
  printf 'cargo=%s\n' "$(cargo --version)"
  printf 'wasm-bindgen=%s\n' "$(wasm-bindgen --version)"
  if [[ "$OPTIMIZE" -eq 1 ]]; then
    printf 'wasm-opt=%s\n' "$($WASM_OPT_BIN --version)"
  fi
  printf 'jobs=%s\n' "$(sysctl -n hw.logicalcpu 2>/dev/null || getconf _NPROCESSORS_ONLN)"
  printf 'cargoJobs=%s\n' "$CARGO_JOBS"
  printf 'targetRoot=%s\n' "$TARGET_ROOT"
  printf 'wasmOptLevel=%s\n' "$WASM_OPT_LEVEL"
  printf 'wasmCargoProfile=%s\n' "$WASM_CARGO_PROFILE"
} > "$SET_DIR/tool-versions.txt"

for ((run_number = 1; run_number <= RUNS; run_number += 1)); do
  run_target="$TARGET_ROOT/run-$run_number"
  run_dir="$SET_DIR/run-$run_number"
  package_dir="$run_target/package"
  fixture_dir="$run_target/fixtures"
  mkdir -p "$run_dir"

  clear_run_target "$run_target"
  mkdir -p "$run_target" "$package_dir" "$fixture_dir"
  echo "[cold-build] run $run_number/$RUNS: deleted and recreated $run_target"

  started_epoch="$(date +%s)"
  export CARGO_TARGET_DIR="$run_target"
  export CARGO_INCREMENTAL=0
  unset RUSTC_WRAPPER CARGO_BUILD_RUSTC_WRAPPER SCCACHE_DIR

  cp "$FIXTURE_CARDS" "$fixture_dir/cards.json"

  if [[ "$BUILD_PREFLIGHT" -eq 1 ]]; then
    time_command "$run_dir/preflight-time.txt" \
      cargo build --locked --offline --release \
        -j "$CARGO_JOBS" \
        -p ironsmith-registry-sync --bin sync_registry_db --timings
    preflight_timing_html="$(latest_cargo_timing "$run_target")"
    [[ -n "$preflight_timing_html" ]] || { echo "preflight Cargo timing HTML was not produced" >&2; exit 1; }
    cp "$preflight_timing_html" "$run_dir/preflight-cargo-timing.html"
    python3 "$ROOT_DIR/scripts/parse_cargo_timing.py" \
      "$run_dir/preflight-cargo-timing.html" \
      --output "$run_dir/preflight-cargo-summary.json"
    "$run_target/release/sync_registry_db" \
      --cards "$fixture_dir/cards.json" \
      --db-path "$fixture_dir/status.sqlite3" \
      > "$run_dir/preflight-smoke.txt"
  fi

  cargo_packages=(
    -p ironsmith-engine-wasm
    -p ironsmith-compiler-wasm
    -p ironsmith-verifier-wasm
  )
  time_command "$run_dir/wasm-cargo-time.txt" \
    cargo build --locked --offline --profile "$WASM_CARGO_PROFILE" \
      -j "$CARGO_JOBS" \
      "${cargo_packages[@]}" \
      --target wasm32-unknown-unknown \
      --no-default-features \
      --features ironsmith-engine-wasm/wasm-lean \
      --timings

  timing_html="$(latest_cargo_timing "$run_target")"
  [[ -n "$timing_html" ]] || { echo "Cargo timing HTML was not produced" >&2; exit 1; }
  cp "$timing_html" "$run_dir/cargo-timing.html"
  python3 "$ROOT_DIR/scripts/parse_cargo_timing.py" \
    "$run_dir/cargo-timing.html" \
    --output "$run_dir/cargo-summary.json"

  engine_raw="$run_target/wasm32-unknown-unknown/$WASM_CARGO_PROFILE/ironsmith_engine_wasm.wasm"
  compiler_raw="$run_target/wasm32-unknown-unknown/$WASM_CARGO_PROFILE/ironsmith_compiler_wasm.wasm"
  verifier_raw="$run_target/wasm32-unknown-unknown/$WASM_CARGO_PROFILE/ironsmith_verifier_wasm.wasm"
  raw_wasm_files=("$engine_raw" "$compiler_raw" "$verifier_raw")
  for raw_wasm in "${raw_wasm_files[@]}"; do
    [[ -f "$raw_wasm" ]] || { echo "missing raw wasm artifact: $raw_wasm" >&2; exit 1; }
  done

  bindgen_started="$(date +%s)"
  time_command "$run_dir/bindgen-engine-time.txt" \
    wasm-bindgen "$engine_raw" \
      --target web \
      --out-dir "$package_dir" \
      --out-name engine
  time_command "$run_dir/bindgen-compiler-time.txt" \
    wasm-bindgen "$compiler_raw" \
      --target web \
      --out-dir "$package_dir" \
      --out-name compiler
  time_command "$run_dir/bindgen-verifier-time.txt" \
    wasm-bindgen "$verifier_raw" \
      --target web \
      --out-dir "$package_dir" \
      --out-name verifier
  bindgen_finished="$(date +%s)"

  artifact_names=(engine compiler verifier)
  generated_wasm=(
    "$package_dir/engine_bg.wasm"
    "$package_dir/compiler_bg.wasm"
    "$package_dir/verifier_bg.wasm"
  )
  engine_generated="${generated_wasm[0]}"
  compiler_generated="${generated_wasm[1]}"
  verifier_generated="${generated_wasm[2]}"
  raw_wasm_bytes=()
  for artifact in "${generated_wasm[@]}"; do
    raw_wasm_bytes+=("$(wc -c < "$artifact" | tr -d '[:space:]')")
  done

  optimize_started="$(date +%s)"
  if [[ "$OPTIMIZE" -eq 1 ]]; then
    time_command "$run_dir/wasm-opt-engine-time.txt" \
      "$WASM_OPT_BIN" "$WASM_OPT_LEVEL" "$engine_generated" -o "$engine_generated.optimized" &
    engine_opt_pid="$!"
    time_command "$run_dir/wasm-opt-compiler-time.txt" \
      "$WASM_OPT_BIN" "$WASM_OPT_LEVEL" "$compiler_generated" -o "$compiler_generated.optimized" &
    compiler_opt_pid="$!"
    time_command "$run_dir/wasm-opt-verifier-time.txt" \
      "$WASM_OPT_BIN" "$WASM_OPT_LEVEL" "$verifier_generated" -o "$verifier_generated.optimized" &
    verifier_opt_pid="$!"
    wait "$engine_opt_pid"
    wait "$compiler_opt_pid"
    wait "$verifier_opt_pid"
    for artifact in "${generated_wasm[@]}"; do
      mv "$artifact.optimized" "$artifact"
    done
  fi
  optimize_finished="$(date +%s)"

  artifact_size_args=()
  for ((index = 0; index < ${#generated_wasm[@]}; index += 1)); do
    artifact_size_args+=(
      "${artifact_names[$index]}"
      "${raw_wasm_bytes[$index]}"
      "${generated_wasm[$index]}"
    )
  done
  python3 - "$run_dir/artifact-sizes.json" "${artifact_size_args[@]}" <<'PY'
import json
import sys
from pathlib import Path

output, *values = sys.argv[1:]
artifacts = {}
for index in range(0, len(values), 3):
    name, raw_bytes, optimized_path = values[index:index + 3]
    artifacts[name] = {
        "rawBytes": int(raw_bytes),
        "optimizedBytes": Path(optimized_path).stat().st_size,
    }
Path(output).write_text(json.dumps(artifacts, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

  cp "$ROOT_DIR/npm/ironsmith-wasm/package.template.json" "$package_dir/package.json"
  cp "$ROOT_DIR/npm/ironsmith-wasm/split-facade.js" "$package_dir/ironsmith.js"
  cp "$ROOT_DIR/npm/ironsmith-wasm/split-facade.d.ts" "$package_dir/ironsmith.d.ts"
  smoke_started="$(date +%s)"
  time_command "$run_dir/smoke-time.txt" \
    node --input-type=module - \
      "$package_dir/ironsmith.js" \
      "${generated_wasm[@]}" <<'JS'
import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const [modulePath, enginePath, compilerPath, verifierPath] = process.argv.slice(2);
const loaded = await import(pathToFileURL(modulePath));
if (typeof loaded.default !== "function") {
  throw new Error("split wasm package has no default initializer");
}
await loaded.default({
  engine: await readFile(enginePath),
  compiler: await readFile(compilerPath),
  verifier: await readFile(verifierPath),
});
for (const exportName of ["WasmGame", "compileCardArtifact", "compileAndRegisterCard", "ziffleKeygen"]) {
  if (typeof loaded[exportName] !== "function") {
    throw new Error(`split wasm package is missing ${exportName}`);
  }
}
if (typeof loaded.WasmGame.prototype.ziffleKeygen !== "function") {
  throw new Error("WasmGame verifier compatibility method was not installed");
}
const game = new loaded.WasmGame();
const artifact = loaded.compileAndRegisterCard(game, {
  name: "Cold Build Smoke Bolt",
  text: "Type: Instant\nCold Build Smoke Bolt deals 3 damage to any target.",
});
loaded.validateCompiledCardArtifact(artifact);
const bakedArtifact = loaded.compileCardArtifact({
  name: "Cold Build Baked Bear",
  text: "Type: Creature — Bear\nPower/Toughness: 2/2",
});
const baked = game.registerExternalCardSources({
  canonicalName: "Cold Build Baked Bear",
  aliases: [],
  artifacts: [bakedArtifact],
  group: {
    kind: "single",
    name: "Cold Build Baked Bear",
    block: "this deliberately is not valid compiler input",
    score: 1,
  },
});
if (baked.loaded !== 1 || !game.isKnownCardName("Cold Build Baked Bear")) {
  throw new Error(`baked artifact registration failed: ${JSON.stringify(baked)}`);
}
const external = game.registerExternalCardSources({
  canonicalName: "Cold Build Source Bear",
  aliases: [],
  group: {
    kind: "single",
    name: "Cold Build Source Bear",
    block: "Type: Creature — Bear\nPower/Toughness: 2/2",
    score: 1,
  },
});
if (external.loaded !== 1 || !game.isKnownCardName("Cold Build Source Bear")) {
  throw new Error(`compiler-to-engine source compatibility failed: ${JSON.stringify(external)}`);
}
const linked = game.registerExternalCardSources({
  canonicalName: "Cold Front",
  aliases: [{ alias: "Cold Front // Cold Back", canonical: "Cold Front" }],
  group: {
    kind: "linked",
    layout: "transform_like",
    combinedName: "Cold Front // Cold Back",
    faces: [
      { name: "Cold Front", block: "Type: Creature — Human\nPower/Toughness: 1/1", score: 1 },
      { name: "Cold Back", block: "Type: Creature — Werewolf\nPower/Toughness: 2/2", score: 1 },
    ],
  },
});
if (linked.loaded !== 2 || !game.isKnownCardName("Cold Back")) {
  throw new Error(`linked artifact registration failed: ${JSON.stringify(linked)}`);
}
const manabrew = game.registerManabrewDeckSources([{
  name: "cold build deck",
  cards: [{
    identity: { name: "Cold Build Plant" },
    manaCost: "{G}",
    types: ["Creature"],
    subtypes: ["Plant"],
    power: "1",
    toughness: "1",
    text: "",
  }],
}]);
if (manabrew.loaded !== 1 || !game.isKnownCardName("Cold Build Plant")) {
  throw new Error(`Manabrew artifact registration failed: ${JSON.stringify(manabrew)}`);
}
JS
  smoke_finished="$(date +%s)"
  finished_epoch="$(date +%s)"

  write_phase_json \
    "$run_dir" \
    "$run_number" \
    "$started_epoch" \
    "$finished_epoch" \
    "$run_dir/artifact-sizes.json" \
    "$run_dir/cargo-summary.json" \
    "$((bindgen_finished - bindgen_started))" \
    "$((optimize_finished - optimize_started))" \
    "$((smoke_finished - smoke_started))"

  echo "[cold-build] run $run_number result: $run_dir/result.json"

  if [[ "$KEEP_TARGET" -eq 0 || "$run_number" -lt "$RUNS" ]]; then
    clear_run_target "$run_target"
    echo "[cold-build] removed $run_target after preserving results"
  fi
done

python3 - "$SET_DIR" <<'PY'
import json
import statistics
import sys
from pathlib import Path

root = Path(sys.argv[1])
runs = [
    json.loads(path.read_text(encoding="utf-8"))
    for path in sorted(root.glob("run-*/result.json"))
]
walls = [run["totalWallSeconds"] for run in runs]
summary = {
    "schemaVersion": 1,
    "runCount": len(runs),
    "allUnderFiveMinutes": all(wall <= 300 for wall in walls),
    "minimumWallSeconds": min(walls),
    "medianWallSeconds": statistics.median(walls),
    "maximumWallSeconds": max(walls),
    "runs": runs,
}
(root / "summary.json").write_text(
    json.dumps(summary, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
print(json.dumps(summary, indent=2, sort_keys=True))
PY
