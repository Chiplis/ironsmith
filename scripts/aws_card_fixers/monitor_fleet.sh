#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AWS_REGION="${AWS_REGION:-$(aws configure get region 2>/dev/null || true)}"
AWS_REGION="${AWS_REGION:-us-east-2}"
S3_ASSET_PREFIX="${S3_ASSET_PREFIX:-}"
WATCH_INTERVAL="${WATCH_INTERVAL:-0}"
DB_PATH="${DB_PATH:-$ROOT/reports/engine-status.sqlite3}"
SYNC_PR_CREATED="${SYNC_PR_CREATED:-1}"
TERMINATE_TERMINAL_INSTANCES="${TERMINATE_TERMINAL_INSTANCES:-1}"
CLEANUP_EXPIRED_INSTANCES="${CLEANUP_EXPIRED_INSTANCES:-1}"
CLEANUP_EXPIRED_PROJECT_INSTANCES="${CLEANUP_EXPIRED_PROJECT_INSTANCES:-1}"
SESSION_ID="${S3_ASSET_PREFIX%/}"
SESSION_ID="${SESSION_ID##*/}"

usage() {
  cat <<EOF
Usage:
  S3_ASSET_PREFIX=s3://bucket/session scripts/aws_card_fixers/monitor_fleet.sh

Optional env:
  AWS_PROFILE=AdministratorAccess-550204982899
  AWS_REGION=us-east-2
  WATCH_INTERVAL=30
  DB_PATH=reports/engine-status.sqlite3
  SYNC_PR_CREATED=1
  TERMINATE_TERMINAL_INSTANCES=1
  CLEANUP_EXPIRED_INSTANCES=1
  CLEANUP_EXPIRED_PROJECT_INSTANCES=1

Set WATCH_INTERVAL to poll repeatedly.
Set SYNC_PR_CREATED=0 to render status without updating local DB PR/running flags.
Set TERMINATE_TERMINAL_INSTANCES=0 to avoid terminating complete/failed worker instances.
Set CLEANUP_EXPIRED_INSTANCES=0 to disable TTL cleanup for this fleet session.
Set CLEANUP_EXPIRED_PROJECT_INSTANCES=0 to disable global Ironsmith TTL cleanup.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ -z "$S3_ASSET_PREFIX" ]]; then
  echo "S3_ASSET_PREFIX is required, for example s3://ironsmith-card-fixers-ACCOUNT-us-east-2/20260522T000000Z" >&2
  exit 2
fi

AWS=(aws)
if [[ -n "${AWS_PROFILE:-}" ]]; then
  AWS+=(--profile "$AWS_PROFILE")
fi
AWS+=(--region "$AWS_REGION")

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

sync_status_to_db() {
  if [[ "$SYNC_PR_CREATED" == "0" ]]; then
    return
  fi
  if [[ ! -f "$DB_PATH" ]]; then
    return
  fi

  python3 - "$TMPDIR/status" "$DB_PATH" <<'PY'
import json
import pathlib
import sqlite3
import sys

status_dir = pathlib.Path(sys.argv[1])
db_path = pathlib.Path(sys.argv[2])
completed_cards = []
terminal_cards = []

for path in sorted(status_dir.glob("*.json")):
    try:
        row = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        continue
    card_name = row.get("card_name")
    if not card_name:
        continue
    if row.get("state") in {"failed", "skipped"} or (
        row.get("state") == "succeeded" and row.get("step") == "complete"
    ):
        terminal_cards.append(card_name)
    if (
        row.get("state") == "succeeded"
        and row.get("step") == "complete"
        and row.get("pr_url")
    ):
        completed_cards.append(card_name)

if not completed_cards and not terminal_cards:
    raise SystemExit(0)

conn = sqlite3.connect(db_path, timeout=60)
try:
    conn.execute("PRAGMA busy_timeout = 60000")
    columns = {
        row[1]
        for row in conn.execute("PRAGMA table_info(latest_card_observation)")
    }
    if "pr_created" not in columns:
        conn.execute(
            "ALTER TABLE latest_card_observation "
            "ADD COLUMN pr_created INTEGER NOT NULL DEFAULT 0"
        )
        conn.executescript(
            """
            DROP VIEW IF EXISTS latest_card_compilation;
            CREATE VIEW latest_card_compilation AS
            SELECT cc.*, latest.agent_running, latest.pr_created
            FROM latest_card_observation latest
            JOIN card_compilation cc
            ON cc.id = latest.compilation_id;
            """
        )
        version = conn.execute("PRAGMA user_version").fetchone()[0]
        if version < 11:
            conn.execute("PRAGMA user_version = 11")

    updated = []
    cleared = []
    for card_name in sorted(set(terminal_cards)):
        cursor = conn.execute(
            "UPDATE latest_card_observation "
            "SET agent_running = 0 "
            "WHERE card_name = ?1 AND COALESCE(agent_running, 0) != 0",
            (card_name,),
        )
        if cursor.rowcount:
            cleared.append(card_name)

    for card_name in sorted(set(completed_cards)):
        cursor = conn.execute(
            "UPDATE latest_card_observation "
            "SET pr_created = 1 "
            "WHERE card_name = ?1 AND COALESCE(pr_created, 0) = 0",
            (card_name,),
        )
        if cursor.rowcount:
            updated.append(card_name)
    conn.commit()
finally:
    conn.close()

if updated:
    print("Marked pr_created=1 for: " + ", ".join(sorted(updated)))
if cleared:
    print("Cleared agent_running for: " + ", ".join(sorted(cleared)))
PY
}

cleanup_instances() {
  local ids_file="$TMPDIR/terminate-instance-ids.txt"
  : > "$ids_file"

  if [[ "$TERMINATE_TERMINAL_INSTANCES" != "0" ]] && compgen -G "$TMPDIR/status/*.json" >/dev/null; then
    python3 - "$TMPDIR/status" >> "$ids_file" <<'PY'
import json
import pathlib
import sys

status_dir = pathlib.Path(sys.argv[1])
for path in sorted(status_dir.glob("*.json")):
    try:
        row = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        continue
    instance_id = row.get("instance_id", "")
    if not instance_id.startswith("i-"):
        continue
    if row.get("state") in {"failed", "skipped"} or (
        row.get("state") == "succeeded" and row.get("step") == "complete"
    ):
        print(instance_id)
PY
  fi

  if [[ "$CLEANUP_EXPIRED_INSTANCES" != "0" && -n "$SESSION_ID" ]]; then
    local instances_json="$TMPDIR/session-instances.json"
    "${AWS[@]}" ec2 describe-instances \
      --filters "Name=tag:IronsmithSession,Values=${SESSION_ID}" \
                "Name=instance-state-name,Values=pending,running,stopping,stopped" \
      --query 'Reservations[].Instances[].{Id:InstanceId,State:State.Name,Tags:Tags}' \
      --output json > "$instances_json" 2>/dev/null || true
    python3 - "$instances_json" >> "$ids_file" <<'PY'
import datetime as dt
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
if not path.exists() or not path.read_text(encoding="utf-8").strip():
    raise SystemExit(0)

instances = json.loads(path.read_text(encoding="utf-8"))
now = dt.datetime.now(dt.timezone.utc)
for instance in instances:
    tags = {
        tag.get("Key"): tag.get("Value")
        for tag in instance.get("Tags", [])
    }
    expires_at = tags.get("IronsmithExpiresAt")
    if not expires_at:
        continue
    try:
        parsed = dt.datetime.fromisoformat(expires_at.replace("Z", "+00:00"))
    except ValueError:
        continue
    if parsed <= now:
        print(instance["Id"])
PY
  fi

  if [[ "$CLEANUP_EXPIRED_PROJECT_INSTANCES" != "0" ]]; then
    local project_instances_json="$TMPDIR/project-instances.json"
    "${AWS[@]}" ec2 describe-instances \
      --filters "Name=tag:Project,Values=ironsmith-card-fixer" \
                "Name=instance-state-name,Values=pending,running,stopping,stopped" \
      --query 'Reservations[].Instances[].{Id:InstanceId,State:State.Name,Tags:Tags}' \
      --output json > "$project_instances_json" 2>/dev/null || true
    python3 - "$project_instances_json" >> "$ids_file" <<'PY'
import datetime as dt
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
if not path.exists() or not path.read_text(encoding="utf-8").strip():
    raise SystemExit(0)

instances = json.loads(path.read_text(encoding="utf-8"))
now = dt.datetime.now(dt.timezone.utc)
for instance in instances:
    tags = {
        tag.get("Key"): tag.get("Value")
        for tag in instance.get("Tags", [])
    }
    expires_at = tags.get("IronsmithExpiresAt")
    if not expires_at:
        continue
    try:
        parsed = dt.datetime.fromisoformat(expires_at.replace("Z", "+00:00"))
    except ValueError:
        continue
    if parsed <= now:
        print(instance["Id"])
PY
  fi

  if [[ ! -s "$ids_file" ]]; then
    return
  fi

  local deduped_ids_file="$TMPDIR/terminate-instance-ids.dedup.txt"
  sort -u "$ids_file" > "$deduped_ids_file"

  local ids=()
  local instance_id
  while IFS= read -r instance_id; do
    [[ -n "$instance_id" ]] || continue
    ids+=("$instance_id")
  done < "$deduped_ids_file"

  if [[ "${#ids[@]}" -eq 0 ]]; then
    return
  fi

  "${AWS[@]}" ec2 terminate-instances --instance-ids "${ids[@]}" >/dev/null 2>&1 || true
  printf 'Termination requested for: %s\n' "${ids[*]}"
}

mark_dead_status_instances() {
  if ! compgen -G "$TMPDIR/status/*.json" >/dev/null; then
    return
  fi

  local instance_ids_file="$TMPDIR/status-instance-ids.txt"
  python3 - "$TMPDIR/status" > "$instance_ids_file" <<'PY'
import json
import pathlib
import sys

status_dir = pathlib.Path(sys.argv[1])
for path in sorted(status_dir.glob("*.json")):
    try:
        row = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        continue
    if row.get("state") in {"failed", "skipped"} or (
        row.get("state") == "succeeded" and row.get("step") == "complete"
    ):
        continue
    instance_id = str(row.get("instance_id", ""))
    if instance_id.startswith("i-"):
        print(instance_id)
PY

  if [[ ! -s "$instance_ids_file" ]]; then
    return
  fi

  local instances_json="$TMPDIR/status-instances.json"
  "${AWS[@]}" ec2 describe-instances \
    --instance-ids $(sort -u "$instance_ids_file") \
    --query 'Reservations[].Instances[].{Id:InstanceId,State:State.Name,Reason:StateTransitionReason}' \
    --output json > "$instances_json" 2>/dev/null || echo '[]' > "$instances_json"

  local changed_files="$TMPDIR/dead-status-files.txt"
  python3 - "$TMPDIR/status" "$instances_json" > "$changed_files" <<'PY'
import datetime as dt
import json
import pathlib
import sys

status_dir = pathlib.Path(sys.argv[1])
instances_path = pathlib.Path(sys.argv[2])
instances = {
    row.get("Id"): row
    for row in json.loads(instances_path.read_text(encoding="utf-8") or "[]")
}
dead_states = {"shutting-down", "terminated", "stopping", "stopped"}
changed = []

for path in sorted(status_dir.glob("*.json")):
    try:
        row = json.loads(path.read_text(encoding="utf-8"))
    except Exception:
        continue
    if row.get("state") in {"failed", "skipped"} or (
        row.get("state") == "succeeded" and row.get("step") == "complete"
    ):
        continue
    instance_id = row.get("instance_id")
    instance = instances.get(instance_id)
    state = instance.get("State") if instance else None
    if state not in dead_states:
        continue
    reason = instance.get("Reason") or f"EC2 instance is {state}"
    row["timestamp"] = dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    row["state"] = "failed"
    row["step"] = "instance_interrupted"
    row["message"] = reason
    path.write_text(json.dumps(row, sort_keys=True) + "\n", encoding="utf-8")
    changed.append(path)

for path in changed:
    print(path.name)
PY

  local changed
  while IFS= read -r changed; do
    [[ -n "$changed" ]] || continue
    "${AWS[@]}" s3 cp "$TMPDIR/status/$changed" "${S3_ASSET_PREFIX}/status/$changed" >/dev/null 2>&1 || true
  done < "$changed_files"
}

render_once() {
  rm -rf "$TMPDIR/status"
  mkdir -p "$TMPDIR/status"
  "${AWS[@]}" s3 cp --recursive "${S3_ASSET_PREFIX}/status/" "$TMPDIR/status/" >/dev/null 2>&1 || true

  printf 'Status prefix: %s/status/\n' "$S3_ASSET_PREFIX"
  printf 'Updated: %s\n\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  mark_dead_status_instances
  cleanup_instances

  if ! compgen -G "$TMPDIR/status/*.json" >/dev/null; then
    echo "No worker status files yet."
    return
  fi

  sync_status_to_db
  printf '\n'

  python3 - "$TMPDIR/status" <<'PY'
import json
import pathlib
import sys

status_dir = pathlib.Path(sys.argv[1])
rows = []
for path in sorted(status_dir.glob("*.json")):
    try:
        rows.append(json.loads(path.read_text()))
    except Exception as exc:
        rows.append({"instance_id": path.stem, "state": "unreadable", "step": "status_parse", "message": str(exc)})

headers = ["state", "step", "instance_id", "card_name", "message", "pr_url", "timestamp"]
widths = {header: len(header) for header in headers}
for row in rows:
    for header in headers:
        value = str(row.get(header, ""))
        if header == "message" and len(value) > 72:
            value = value[:69] + "..."
        widths[header] = max(widths[header], len(value))

line = "  ".join(header.ljust(widths[header]) for header in headers)
print(line)
print("  ".join("-" * widths[header] for header in headers))
for row in rows:
    values = []
    for header in headers:
        value = str(row.get(header, ""))
        if header == "message" and len(value) > 72:
            value = value[:69] + "..."
        values.append(value.ljust(widths[header]))
    print("  ".join(values))

states = {}
for row in rows:
    if row.get("state") == "failed":
        state = "failed"
    elif row.get("state") == "succeeded" and row.get("step") == "complete":
        state = "complete"
    elif row.get("state") == "unreadable":
        state = "unreadable"
    else:
        state = "running"
    states[state] = states.get(state, 0) + 1
print()
print("Summary: " + ", ".join(f"{state}={count}" for state, count in sorted(states.items())))
PY
}

while true; do
  render_once
  if [[ "$WATCH_INTERVAL" == "0" ]]; then
    break
  fi
  sleep "$WATCH_INTERVAL"
  printf '\n'
done
