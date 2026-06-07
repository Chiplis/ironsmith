#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  cat <<EOF
Usage:
  AWS_PROFILE=ironsmith-843750990226 \\
  GITHUB_TOKEN_SSM_PARAM=/ironsmith/github-token \\
  OPENCODE_AUTH_JSON_SSM_PARAM=/ironsmith/opencode-auth-json \\
  WORKER_ENTRY_SKILL=ironsmith-aws-card-fixer-fleet \\
  scripts/aws_card_fixers/run_dev_loop.sh

This dev loop now always uses the rolling merge-and-refill scheduler.

Common env:
  MAX_ACTIVE_WORKERS=${MAX_ACTIVE_WORKERS:-${BATCH_SIZE:-8}}
  MAX_TOTAL_CARDS=${MAX_TOTAL_CARDS:-0}
  POLL_INTERVAL=${POLL_INTERVAL:-60}
  SESSION_TIMEOUT_SECONDS=${SESSION_TIMEOUT_SECONDS:-${BATCH_TIMEOUT_SECONDS:-28800}}
  REFRESH_AFTER_MERGE=${REFRESH_AFTER_MERGE:-1}
  BAKE_EVERY_MERGED_PRS=${BAKE_EVERY_MERGED_PRS:-1}
  WORKER_ARCH=${WORKER_ARCH:-arm64}
  INSTANCE_TYPES=${INSTANCE_TYPES:-"t4g.medium c7g.large c6g.large t4g.large"}

Legacy BATCH_SIZE is accepted as the default MAX_ACTIVE_WORKERS value.
EOF
  exit 0
fi

export MAX_ACTIVE_WORKERS="${MAX_ACTIVE_WORKERS:-${BATCH_SIZE:-8}}"
export SESSION_TIMEOUT_SECONDS="${SESSION_TIMEOUT_SECONDS:-${BATCH_TIMEOUT_SECONDS:-28800}}"
export BAKE_EVERY_MERGED_PRS="${BAKE_EVERY_MERGED_PRS:-1}"

exec "$ROOT/scripts/aws_card_fixers/run_rolling_dev_loop.sh" "$@"
