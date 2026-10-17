#!/usr/bin/env bash
# Refresh suite membership before Cargo builds, including newly added tests.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 scripts/generate-native-test-suites.py
# A single consolidated suite can use multiple GiB while compiling. Keep the
# default memory demand bounded; callers can raise this on larger machines.
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
exec cargo nextest run --cargo-profile test-fast --workspace --all-targets --no-fail-fast "$@"
