#!/bin/bash
set -uo pipefail
ROOT=$(pwd)
export RUSTUP_HOME="$ROOT/reports/current-refresh-20261008/toolchain/rustup"
export CARGO_HOME="$ROOT/reports/current-refresh-20261008/toolchain/cargo"
export PATH="$CARGO_HOME/bin:$PATH"
export CARGO_BUILD_JOBS=1 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=256 CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_PROFILE_RELEASE_OPT_LEVEL=1 RAYON_NUM_THREADS=1
for key in $(env | cut -d= -f1 | grep '^IRONSMITH_'); do unset "$key"; done
rustc -Vv > reports/current-refresh-20261008/rustc-version.txt
cargo -Vv > reports/current-refresh-20261008/cargo-version.txt
git rev-parse HEAD HEAD^{tree} > reports/current-refresh-20261008/build-source.txt
git status --porcelain > reports/current-refresh-20261008/build-source-status.txt
date -u +%FT%TZ > reports/current-refresh-20261008/build-start.txt
cargo build --locked -p ironsmith-tools --bin sync_card_status_db --release > reports/current-refresh-20261008/build.stdout.log 2> reports/current-refresh-20261008/build.stderr.log
code=$?
printf '%s\n' "$code" > reports/current-refresh-20261008/build-exit-code.txt
date -u +%FT%TZ > reports/current-refresh-20261008/build-finish.txt
exit "$code"
