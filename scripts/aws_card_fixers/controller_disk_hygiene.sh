#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PHASE="${1:-manual}"

CONTROLLER_DISK_HYGIENE="${CONTROLLER_DISK_HYGIENE:-${IRONSMITH_CONTROLLER:-0}}"
if [[ "$CONTROLLER_DISK_HYGIENE" != "1" ]]; then
  exit 0
fi

DB_PATH="${DB_PATH:-$ROOT/reports/engine-status.sqlite3}"
CONTROLLER_MIN_FREE_GB="${CONTROLLER_MIN_FREE_GB:-10}"
CONTROLLER_AGGRESSIVE_CLEAN_FREE_GB="${CONTROLLER_AGGRESSIVE_CLEAN_FREE_GB:-18}"
CONTROLLER_CARGO_CLEAN_AFTER_REFRESH="${CONTROLLER_CARGO_CLEAN_AFTER_REFRESH:-1}"
CONTROLLER_VACUUM_DB_AFTER_SYNC="${CONTROLLER_VACUUM_DB_AFTER_SYNC:-1}"
CONTROLLER_DISK_LOG_DU="${CONTROLLER_DISK_LOG_DU:-1}"

gb_to_kb() {
  local gb="$1"
  if [[ ! "$gb" =~ ^[0-9]+$ ]]; then
    echo "Controller disk hygiene expects an integer GiB value, got: ${gb}" >&2
    exit 2
  fi
  printf '%s\n' $((gb * 1024 * 1024))
}

available_kb() {
  df -Pk "$ROOT" | awk 'NR == 2 { print $4 }'
}

human_free() {
  df -h "$ROOT" | awk 'NR == 2 { print $4 " free on " $6 }'
}

log_usage() {
  echo "[controller_disk_hygiene] phase=${PHASE}; $(human_free)"
  if [[ "$CONTROLLER_DISK_LOG_DU" != "1" ]]; then
    return
  fi
  du -sh \
    "$ROOT/target" \
    "$DB_PATH" \
    "$ROOT/reports/aws-card-fixer-dev-loop" \
    /root/.cache \
    /root/.cargo \
    /root/.npm \
    /root/.local/share/opencode \
    /tmp \
    /var/tmp \
    2>/dev/null | sort -h || true
}

clean_tmp_and_caches() {
  echo "[controller_disk_hygiene] pruning temp files and lightweight caches"
  rm -rf \
    /tmp/ironsmith-* \
    /tmp/controller-overlay.tar.gz \
    /tmp/ironsmith-skills.tar.gz \
    /tmp/controller-bootstrap.sh \
    /tmp/compile_oracle_text-smoke.txt \
    /tmp/pr-comment.json \
    /tmp/pr.json \
    /tmp/ironsmith-cards.json.gz \
    2>/dev/null || true
  find /var/tmp -mindepth 1 -maxdepth 1 -exec rm -rf {} + 2>/dev/null || true
  find "$ROOT/target" -type d -name incremental -prune -exec rm -rf {} + 2>/dev/null || true
  rm -rf "$ROOT/target/tmp" 2>/dev/null || true
  npm cache clean --force >/dev/null 2>&1 || true
}

cargo_clean_controller() {
  if [[ ! -d "$ROOT/target" ]]; then
    return
  fi
  echo "[controller_disk_hygiene] running cargo clean for controller checkout"
  if command -v cargo >/dev/null 2>&1; then
    (cd "$ROOT" && cargo clean) || rm -rf "$ROOT/target"
  else
    rm -rf "$ROOT/target"
  fi
}

vacuum_status_db() {
  if [[ "$CONTROLLER_VACUUM_DB_AFTER_SYNC" != "1" || ! -f "$DB_PATH" ]]; then
    return
  fi
  if ! command -v sqlite3 >/dev/null 2>&1; then
    echo "[controller_disk_hygiene] sqlite3 not found; skipping DB vacuum"
    return
  fi

  local db_kb free_kb min_extra_kb required_kb
  db_kb="$(du -sk "$DB_PATH" | awk '{ print $1 }')"
  free_kb="$(available_kb)"
  min_extra_kb="$(gb_to_kb 1)"
  required_kb=$((db_kb + min_extra_kb))

  if (( free_kb < required_kb )); then
    echo "[controller_disk_hygiene] skipping DB vacuum; need about $((required_kb / 1024 / 1024)) GiB free, have $((free_kb / 1024 / 1024)) GiB"
    return
  fi

  echo "[controller_disk_hygiene] vacuuming status DB"
  sqlite3 "$DB_PATH" \
    'PRAGMA busy_timeout = 60000; PRAGMA wal_checkpoint(TRUNCATE); VACUUM; PRAGMA optimize;' \
    || echo "[controller_disk_hygiene] warning: DB vacuum failed; continuing"
}

ensure_free_space() {
  local min_kb aggressive_kb free_kb
  min_kb="$(gb_to_kb "$CONTROLLER_MIN_FREE_GB")"
  aggressive_kb="$(gb_to_kb "$CONTROLLER_AGGRESSIVE_CLEAN_FREE_GB")"
  free_kb="$(available_kb)"

  if (( free_kb < aggressive_kb || free_kb < min_kb )); then
    cargo_clean_controller
    clean_tmp_and_caches
    free_kb="$(available_kb)"
  fi

  if (( free_kb < min_kb )); then
    echo "[controller_disk_hygiene] only $((free_kb / 1024 / 1024)) GiB free after cleanup; require ${CONTROLLER_MIN_FREE_GB} GiB" >&2
    return 1
  fi
}

log_usage

case "$PHASE" in
  startup|pre_launch)
    clean_tmp_and_caches
    ensure_free_space
    ;;
  post_merge)
    clean_tmp_and_caches
    ensure_free_space
    ;;
  post_sync|post_refresh)
    clean_tmp_and_caches
    if [[ "$CONTROLLER_CARGO_CLEAN_AFTER_REFRESH" == "1" ]]; then
      cargo_clean_controller
    fi
    vacuum_status_db
    clean_tmp_and_caches
    ensure_free_space
    ;;
  *)
    clean_tmp_and_caches
    ensure_free_space
    ;;
esac

echo "[controller_disk_hygiene] complete; $(human_free)"
