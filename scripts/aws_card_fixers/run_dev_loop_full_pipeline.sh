#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

RUN_ID="${RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)-dev-loop}"
RUN_DIR="${RUN_DIR:-$ROOT/reports/aws-card-fixer-dev-loop/${RUN_ID}}"
LOG_FILE="${LOG_FILE:-$RUN_DIR/dev-loop-${RUN_ID}.log}"

usage() {
  cat <<EOF
Usage:
  scripts/aws_card_fixers/run_dev_loop_full_pipeline.sh

Optional overrides:
  RUN_ID=$RUN_ID
  RUN_DIR=$RUN_DIR
  LOG_FILE=$LOG_FILE
  USE_INSTANCE_PROFILE=${USE_INSTANCE_PROFILE:-0}
  MAX_ACTIVE_WORKERS=${MAX_ACTIVE_WORKERS:-8}
  DRY_RUN=${DRY_RUN:-0}
  REVIEW_OPEN_PRS_ON_START=${REVIEW_OPEN_PRS_ON_START:-1}
  USE_SPOT=${USE_SPOT:-1}
  WORKER_ARCH=${WORKER_ARCH:-arm64}
  OPENCODE_VARIANT=${OPENCODE_VARIANT:-fast}
  S3_SESSION_PREFIX=${S3_SESSION_PREFIX:-sessions}
  CONTROLLER_DISK_HYGIENE=${CONTROLLER_DISK_HYGIENE:-${IRONSMITH_CONTROLLER:-0}}

This starts the AWS worker dev loop with the current full-pipeline defaults:
worker AMI required, rolling merge-and-refill scheduling, status DB refresh after
each merge group, and ARM AMI rebaking after every worker PR merge group.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

mkdir -p "$RUN_DIR"

cat <<EOF
Starting Ironsmith AWS card-fixer dev loop
Run ID:      $RUN_ID
Run dir:     $RUN_DIR
Unified log: $LOG_FILE

The current terminal will stream the unified log. From another terminal:
  tail -f "$LOG_FILE"

EOF

if [[ "${USE_INSTANCE_PROFILE:-0}" == "1" ]]; then
  unset AWS_PROFILE
else
  export AWS_PROFILE="${AWS_PROFILE:-ironsmith-843750990226}"
fi
export AWS_REGION="${AWS_REGION:-us-east-2}"
export BASE_BRANCH="${BASE_BRANCH:-main}"
export DB_PATH="${DB_PATH:-$ROOT/reports/engine-status.sqlite3}"

export MAX_ACTIVE_WORKERS="${MAX_ACTIVE_WORKERS:-${BATCH_SIZE:-8}}"
export WORKER_ARCH="${WORKER_ARCH:-arm64}"
if [[ "$WORKER_ARCH" == "arm64" ]]; then
  export INSTANCE_TYPE="${INSTANCE_TYPE:-t4g.medium}"
  export INSTANCE_TYPES="${INSTANCE_TYPES:-t4g.medium t4g.large c8g.large c7g.large c6g.large m7g.large m6g.large r7g.large r6g.large}"
else
  export INSTANCE_TYPE="${INSTANCE_TYPE:-c7i.large}"
  export INSTANCE_TYPES="${INSTANCE_TYPES:-c7i.large c7a.large c6a.large c6i.large m7a.large m7i.large m6a.large m6i.large r7a.large r7i.large}"
fi
export USE_SPOT="${USE_SPOT:-1}"
export USE_EC2_FLEET="${USE_EC2_FLEET:-1}"
export SPOT_MAX_PRICE="${SPOT_MAX_PRICE:-}"
export BURSTABLE_CPU_CREDITS="${BURSTABLE_CPU_CREDITS:-standard}"
export MAX_TOTAL_CARDS="${MAX_TOTAL_CARDS:-0}"
export POLL_INTERVAL="${POLL_INTERVAL:-60}"
export BATCH_TIMEOUT_SECONDS="${BATCH_TIMEOUT_SECONDS:-28800}"
export SESSION_TIMEOUT_SECONDS="${SESSION_TIMEOUT_SECONDS:-$BATCH_TIMEOUT_SECONDS}"
export STOP_ON_FAILED="${STOP_ON_FAILED:-0}"
export DRY_RUN="${DRY_RUN:-0}"

export REVIEW_OPEN_PRS_ON_START="${REVIEW_OPEN_PRS_ON_START:-1}"
export REFRESH_AFTER_MERGE="${REFRESH_AFTER_MERGE:-1}"
export BAKE_EVERY_MERGED_PRS="${BAKE_EVERY_MERGED_PRS:-1}"
export BAKE_ON_STOP="${BAKE_ON_STOP:-0}"
export CODEX_HANDLE_REMAINING_PRS="${CODEX_HANDLE_REMAINING_PRS:-1}"
export STEWARD_HANDLE_REMAINING_PRS="${STEWARD_HANDLE_REMAINING_PRS:-1}"
export STEWARD_COMMAND="${STEWARD_COMMAND:-opencode}"
export STEWARD_MODEL="${STEWARD_MODEL:-${OPENCODE_MODEL:-openai/gpt-5.5-fast}}"
export STEWARD_VARIANT="${STEWARD_VARIANT:-${OPENCODE_VARIANT:-fast}}"
export CONTROLLER_CARGO_JOBS="${CONTROLLER_CARGO_JOBS:-${CARGO_BUILD_JOBS:-2}}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$CONTROLLER_CARGO_JOBS}"
if [[ "${IRONSMITH_CONTROLLER:-0}" == "1" ]]; then
  export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
else
  export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-1}"
fi
export SAFE_MERGE_VERIFY_COMMAND="${SAFE_MERGE_VERIFY_COMMAND:-cargo check --workspace -j ${CONTROLLER_CARGO_JOBS}}"
export PARALLEL_SYNC_AND_BAKE="${PARALLEL_SYNC_AND_BAKE:-0}"

export GITHUB_TOKEN_SSM_PARAM="${GITHUB_TOKEN_SSM_PARAM:-/ironsmith/github-token}"
export OPENAI_API_KEY_SSM_PARAM="${OPENAI_API_KEY_SSM_PARAM:-}"
if [[ -n "$OPENAI_API_KEY_SSM_PARAM" ]]; then
  export OPENCODE_AUTH_JSON_SSM_PARAM="${OPENCODE_AUTH_JSON_SSM_PARAM:-}"
else
  export OPENCODE_AUTH_JSON_SSM_PARAM="${OPENCODE_AUTH_JSON_SSM_PARAM:-/ironsmith/opencode-auth-json}"
fi
export OPENCODE_MODEL="${OPENCODE_MODEL:-openai/gpt-5.5-fast}"
export OPENCODE_VARIANT="${OPENCODE_VARIANT:-fast}"
export OPENCODE_FAST_REASONING_EFFORT="${OPENCODE_FAST_REASONING_EFFORT:-high}"
export OPENCODE_FAST_TEXT_VERBOSITY="${OPENCODE_FAST_TEXT_VERBOSITY:-low}"
export OPENCODE_FAST_SERVICE_TIER="${OPENCODE_FAST_SERVICE_TIER:-priority}"
export OPENCODE_STALE_TIMEOUT_SECONDS="${OPENCODE_STALE_TIMEOUT_SECONDS:-1800}"
export OPENCODE_HEARTBEAT_SECONDS="${OPENCODE_HEARTBEAT_SECONDS:-60}"
export OPENCODE_NO_COMMIT_RETRIES="${OPENCODE_NO_COMMIT_RETRIES:-1}"
export POST_PR_STEWARD_MAX_REPAIRS="${POST_PR_STEWARD_MAX_REPAIRS:-3}"

export WORKER_ENTRY_SKILL="${WORKER_ENTRY_SKILL:-ironsmith-aws-card-fixer-fleet}"
if [[ "$WORKER_ARCH" == "arm64" ]]; then
  export WORKER_AMI_SSM_PARAM="${WORKER_AMI_SSM_PARAM:-/ironsmith/card-fixer-worker-ami-arm64}"
  export SOURCE_AMI_SSM_PARAM="${SOURCE_AMI_SSM_PARAM:-/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64}"
else
  export WORKER_AMI_SSM_PARAM="${WORKER_AMI_SSM_PARAM:-/ironsmith/card-fixer-worker-ami}"
  export SOURCE_AMI_SSM_PARAM="${SOURCE_AMI_SSM_PARAM:-/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64}"
fi
export REQUIRE_WORKER_AMI="${REQUIRE_WORKER_AMI:-1}"
export AMI_BUILD_RELEASE_TOOLS="${AMI_BUILD_RELEASE_TOOLS:-1}"
export AMI_CARGO_CLEAN_BEFORE_BUILD="${AMI_CARGO_CLEAN_BEFORE_BUILD:-1}"
export DEREGISTER_OLD_AMI="${DEREGISTER_OLD_AMI:-1}"
export CLEANUP_OLD_WORKER_AMIS="${CLEANUP_OLD_WORKER_AMIS:-1}"
export RETAIN_WORKER_AMIS_PER_ARCH="${RETAIN_WORKER_AMIS_PER_ARCH:-1}"
export INSTANCE_TTL_HOURS="${INSTANCE_TTL_HOURS:-6}"
export SELF_TERMINATE="${SELF_TERMINATE:-1}"
export S3_SESSION_PREFIX="${S3_SESSION_PREFIX:-sessions}"

export RUN_ID
export RUN_DIR
export LOG_FILE

exec "$ROOT/scripts/aws_card_fixers/run_rolling_dev_loop.sh"
