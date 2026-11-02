#!/bin/sh
# Fast, read-only card coverage loop. Run from any directory:
# scripts/check-card-scores.sh --names /absolute/names.txt --out /absolute/results.json
# Optional --checks FILE adds audit/structure/mutation contracts.
# With no arguments, run the saved regression contracts into work/.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$repo_root"
if [ "$#" -eq 0 ]; then
    mkdir -p "$repo_root/work"
    set -- --names "$repo_root/crates/ironsmith-tools/tests/fixtures/card_score_probe_names.txt.fixture" \
        --checks "$repo_root/crates/ironsmith-tools/tests/fixtures/card_score_probe_checks.json.fixture" \
        --out "$repo_root/work/card-score-probe.json"
fi
export CARGO_INCREMENTAL=1
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-4}"
score_target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
case "$score_target_dir" in
    /*) ;;
    *) score_target_dir="$repo_root/$score_target_dir" ;;
esac
cargo build --profile artifact-bake -p ironsmith-tools --bin card_score_probe >&2
exec "$score_target_dir/artifact-bake/card_score_probe" "$@"
