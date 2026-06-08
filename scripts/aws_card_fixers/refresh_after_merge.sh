#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DB_PATH="${DB_PATH:-$ROOT/reports/engine-status.sqlite3}"
BASE_BRANCH="${BASE_BRANCH:-main}"
PULL_MAIN="${PULL_MAIN:-1}"
SYNC_DB="${SYNC_DB:-1}"
BAKE_AMI="${BAKE_AMI:-1}"
PARALLEL_SYNC_AND_BAKE="${PARALLEL_SYNC_AND_BAKE:-0}"
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
if [[ "${IRONSMITH_CONTROLLER:-0}" == "1" ]]; then
  CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
else
  CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-1}"
fi
SYNC_DB_ATTEMPTS="${SYNC_DB_ATTEMPTS:-3}"
SYNC_DB_RETRY_SECONDS="${SYNC_DB_RETRY_SECONDS:-20}"
WORKER_ARCH="${WORKER_ARCH:-arm64}"
if [[ "$WORKER_ARCH" == "arm64" ]]; then
  WORKER_AMI_SSM_PARAM="${WORKER_AMI_SSM_PARAM:-/ironsmith/card-fixer-worker-ami-arm64}"
  SOURCE_AMI_SSM_PARAM="${SOURCE_AMI_SSM_PARAM:-/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64}"
else
  WORKER_AMI_SSM_PARAM="${WORKER_AMI_SSM_PARAM:-/ironsmith/card-fixer-worker-ami}"
  SOURCE_AMI_SSM_PARAM="${SOURCE_AMI_SSM_PARAM:-/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64}"
fi
AMI_BUILD_RELEASE_TOOLS="${AMI_BUILD_RELEASE_TOOLS:-1}"
AMI_CARGO_CLEAN_BEFORE_BUILD="${AMI_CARGO_CLEAN_BEFORE_BUILD:-1}"
DEREGISTER_OLD_AMI="${DEREGISTER_OLD_AMI:-1}"
CLEANUP_OLD_WORKER_AMIS="${CLEANUP_OLD_WORKER_AMIS:-1}"
RETAIN_WORKER_AMIS_PER_ARCH="${RETAIN_WORKER_AMIS_PER_ARCH:-1}"
export CARGO_BUILD_JOBS CARGO_INCREMENTAL
export WORKER_ARCH WORKER_AMI_SSM_PARAM SOURCE_AMI_SSM_PARAM
export AMI_BUILD_RELEASE_TOOLS AMI_CARGO_CLEAN_BEFORE_BUILD
export DEREGISTER_OLD_AMI CLEANUP_OLD_WORKER_AMIS RETAIN_WORKER_AMIS_PER_ARCH

log_step() {
  printf '[refresh_after_merge] %s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"
}

controller_disk_hygiene() {
  if [[ "${CONTROLLER_DISK_HYGIENE:-${IRONSMITH_CONTROLLER:-0}}" != "1" ]]; then
    return 0
  fi
  if [[ ! -x "$ROOT/scripts/aws_card_fixers/controller_disk_hygiene.sh" ]]; then
    log_step "controller disk hygiene requested, but helper is missing or not executable"
    return 1
  fi
  "$ROOT/scripts/aws_card_fixers/controller_disk_hygiene.sh" "$1"
}

usage() {
  cat <<EOF
Usage:
  AWS_PROFILE=ironsmith-843750990226 \\
  AWS_REGION=us-east-2 \\
  scripts/aws_card_fixers/refresh_after_merge.sh

Optional env:
  BASE_BRANCH=main
  DB_PATH=reports/engine-status.sqlite3
  PULL_MAIN=1
  SYNC_DB=1
  BAKE_AMI=1
  PARALLEL_SYNC_AND_BAKE=0
  WORKER_ARCH=arm64
  WORKER_AMI_SSM_PARAM=/ironsmith/card-fixer-worker-ami-arm64
  SOURCE_AMI_SSM_PARAM=/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64
  AMI_BUILD_RELEASE_TOOLS=1
  AMI_CARGO_CLEAN_BEFORE_BUILD=1
  DEREGISTER_OLD_AMI=1
  CLEANUP_OLD_WORKER_AMIS=1
  RETAIN_WORKER_AMIS_PER_ARCH=1

This is the post-merge handoff: update local main, refresh the local engine
status DB, and bake/publish a fresh ARM worker AMI. By default this runs
serially to avoid memory pressure on the controller. The AMI bake cleans and
rebuilds Ironsmith tool crates, publishes the ARM SSM parameter, deregisters the
previous AMI, and deletes stale worker AMI snapshots. Set PARALLEL_SYNC_AND_BAKE=1
on larger machines if you want the old parallel behavior.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

cd "$ROOT"

if [[ "$PULL_MAIN" == "1" ]]; then
  log_step "fetching origin/${BASE_BRANCH}"
  git fetch origin "$BASE_BRANCH"
  current_branch="$(git branch --show-current)"
  if [[ "$current_branch" != "$BASE_BRANCH" ]]; then
    log_step "checking out ${BASE_BRANCH}"
    git checkout "$BASE_BRANCH"
  fi
  log_step "pulling origin/${BASE_BRANCH}"
  git pull --ff-only origin "$BASE_BRANCH"
fi

run_sync_db() {
  local attempt=1
  local status=0
  while (( attempt <= SYNC_DB_ATTEMPTS )); do
    log_step "sync_card_status_db attempt ${attempt}/${SYNC_DB_ATTEMPTS}"
    if cargo run --release -p ironsmith-tools --bin sync_card_status_db -- --db-path "$DB_PATH"; then
      return 0
    fi
    status=$?
    if (( attempt >= SYNC_DB_ATTEMPTS )); then
      log_step "sync_card_status_db failed after ${attempt} attempt(s), status=${status}"
      return "$status"
    fi
    log_step "sync_card_status_db failed with status=${status}; retrying in ${SYNC_DB_RETRY_SECONDS}s"
    sleep "$SYNC_DB_RETRY_SECONDS"
    attempt=$((attempt + 1))
  done
}

run_bake_ami() {
  log_step "baking ${WORKER_ARCH} worker AMI; cleanup_old=${CLEANUP_OLD_WORKER_AMIS}; clean_tools=${AMI_CARGO_CLEAN_BEFORE_BUILD}"
  "$ROOT/scripts/aws_card_fixers/bake_worker_ami.sh"
}

if [[ "$SYNC_DB" == "1" && "$BAKE_AMI" == "1" && "$PARALLEL_SYNC_AND_BAKE" == "1" ]]; then
  echo "Starting DB sync and AMI bake in parallel."
  run_sync_db &
  sync_pid=$!
  run_bake_ami &
  bake_pid=$!

  sync_status=0
  bake_status=0
  wait "$sync_pid" || sync_status=$?
  wait "$bake_pid" || bake_status=$?

  if [[ "$sync_status" -ne 0 || "$bake_status" -ne 0 ]]; then
    echo "Post-merge refresh failed: sync_status=${sync_status}, bake_status=${bake_status}" >&2
    exit 1
  fi
else
  if [[ "$SYNC_DB" == "1" ]]; then
    run_sync_db
    controller_disk_hygiene post_sync
  fi

  if [[ "$BAKE_AMI" == "1" ]]; then
    run_bake_ami
  fi
fi

controller_disk_hygiene post_refresh

echo "Post-merge refresh complete."
