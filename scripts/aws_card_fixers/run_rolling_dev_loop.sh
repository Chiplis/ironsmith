#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DB_PATH="${DB_PATH:-$ROOT/reports/engine-status.sqlite3}"
MAX_ACTIVE_WORKERS="${MAX_ACTIVE_WORKERS:-8}"
MAX_TOTAL_CARDS="${MAX_TOTAL_CARDS:-0}"
POLL_INTERVAL="${POLL_INTERVAL:-60}"
SESSION_TIMEOUT_SECONDS="${SESSION_TIMEOUT_SECONDS:-28800}"
BASE_BRANCH="${BASE_BRANCH:-main}"
GITHUB_REPO="${GITHUB_REPO:-Chiplis/ironsmith}"
AWS_REGION="${AWS_REGION:-$(aws configure get region 2>/dev/null || true)}"
AWS_REGION="${AWS_REGION:-us-east-2}"
RUN_ID="${RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)-rolling-dev-loop}"
RUN_DIR="${RUN_DIR:-$ROOT/reports/aws-card-fixer-dev-loop/${RUN_ID}}"
DRY_RUN="${DRY_RUN:-0}"
RESUME_RUN="${RESUME_RUN:-0}"
STOP_ON_FAILED="${STOP_ON_FAILED:-0}"
REFRESH_AFTER_MERGE="${REFRESH_AFTER_MERGE:-1}"
BAKE_EVERY_MERGED_PRS="${BAKE_EVERY_MERGED_PRS:-1}"
BAKE_ON_STOP="${BAKE_ON_STOP:-0}"
LOG_FILE="${LOG_FILE:-}"
S3_SESSION_PREFIX="${S3_SESSION_PREFIX:-sessions}"

usage() {
  cat <<EOF
Usage:
  AWS_PROFILE=ironsmith-843750990226 \\
  GITHUB_TOKEN_SSM_PARAM=/ironsmith/github-token \\
  OPENCODE_AUTH_JSON_SSM_PARAM=/ironsmith/opencode-auth-json \\
  WORKER_ENTRY_SKILL=ironsmith-aws-card-fixer-fleet \\
  scripts/aws_card_fixers/run_rolling_dev_loop.sh

Optional env:
  MAX_ACTIVE_WORKERS=8
  MAX_TOTAL_CARDS=0
  POLL_INTERVAL=60
  SESSION_TIMEOUT_SECONDS=28800
  BASE_BRANCH=main
  DB_PATH=reports/engine-status.sqlite3
  RUN_ID=custom-run-id
  RUN_DIR=reports/aws-card-fixer-dev-loop/RUN_ID
  DRY_RUN=0
  RESUME_RUN=0
  STOP_ON_FAILED=0
  REFRESH_AFTER_MERGE=1
  BAKE_EVERY_MERGED_PRS=1
  BAKE_ON_STOP=0
  S3_SESSION_PREFIX=sessions
  LOG_FILE=reports/aws-card-fixer-dev-loop/RUN_ID/dev-loop-DATE.log
  WORKER_ARCH=arm64
  INSTANCE_TYPES="t4g.medium c7g.large c6g.large t4g.large"
  USE_SPOT=1
  WORKER_AMI_SSM_PARAM=/ironsmith/card-fixer-worker-ami-arm64
  REQUIRE_WORKER_AMI=0
  SAFE_MERGE_VERIFY_COMMAND='cargo check --workspace -j 1'
  CONTROLLER_DISK_HYGIENE=1
  CONTROLLER_MIN_FREE_GB=10
  CONTROLLER_AGGRESSIVE_CLEAN_FREE_GB=18

MAX_ACTIVE_WORKERS is the rolling worker window. A worker is replaced only after
its PR has been merge-reviewed and origin/main has been refreshed locally.
BAKE_EVERY_MERGED_PRS controls AMI refresh after merged worker PRs; the default
of 1 rebakes after every merge group. Set it to 0 to skip rolling bakes. DB sync
still runs after every merge group when REFRESH_AFTER_MERGE=1.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ ! -f "$DB_PATH" ]]; then
  echo "DB not found: $DB_PATH" >&2
  exit 2
fi
if (( MAX_ACTIVE_WORKERS <= 0 )); then
  echo "MAX_ACTIVE_WORKERS must be positive." >&2
  exit 2
fi

AWS=(aws)
if [[ -n "${AWS_PROFILE:-}" ]]; then
  AWS+=(--profile "$AWS_PROFILE")
fi
AWS+=(--region "$AWS_REGION")

controller_disk_hygiene() {
  if [[ "${CONTROLLER_DISK_HYGIENE:-${IRONSMITH_CONTROLLER:-0}}" != "1" ]]; then
    return 0
  fi
  if [[ ! -x "$ROOT/scripts/aws_card_fixers/controller_disk_hygiene.sh" ]]; then
    echo "Controller disk hygiene requested, but helper is missing or not executable." >&2
    return 1
  fi
  "$ROOT/scripts/aws_card_fixers/controller_disk_hygiene.sh" "$1"
}

mkdir -p "$RUN_DIR"
if [[ -z "$LOG_FILE" ]]; then
  LOG_FILE="${RUN_DIR}/dev-loop-$(date -u +%Y%m%dT%H%M%SZ).log"
fi
if [[ "$RESUME_RUN" == "1" ]]; then
  touch "$LOG_FILE"
else
  : > "$LOG_FILE"
fi
exec > >(tee -a "$LOG_FILE") 2>&1

SESSIONS_FILE="${RUN_DIR}/sessions.tsv"
QUEUED_PRS_FILE="${RUN_DIR}/queued-prs.txt"
MERGED_PRS_FILE="${RUN_DIR}/merged-prs.txt"
RESUME_CARDS_FILE="${RUN_DIR}/resume-cards.txt"
RESUME_PRS_FILE="${RUN_DIR}/resume-prs.tsv"
CLOSED_RESUME_PRS_FILE="${RUN_DIR}/closed-resume-prs.txt"
if [[ "$RESUME_RUN" == "1" ]]; then
  touch "$SESSIONS_FILE" "$QUEUED_PRS_FILE" "$MERGED_PRS_FILE" "$RESUME_CARDS_FILE" "$RESUME_PRS_FILE" "$CLOSED_RESUME_PRS_FILE"
else
  : > "$SESSIONS_FILE"
  : > "$QUEUED_PRS_FILE"
  : > "$MERGED_PRS_FILE"
  : > "$RESUME_CARDS_FILE"
  : > "$RESUME_PRS_FILE"
  : > "$CLOSED_RESUME_PRS_FILE"
fi

echo "Run ID: $RUN_ID"
echo "Run dir: $RUN_DIR"
echo "Unified log: $LOG_FILE"
echo "Started: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Rolling window: ${MAX_ACTIVE_WORKERS} worker(s)"
if [[ "$RESUME_RUN" == "1" ]]; then
  echo "Resume mode: preserving existing run state."
fi

controller_disk_hygiene startup

account_id=""
bucket="${BUCKET:-}"
if [[ "$DRY_RUN" != "1" ]]; then
  account_id="$("${AWS[@]}" sts get-caller-identity --query Account --output text)"
  if [[ -z "$bucket" ]]; then
    bucket="ironsmith-card-fixers-${account_id}-${AWS_REGION}"
  fi
fi

eligible_count() {
  sqlite3 "$DB_PATH" "
    SELECT COUNT(*)
    FROM latest_card_compilation
    WHERE parse_status = 'parse_failed'
      AND COALESCE(agent_running, 0) = 0
      AND COALESCE(pr_created, 0) = 0;
  "
}

min_int() {
  local result="$1"
  shift
  local value
  for value in "$@"; do
    if (( value < result )); then
      result="$value"
    fi
  done
  printf '%s\n' "$result"
}

print_next_cards() {
  local count="$1"
  sqlite3 -header -column "$DB_PATH" "
    SELECT card_name
    FROM latest_card_compilation
    WHERE parse_status = 'parse_failed'
      AND COALESCE(agent_running, 0) = 0
      AND COALESCE(pr_created, 0) = 0
    ORDER BY random()
    LIMIT ${count};
  "
}

session_state() {
  local status_dir="$1"
  local expected_count="$2"
  python3 - "$status_dir" "$expected_count" <<'PY'
import json
import pathlib
import sys

status_dir = pathlib.Path(sys.argv[1])
expected_count = int(sys.argv[2])
total = succeeded = failed = 0

for path in sorted(status_dir.glob("*.json")):
    try:
        row = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        failed += 1
        total += 1
        continue
    total += 1
    if row.get("state") == "succeeded" and row.get("step") == "complete" and row.get("pr_url"):
        succeeded += 1
    elif row.get("state") in {"failed", "skipped"}:
        failed += 1

running = max(total - succeeded - failed, 0)
missing = max(expected_count - total, 0)
print(f"total={total}")
print(f"succeeded={succeeded}")
print(f"failed={failed}")
print(f"running={running}")
print(f"missing={missing}")
PY
}

collect_succeeded_pr_urls() {
  local status_dir="$1"
  local output_file="$2"
  python3 - "$status_dir" > "$output_file" <<'PY'
import json
import pathlib
import sys

status_dir = pathlib.Path(sys.argv[1])
seen = set()
for path in sorted(status_dir.glob("*.json")):
    try:
        row = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        continue
    if not (
        row.get("state") == "succeeded"
        and row.get("step") == "complete"
        and row.get("pr_url")
    ):
        continue
    pr_url = str(row["pr_url"])
    if pr_url in seen:
        continue
    seen.add(pr_url)
    print(pr_url)
PY
}

collect_resumable_cards() {
  local status_dir="$1"
  python3 - "$status_dir" "$RESUME_CARDS_FILE" "$RESUME_PRS_FILE" <<'PY'
import json
import pathlib
import sys

status_dir = pathlib.Path(sys.argv[1])
cards_path = pathlib.Path(sys.argv[2])
prs_path = pathlib.Path(sys.argv[3])
card_rows = []
pr_rows = []

for path in sorted(status_dir.glob("*.json")):
    try:
        row = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        continue
    if not (
        row.get("state") == "failed"
        and row.get("step") == "spot_interruption_notice"
        and row.get("pr_url")
        and row.get("card_name")
    ):
        continue
    card_name = str(row["card_name"])
    pr_url = str(row["pr_url"])
    card_rows.append(card_name)
    pr_rows.append(f"{card_name}\t{pr_url}")

if card_rows:
    with cards_path.open("a", encoding="utf-8") as handle:
        for card_name in card_rows:
            print(card_name, file=handle)
if pr_rows:
    with prs_path.open("a", encoding="utf-8") as handle:
        for row in pr_rows:
            print(row, file=handle)
PY
}

close_resolved_resume_prs() {
  if [[ ! -s "$RESUME_PRS_FILE" ]]; then
    return
  fi
  if ! command -v gh >/dev/null 2>&1; then
    echo "gh is not available; cannot close resolved Spot-interrupted PRs." >&2
    return
  fi

  local close_candidates="${RUN_DIR}/close-resolved-resume-prs.tsv"
  python3 - "$DB_PATH" "$RESUME_PRS_FILE" "$CLOSED_RESUME_PRS_FILE" > "$close_candidates" <<'PY'
import pathlib
import sqlite3
import sys

db_path = pathlib.Path(sys.argv[1])
resume_prs_path = pathlib.Path(sys.argv[2])
closed_path = pathlib.Path(sys.argv[3])

closed = set()
if closed_path.exists():
    closed.update(line.strip() for line in closed_path.read_text(encoding="utf-8").splitlines() if line.strip())

rows = []
seen = set()
for raw in resume_prs_path.read_text(encoding="utf-8").splitlines():
    parts = raw.split("\t", 1)
    if len(parts) != 2:
        continue
    card_name, pr_url = (part.strip() for part in parts)
    if not card_name or not pr_url or pr_url in closed or pr_url in seen:
        continue
    seen.add(pr_url)
    rows.append((card_name, pr_url))

if not rows:
    raise SystemExit(0)

conn = sqlite3.connect(db_path, timeout=60)
try:
    conn.execute("PRAGMA busy_timeout = 60000")
    for card_name, pr_url in rows:
        db_row = conn.execute(
            """
            SELECT parse_status, COALESCE(pr_created, 0)
            FROM latest_card_compilation
            WHERE card_name = ?1
            """,
            (card_name,),
        ).fetchone()
        if not db_row:
            continue
        parse_status, pr_created = db_row
        if parse_status == "parse_failed":
            continue
        print(f"{card_name}\t{pr_url}\t{parse_status}\t{int(pr_created or 0)}")
finally:
    conn.close()
PY

  local card_name pr_url parse_status pr_created pr_number
  while IFS=$'\t' read -r card_name pr_url parse_status pr_created; do
    [[ -n "${card_name:-}" && -n "${pr_url:-}" ]] || continue
    pr_number="${pr_url##*/}"
    if [[ "$(gh pr view "$pr_number" --repo "$GITHUB_REPO" --json state --jq .state 2>/dev/null || true)" != "OPEN" ]]; then
      printf '%s\n' "$pr_url" >> "$CLOSED_RESUME_PRS_FILE"
      continue
    fi
    echo "Closing resolved Spot-interrupted PR ${pr_url} for ${card_name}: parse_status=${parse_status}, pr_created=${pr_created}."
    gh pr close "$pr_number" \
      --repo "$GITHUB_REPO" \
      --comment "Closing this Spot-interrupted worker PR because the refreshed engine status DB no longer reports \`${card_name}\` as parse-failed.\n\nCurrent DB state: \`parse_status=${parse_status}\`, \`pr_created=${pr_created}\`.\n\nThat means a merged change on \`main\` fixed the card or made this resume PR obsolete." \
      >/dev/null
    printf '%s\n' "$pr_url" >> "$CLOSED_RESUME_PRS_FILE"
  done < "$close_candidates"

  sort -u "$CLOSED_RESUME_PRS_FILE" -o "$CLOSED_RESUME_PRS_FILE"
}

filter_new_pr_urls() {
  local input_file="$1"
  local output_file="$2"
  python3 - "$input_file" "$QUEUED_PRS_FILE" "$MERGED_PRS_FILE" > "$output_file" <<'PY'
import pathlib
import sys

input_path, queued_path, merged_path = map(pathlib.Path, sys.argv[1:4])
known = set()
for path in (queued_path, merged_path):
    if path.exists():
        known.update(line.strip() for line in path.read_text(encoding="utf-8").splitlines() if line.strip())

seen = set()
for raw in input_path.read_text(encoding="utf-8").splitlines():
    value = raw.strip()
    if not value or value in known or value in seen:
        continue
    seen.add(value)
    print(value)
PY
}

poll_sessions() {
  local all_prs="${RUN_DIR}/all-ready-prs.txt"
  local new_prs="${RUN_DIR}/new-ready-prs.txt"
  local active_sessions="${RUN_DIR}/sessions.active.tsv"
  local session_id s3_prefix expected launched_at status_log status_dir state_file
  local active=0 succeeded_all=0 failed_all=0 terminal_all=0 expected_all=0
  : > "$all_prs"
  : > "$active_sessions"

  while IFS=$'\t' read -r session_id s3_prefix expected launched_at; do
    [[ -n "${session_id:-}" ]] || continue
    status_log="${RUN_DIR}/${session_id}-status.log"
    status_dir="${RUN_DIR}/${session_id}-status-json"
    mkdir -p "$status_dir"

    S3_ASSET_PREFIX="$s3_prefix" \
    AWS_REGION="$AWS_REGION" \
    DB_PATH="$DB_PATH" \
    WATCH_INTERVAL=0 \
      "$ROOT/scripts/aws_card_fixers/monitor_fleet.sh" | tee -a "$status_log"

    rm -rf "$status_dir"
    mkdir -p "$status_dir"
    "${AWS[@]}" s3 cp --recursive "${s3_prefix}/status/" "$status_dir/" >/dev/null 2>&1 || true
    state_file="${RUN_DIR}/${session_id}-state.env"
    session_state "$status_dir" "$expected" > "$state_file"

    total=0 succeeded=0 failed=0 running=0 missing=0
    # state_file is generated by session_state with numeric shell assignments.
    # shellcheck disable=SC1090
    source "$state_file"

    active=$((active + running + missing))
    succeeded_all=$((succeeded_all + succeeded))
    failed_all=$((failed_all + failed))
    terminal_all=$((terminal_all + succeeded + failed))
    expected_all=$((expected_all + expected))
    collect_succeeded_pr_urls "$status_dir" "${RUN_DIR}/${session_id}-prs.txt"
    cat "${RUN_DIR}/${session_id}-prs.txt" >> "$all_prs"
    collect_resumable_cards "$status_dir"
    if (( succeeded + failed < expected )); then
      printf '%s\t%s\t%s\t%s\n' "$session_id" "$s3_prefix" "$expected" "$launched_at" >> "$active_sessions"
    fi

    if (( failed > 0 && STOP_ON_FAILED != 0 )); then
      echo "Session ${session_id} has ${failed} failed worker(s); STOP_ON_FAILED=1." >&2
      exit 4
    fi
    if (( $(date +%s) - launched_at >= SESSION_TIMEOUT_SECONDS && succeeded + failed < expected )); then
      echo "Session ${session_id} exceeded SESSION_TIMEOUT_SECONDS=${SESSION_TIMEOUT_SECONDS}." >&2
      exit 5
    fi
  done < "$SESSIONS_FILE"

  sort -u "$all_prs" -o "$all_prs"
  sort -u "$RESUME_CARDS_FILE" -o "$RESUME_CARDS_FILE"
  sort -u "$RESUME_PRS_FILE" -o "$RESUME_PRS_FILE"
  filter_new_pr_urls "$all_prs" "$new_prs"
  mv "$active_sessions" "$SESSIONS_FILE"

  echo "Rolling state: active=${active}/${MAX_ACTIVE_WORKERS}, terminal=${terminal_all}/${expected_all}, succeeded=${succeeded_all}, failed=${failed_all}, queued_prs=$(wc -l < "$QUEUED_PRS_FILE" | tr -d ' '), merged_prs=$(wc -l < "$MERGED_PRS_FILE" | tr -d ' ')"
  printf '%s\n' "$active" > "${RUN_DIR}/active-workers.txt"
}

launch_workers() {
  local count="$1"
  local wave="$2"
  local session_id="${RUN_ID}-wave$(printf '%04d' "$wave")"
  local s3_prefix="s3://${bucket}/${S3_SESSION_PREFIX}/${session_id}"
  local launch_log="${RUN_DIR}/${session_id}-launch.log"

  if (( count <= 0 )); then
    return
  fi

  echo
  echo "Launching ${count} replacement worker(s)."
  echo "Session: ${session_id}"

  if [[ "$DRY_RUN" == "1" ]]; then
    print_next_cards "$count"
    return
  fi

  controller_disk_hygiene pre_launch

  if SESSION_ID="$session_id" \
    INSTANCE_COUNT="$count" \
    DB_PATH="$DB_PATH" \
    RESUME_CARDS_FILE="$RESUME_CARDS_FILE" \
    BASE_BRANCH="$BASE_BRANCH" \
    AWS_REGION="$AWS_REGION" \
    S3_SESSION_PREFIX="$S3_SESSION_PREFIX" \
      "$ROOT/scripts/aws_card_fixers/launch_fleet.sh" | tee "$launch_log"; then
    printf '%s\t%s\t%s\t%s\n' "$session_id" "$s3_prefix" "$count" "$(date +%s)" >> "$SESSIONS_FILE"
  else
    local launch_status=$?
    echo "Rolling launch failed with status ${launch_status}." >&2
    exit "$launch_status"
  fi
}

merged_since_bake=0

merge_ready_prs() {
  local new_prs="${RUN_DIR}/new-ready-prs.txt"
  if [[ ! -s "$new_prs" ]]; then
    return 1
  fi

  local merge_id
  merge_id="merge$(date -u +%Y%m%dT%H%M%SZ)"
  local queue_file="${RUN_DIR}/${merge_id}-prs.txt"
  local merge_log="${RUN_DIR}/${merge_id}.log"
  local refresh_log="${RUN_DIR}/${merge_id}-refresh.log"
  cp "$new_prs" "$queue_file"
  cat "$queue_file" >> "$QUEUED_PRS_FILE"
  sort -u "$QUEUED_PRS_FILE" -o "$QUEUED_PRS_FILE"

  local pr_count
  pr_count="$(wc -l < "$queue_file" | tr -d ' ')"
  git fetch origin "$BASE_BRANCH" >/dev/null 2>&1 || true
  local before_remote_head
  before_remote_head="$(git rev-parse "origin/${BASE_BRANCH}" 2>/dev/null || git rev-parse "$BASE_BRANCH")"
  echo
  echo "Merge queue ${merge_id}: reviewing ${pr_count} PR(s)."
  PR_URL_FILE="$queue_file" \
  BASE_BRANCH="$BASE_BRANCH" \
    "$ROOT/scripts/aws_card_fixers/merge_batch_prs.sh" | tee -a "$merge_log"
  controller_disk_hygiene post_merge
  git fetch origin "$BASE_BRANCH" >/dev/null 2>&1 || true
  local after_remote_head
  after_remote_head="$(git rev-parse "origin/${BASE_BRANCH}" 2>/dev/null || git rev-parse "$BASE_BRANCH")"
  local main_changed=0
  if [[ "$before_remote_head" != "$after_remote_head" ]]; then
    main_changed=1
  fi

  cat "$queue_file" >> "$MERGED_PRS_FILE"
  sort -u "$MERGED_PRS_FILE" -o "$MERGED_PRS_FILE"

  if [[ "$REFRESH_AFTER_MERGE" == "1" && "$main_changed" == "1" ]]; then
    merged_since_bake=$((merged_since_bake + pr_count))
    bake_ami=0
    if (( BAKE_EVERY_MERGED_PRS > 0 && merged_since_bake >= BAKE_EVERY_MERGED_PRS )); then
      bake_ami=1
      merged_since_bake=0
    fi
    echo "Refreshing after ${merge_id}; BAKE_AMI=${bake_ami}."
    BASE_BRANCH="$BASE_BRANCH" \
    DB_PATH="$DB_PATH" \
    PULL_MAIN=1 \
    SYNC_DB=1 \
    BAKE_AMI="$bake_ami" \
      "$ROOT/scripts/aws_card_fixers/refresh_after_merge.sh" | tee -a "$refresh_log"
  elif [[ "$REFRESH_AFTER_MERGE" == "1" ]]; then
    echo "No origin/${BASE_BRANCH} change after ${merge_id}; skipping post-merge refresh."
  fi

  return 0
}

final_bake_if_requested() {
  if [[ "$BAKE_ON_STOP" != "1" || "$REFRESH_AFTER_MERGE" != "1" ]]; then
    return
  fi
  if (( merged_since_bake <= 0 )); then
    return
  fi
  echo "Final opportunistic bake for ${merged_since_bake} merged PR(s) since last bake."
  BASE_BRANCH="$BASE_BRANCH" \
  DB_PATH="$DB_PATH" \
  PULL_MAIN=1 \
  SYNC_DB=0 \
  BAKE_AMI=1 \
    "$ROOT/scripts/aws_card_fixers/refresh_after_merge.sh" | tee -a "${RUN_DIR}/final-bake.log"
}

processed=0
wave=1
if [[ "$RESUME_RUN" == "1" ]]; then
  wave="$(
    python3 - "$RUN_DIR" <<'PY'
import pathlib
import re
import sys

run_dir = pathlib.Path(sys.argv[1])
max_wave = 0
pattern = re.compile(r"wave(\d{4})")
for path in run_dir.glob("*wave[0-9][0-9][0-9][0-9]*"):
    match = pattern.search(path.name)
    if match:
        max_wave = max(max_wave, int(match.group(1)))
print(max_wave + 1)
PY
  )"
fi

while true; do
  poll_sessions

  if merge_ready_prs; then
    :
  fi

  close_resolved_resume_prs

  active="$(cat "${RUN_DIR}/active-workers.txt")"
  eligible="$(eligible_count)"
  if (( eligible == 0 && active == 0 )); then
    echo "No eligible parse-failing cards remain and no workers are active."
    final_bake_if_requested
    break
  fi

  remaining_allowed="$MAX_ACTIVE_WORKERS"
  if (( MAX_TOTAL_CARDS > 0 )); then
    remaining_allowed=$((MAX_TOTAL_CARDS - processed))
    if (( remaining_allowed <= 0 )); then
      echo "Reached MAX_TOTAL_CARDS=${MAX_TOTAL_CARDS}; stopping after active workers drain."
      if (( active == 0 )); then
        final_bake_if_requested
        break
      fi
      sleep "$POLL_INTERVAL"
      continue
    fi
  fi

  deficit=$((MAX_ACTIVE_WORKERS - active))
  launch_count="$(min_int "$deficit" "$eligible" "$remaining_allowed")"
  if (( launch_count > 0 )); then
    launch_workers "$launch_count" "$wave"
    processed=$((processed + launch_count))
    wave=$((wave + 1))
    if [[ "$DRY_RUN" == "1" ]]; then
      break
    fi
    poll_sessions
  else
    echo "No replacement launch this poll: active=${active}, eligible=${eligible}, deficit=${deficit}."
  fi

  sleep "$POLL_INTERVAL"
done

echo
echo "Rolling loop complete. Launched ${processed} card(s)."
