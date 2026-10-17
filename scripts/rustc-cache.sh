#!/bin/sh
# Cargo invokes this with the real rustc path followed by its arguments.
set -eu

if ! command -v sccache >/dev/null 2>&1; then
    exec "$@"
fi

# Resolve from this script, not Cargo's cwd (which may be a registry crate).
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
common=$(git -C "$root" rev-parse --path-format=absolute --git-common-dir)
main=$(dirname -- "$common")
export SCCACHE_DIR="${SCCACHE_DIR:-$main/target/sccache}"
export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-2G}"
# Use a separate daemon from other projects, so its cache settings take effect.
export SCCACHE_SERVER_PORT="${SCCACHE_SERVER_PORT:-42317}"
exec sccache "$@"
