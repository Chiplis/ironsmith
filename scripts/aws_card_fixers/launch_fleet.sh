#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT/scripts/aws_card_fixers/cost_controls.sh"
DB_PATH="${DB_PATH:-$ROOT/reports/engine-status.sqlite3}"
RESUME_CARDS_FILE="${RESUME_CARDS_FILE:-}"
CARDS_PATH="${CARDS_PATH:-$ROOT/cards.json}"
INSTANCE_COUNT="${INSTANCE_COUNT:-5}"
WORKER_ARCH="${WORKER_ARCH:-arm64}"
if [[ "$WORKER_ARCH" != "x86_64" && "$WORKER_ARCH" != "arm64" ]]; then
  echo "WORKER_ARCH must be x86_64 or arm64." >&2
  exit 2
fi
if [[ -z "${INSTANCE_TYPE+x}" ]]; then
  if [[ "$WORKER_ARCH" == "arm64" ]]; then
    INSTANCE_TYPE="t4g.medium"
    DEFAULT_INSTANCE_TYPES="t4g.medium t4g.large c8g.large c7g.large c6g.large m7g.large m6g.large r7g.large r6g.large"
  else
    INSTANCE_TYPE="c7i.large"
    DEFAULT_INSTANCE_TYPES="c7i.large c7a.large c6a.large c6i.large m7a.large m7i.large m6a.large m6i.large r7a.large r7i.large"
  fi
else
  DEFAULT_INSTANCE_TYPES="$INSTANCE_TYPE"
fi
INSTANCE_TYPES="${INSTANCE_TYPES:-$DEFAULT_INSTANCE_TYPES}"
USE_SPOT="${USE_SPOT:-1}"
USE_EC2_FLEET="${USE_EC2_FLEET:-1}"
SPOT_MAX_PRICE="${SPOT_MAX_PRICE:-}"
BURSTABLE_CPU_CREDITS="${BURSTABLE_CPU_CREDITS:-standard}"
WORKER_VOLUME_SIZE_GB="${WORKER_VOLUME_SIZE_GB:-60}"
AWS_REGION="${AWS_REGION:-$(aws configure get region 2>/dev/null || true)}"
AWS_REGION="${AWS_REGION:-us-east-2}"
AWS_PROFILE_ARG=()
if [[ -n "${AWS_PROFILE:-}" ]]; then
  AWS_PROFILE_ARG=(--profile "$AWS_PROFILE")
fi
GITHUB_REPO="${GITHUB_REPO:-Chiplis/ironsmith}"
BASE_BRANCH="${BASE_BRANCH:-main}"
SESSION_ID="${SESSION_ID:-$(date -u +%Y%m%dT%H%M%SZ)}"
ROLE_NAME="${ROLE_NAME:-ironsmith-card-fixer-worker-role}"
PROFILE_NAME="${PROFILE_NAME:-ironsmith-card-fixer-worker-profile}"
SECURITY_GROUP_NAME="${SECURITY_GROUP_NAME:-ironsmith-card-fixer-workers}"
BUCKET="${BUCKET:-}"
GITHUB_TOKEN_SSM_PARAM="${GITHUB_TOKEN_SSM_PARAM:-}"
OPENCODE_AUTH_JSON_SSM_PARAM="${OPENCODE_AUTH_JSON_SSM_PARAM:-}"
OPENAI_API_KEY_SSM_PARAM="${OPENAI_API_KEY_SSM_PARAM:-}"
OPENCODE_VERSION="${OPENCODE_VERSION:-1.15.9}"
OPENCODE_MODEL="${OPENCODE_MODEL:-openai/gpt-5.5-fast}"
OPENCODE_VARIANT="${OPENCODE_VARIANT:-fast}"
OPENCODE_FAST_REASONING_EFFORT="${OPENCODE_FAST_REASONING_EFFORT:-high}"
OPENCODE_FAST_TEXT_VERBOSITY="${OPENCODE_FAST_TEXT_VERBOSITY:-low}"
OPENCODE_FAST_SERVICE_TIER="${OPENCODE_FAST_SERVICE_TIER:-priority}"
OPENCODE_STALE_TIMEOUT_SECONDS="${OPENCODE_STALE_TIMEOUT_SECONDS:-1800}"
OPENCODE_HEARTBEAT_SECONDS="${OPENCODE_HEARTBEAT_SECONDS:-60}"
OPENCODE_NO_COMMIT_RETRIES="${OPENCODE_NO_COMMIT_RETRIES:-1}"
POST_PR_STEWARD_MAX_REPAIRS="${POST_PR_STEWARD_MAX_REPAIRS:-3}"
WORKER_ENTRY_SKILL="${WORKER_ENTRY_SKILL:-ironsmith-aws-card-fixer-fleet}"
WORKER_AMI_ID="${WORKER_AMI_ID:-}"
if [[ "$WORKER_ARCH" == "arm64" ]]; then
  DEFAULT_WORKER_AMI_SSM_PARAM="/ironsmith/card-fixer-worker-ami-arm64"
  DEFAULT_SOURCE_AMI_SSM_PARAM="/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64"
else
  DEFAULT_WORKER_AMI_SSM_PARAM="/ironsmith/card-fixer-worker-ami"
  DEFAULT_SOURCE_AMI_SSM_PARAM="/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64"
fi
WORKER_AMI_SSM_PARAM="${WORKER_AMI_SSM_PARAM:-$DEFAULT_WORKER_AMI_SSM_PARAM}"
SOURCE_AMI_SSM_PARAM="${SOURCE_AMI_SSM_PARAM:-$DEFAULT_SOURCE_AMI_SSM_PARAM}"
REQUIRE_WORKER_AMI="${REQUIRE_WORKER_AMI:-1}"
INSTANCE_TTL_HOURS="${INSTANCE_TTL_HOURS:-6}"
SELF_TERMINATE="${SELF_TERMINATE:-1}"
S3_SESSION_PREFIX="${S3_SESSION_PREFIX:-sessions}"

usage() {
  cat <<EOF
Usage:
  AWS_PROFILE=... \\
  GITHUB_TOKEN_SSM_PARAM=/ironsmith/github-token \\
  OPENCODE_AUTH_JSON_SSM_PARAM=/opencode/auth-json \\
  $0

Optional env:
  INSTANCE_COUNT=5
  WORKER_ARCH=arm64
  INSTANCE_TYPE=t4g.medium
  INSTANCE_TYPES="t4g.medium c7g.large c6g.large t4g.large"
  USE_SPOT=1
  USE_EC2_FLEET=1
  SPOT_MAX_PRICE=
  BURSTABLE_CPU_CREDITS=standard
  WORKER_VOLUME_SIZE_GB=60
  AWS_REGION=us-east-2
  BUCKET=existing-or-new-bucket-name
  DB_PATH=reports/engine-status.sqlite3
  CARDS_PATH=cards.json
  GITHUB_REPO=Chiplis/ironsmith
  BASE_BRANCH=main
  OPENCODE_AUTH_JSON_SSM_PARAM=/opencode/auth-json
  OPENAI_API_KEY_SSM_PARAM=/opencode/openai-api-key
  OPENCODE_VERSION=1.15.9
  OPENCODE_MODEL=openai/gpt-5.5-fast
  OPENCODE_VARIANT=fast
  OPENCODE_FAST_REASONING_EFFORT=high
  OPENCODE_FAST_TEXT_VERBOSITY=low
  OPENCODE_FAST_SERVICE_TIER=priority
  OPENCODE_STALE_TIMEOUT_SECONDS=1800
  OPENCODE_HEARTBEAT_SECONDS=60
  OPENCODE_NO_COMMIT_RETRIES=1
  POST_PR_STEWARD_MAX_REPAIRS=3
  WORKER_ENTRY_SKILL=ironsmith-aws-card-fixer-fleet
  WORKER_AMI_ID=ami-...
  WORKER_AMI_SSM_PARAM=/ironsmith/card-fixer-worker-ami-arm64
  SOURCE_AMI_SSM_PARAM=/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64
  REQUIRE_WORKER_AMI=1
  INSTANCE_TTL_HOURS=6
  SELF_TERMINATE=1
  S3_SESSION_PREFIX=sessions

Provide exactly one OpenCode auth parameter: OPENCODE_AUTH_JSON_SSM_PARAM or OPENAI_API_KEY_SSM_PARAM.

Set WORKER_ARCH=arm64 to launch ARM workers. That defaults to:
  INSTANCE_TYPES="t4g.medium c7g.large c6g.large t4g.large"
  WORKER_AMI_SSM_PARAM=/ironsmith/card-fixer-worker-ami-arm64
  SOURCE_AMI_SSM_PARAM=/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64
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
if [[ ! -f "$CARDS_PATH" ]]; then
  echo "cards.json not found: $CARDS_PATH" >&2
  exit 2
fi
if [[ -z "$GITHUB_TOKEN_SSM_PARAM" ]]; then
  echo "GITHUB_TOKEN_SSM_PARAM is required so workers can push branches and open PRs." >&2
  exit 2
fi
if [[ -z "$OPENCODE_AUTH_JSON_SSM_PARAM" && -z "$OPENAI_API_KEY_SSM_PARAM" ]]; then
  echo "An OpenCode auth SSM parameter is required: OPENCODE_AUTH_JSON_SSM_PARAM or OPENAI_API_KEY_SSM_PARAM." >&2
  exit 2
fi
if [[ -n "$OPENCODE_AUTH_JSON_SSM_PARAM" && -n "$OPENAI_API_KEY_SSM_PARAM" ]]; then
  echo "Provide only one OpenCode auth parameter, not both." >&2
  exit 2
fi
if [[ "$USE_SPOT" != "0" && "$USE_SPOT" != "1" ]]; then
  echo "USE_SPOT must be 0 or 1." >&2
  exit 2
fi
if [[ "$USE_EC2_FLEET" != "0" && "$USE_EC2_FLEET" != "1" ]]; then
  echo "USE_EC2_FLEET must be 0 or 1." >&2
  exit 2
fi
if [[ "$BURSTABLE_CPU_CREDITS" != "standard" && "$BURSTABLE_CPU_CREDITS" != "unlimited" ]]; then
  echo "BURSTABLE_CPU_CREDITS must be standard or unlimited." >&2
  exit 2
fi

AWS=(aws)
if [[ -n "${AWS_PROFILE:-}" ]]; then
  AWS+=(--profile "$AWS_PROFILE")
fi
AWS+=(--region "$AWS_REGION")

TMPDIR="$(mktemp -d)"
RESERVED_ROWS_FILE="$TMPDIR/reserved-cards.tsv"
LAUNCHED_CARDS_FILE="$TMPDIR/launched-cards.txt"
: > "$RESERVED_ROWS_FILE"
: > "$LAUNCHED_CARDS_FILE"

release_unlaunched_reservations() {
  if [[ ! -s "$RESERVED_ROWS_FILE" ]]; then
    return
  fi
  python3 - "$DB_PATH" "$RESERVED_ROWS_FILE" "$LAUNCHED_CARDS_FILE" <<'PY' || true
import pathlib
import sqlite3
import sys

db_path = pathlib.Path(sys.argv[1])
reserved_path = pathlib.Path(sys.argv[2])
launched_path = pathlib.Path(sys.argv[3])

reserved = []
for line in reserved_path.read_text(encoding="utf-8").splitlines():
    if not line:
        continue
    reserved.append(line.split("\t", 1)[0])

launched = set()
if launched_path.exists():
    launched = {
        line
        for line in launched_path.read_text(encoding="utf-8").splitlines()
        if line
    }

to_release = [card_name for card_name in reserved if card_name not in launched]
if not to_release:
    raise SystemExit(0)

conn = sqlite3.connect(db_path, timeout=30)
try:
    conn.executemany(
        "UPDATE latest_card_observation "
        "SET agent_running = 0 "
        "WHERE card_name = ?1 AND COALESCE(pr_created, 0) = 0",
        [(card_name,) for card_name in to_release],
    )
    conn.commit()
finally:
    conn.close()

print(
    "Released unlaunched card reservations: " + ", ".join(sorted(to_release)),
    file=sys.stderr,
)
PY
}

cleanup() {
  local exit_code=$?
  set +e
  if [[ "$exit_code" -ne 0 ]]; then
    release_unlaunched_reservations
  fi
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

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

ACCOUNT_ID="$("${AWS[@]}" sts get-caller-identity --query Account --output text)"
if [[ -z "$BUCKET" ]]; then
  BUCKET="ironsmith-card-fixers-${ACCOUNT_ID}-${AWS_REGION}"
fi
if [[ -n "$S3_SESSION_PREFIX" ]]; then
  ASSET_PREFIX="s3://${BUCKET}/${S3_SESSION_PREFIX}/${SESSION_ID}"
else
  ASSET_PREFIX="s3://${BUCKET}/${SESSION_ID}"
fi
EXPIRES_AT="$(INSTANCE_TTL_HOURS="$INSTANCE_TTL_HOURS" python3 - <<'PY'
import datetime as dt
import os

hours = float(os.environ["INSTANCE_TTL_HOURS"])
expires_at = dt.datetime.now(dt.timezone.utc) + dt.timedelta(hours=hours)
print(expires_at.replace(microsecond=0).isoformat().replace("+00:00", "Z"))
PY
)"

run_worker_instance_with_fleet() {
  local user_data="$1"
  local user_data_b64
  local launch_template_configs="$TMPDIR/fleet-launch-template-configs-${WORKER_INDEX}.json"
  local fleet_config="$TMPDIR/fleet-config-${WORKER_INDEX}.json"
  local fleet_response="$TMPDIR/fleet-response-${WORKER_INDEX}.json"
  local fleet_err="$TMPDIR/fleet-${WORKER_INDEX}.err"
  local instance_id
  local candidate_type
  local lt_id
  local lt_name
  local lt_data
  local launch_template_ids=()
  local market_label="spot"

  if [[ "$USE_SPOT" != "1" || "$USE_EC2_FLEET" != "1" ]]; then
    return 1
  fi

  user_data_b64="$(base64 < "$user_data" | tr -d '\n')"
  : > "$launch_template_configs"

  for candidate_type in $INSTANCE_TYPES; do
    lt_name="ironsmith-card-fixer-${SESSION_ID}-${WORKER_INDEX}-${candidate_type//./-}-$(date +%s%N)"
    lt_data="$TMPDIR/launch-template-${WORKER_INDEX}-${candidate_type//./-}.json"
    python3 - "$lt_data" \
      "$AMI_ID" \
      "$candidate_type" \
      "$PROFILE_NAME" \
      "$SG_ID" \
      "$user_data_b64" \
      "$WORKER_VOLUME_SIZE_GB" \
      "$BURSTABLE_CPU_CREDITS" \
      "$SESSION_ID" \
      "$EXPIRES_AT" \
      "$market_label" \
      "$WORKER_INDEX" <<'PY'
import json
import pathlib
import sys

(
    path,
    ami_id,
    instance_type,
    profile_name,
    security_group_id,
    user_data_b64,
    volume_size,
    burstable_cpu_credits,
    session_id,
    expires_at,
    market_label,
    worker_index,
) = sys.argv[1:]

data = {
    "ImageId": ami_id,
    "InstanceType": instance_type,
    "IamInstanceProfile": {"Name": profile_name},
    "SecurityGroupIds": [security_group_id],
    "UserData": user_data_b64,
    "InstanceInitiatedShutdownBehavior": "terminate",
    "BlockDeviceMappings": [
        {
            "DeviceName": "/dev/xvda",
            "Ebs": {
                "VolumeSize": int(volume_size),
                "VolumeType": "gp3",
                "DeleteOnTermination": True,
            },
        }
    ],
    "TagSpecifications": [
        {
            "ResourceType": "instance",
            "Tags": [
                {"Key": "Name", "Value": "ironsmith-card-fixer"},
                {"Key": "Project", "Value": "ironsmith-card-fixer"},
                {"Key": "IronsmithSession", "Value": session_id},
                {"Key": "IronsmithExpiresAt", "Value": expires_at},
                {"Key": "IronsmithMarket", "Value": market_label},
                {"Key": "IronsmithInstanceType", "Value": instance_type},
                {"Key": "IronsmithWorkerIndex", "Value": worker_index},
            ],
        },
        {
            "ResourceType": "volume",
            "Tags": [
                {"Key": "Name", "Value": "ironsmith-card-fixer"},
                {"Key": "Project", "Value": "ironsmith-card-fixer"},
                {"Key": "IronsmithSession", "Value": session_id},
                {"Key": "IronsmithExpiresAt", "Value": expires_at},
                {"Key": "IronsmithInstanceType", "Value": instance_type},
            ],
        },
    ],
}

if instance_type.startswith(("t2.", "t3.", "t3a.", "t4g.")):
    data["CreditSpecification"] = {"CpuCredits": burstable_cpu_credits}

pathlib.Path(path).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
    if lt_id="$("${AWS[@]}" ec2 create-launch-template \
      --launch-template-name "$lt_name" \
      --launch-template-data "file://${lt_data}" \
      --query 'LaunchTemplate.LaunchTemplateId' \
      --output text 2>>"$fleet_err")"; then
      launch_template_ids+=("$lt_id")
      python3 - "$launch_template_configs" "$lt_id" "${SUBNET_IDS[@]}" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
launch_template_id = sys.argv[2]
subnets = sys.argv[3:]
rows = []
if path.exists() and path.read_text(encoding="utf-8").strip():
    rows = json.loads(path.read_text(encoding="utf-8"))
rows.append(
    {
        "LaunchTemplateSpecification": {
            "LaunchTemplateId": launch_template_id,
            "Version": "$Latest",
        },
        "Overrides": [{"SubnetId": subnet_id} for subnet_id in subnets],
    }
)
path.write_text(json.dumps(rows, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY
    else
      echo "Could not create launch template for ${candidate_type}; excluding it from EC2 Fleet." >&2
    fi
  done

  if [[ "${#launch_template_ids[@]}" -eq 0 ]]; then
    echo "Could not create any launch templates for EC2 Fleet." >&2
    return 1
  fi

  python3 - "$fleet_config" "$launch_template_configs" "$SPOT_MAX_PRICE" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
launch_template_configs = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
spot_max_price = sys.argv[3]

spot_options = {
    "AllocationStrategy": "price-capacity-optimized",
    "InstanceInterruptionBehavior": "terminate",
}
if spot_max_price:
    spot_options["MaxTotalPrice"] = spot_max_price

config = {
    "Type": "instant",
    "TargetCapacitySpecification": {
        "TotalTargetCapacity": 1,
        "DefaultTargetCapacityType": "spot",
    },
    "SpotOptions": spot_options,
    "LaunchTemplateConfigs": launch_template_configs,
}
path.write_text(json.dumps(config, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

  if ! "${AWS[@]}" ec2 create-fleet \
    --cli-input-json "file://${fleet_config}" \
    --output json > "$fleet_response" 2>>"$fleet_err"; then
    echo "EC2 Fleet launch failed; falling back to run-instances." >&2
    sed 's/^/  /' "$fleet_err" >&2 || true
    for lt_id in "${launch_template_ids[@]}"; do
      "${AWS[@]}" ec2 delete-launch-template --launch-template-id "$lt_id" >/dev/null 2>&1 || true
    done
    return 1
  fi

  instance_id="$(python3 - "$fleet_response" <<'PY'
import json
import pathlib
import sys

data = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
for row in data.get("Instances", []):
    for instance_id in row.get("InstanceIds", []):
        if instance_id.startswith("i-"):
            print(instance_id)
            raise SystemExit(0)
raise SystemExit(1)
PY
  )" || true

  for lt_id in "${launch_template_ids[@]}"; do
    "${AWS[@]}" ec2 delete-launch-template --launch-template-id "$lt_id" >/dev/null 2>&1 || true
  done

  if [[ -z "$instance_id" ]]; then
    echo "EC2 Fleet did not return an instance id; falling back to run-instances." >&2
    sed 's/^/  /' "$fleet_response" >&2 || true
    return 1
  fi

  local instance_info
  instance_info="$("${AWS[@]}" ec2 describe-instances \
    --instance-ids "$instance_id" \
    --query 'Reservations[0].Instances[0].[InstanceType,SubnetId]' \
    --output text 2>/dev/null || true)"
  if [[ -n "$instance_info" && "$instance_info" != "None" ]]; then
    printf '%s\t%s\t%s\n' "$instance_id" $instance_info
  else
    printf '%s\t%s\t%s\n' "$instance_id" "ec2-fleet" "ec2-fleet"
  fi
  return 0
}

run_worker_instance() {
  local user_data="$1"
  local candidate_type
  local subnet_id
  local instance_id
  local market_label="on-demand"

  if [[ "$USE_SPOT" == "1" ]]; then
    market_label="spot"
  fi

  if run_worker_instance_with_fleet "$user_data"; then
    return 0
  fi

  for candidate_type in $INSTANCE_TYPES; do
    for subnet_id in "${SUBNET_IDS[@]}"; do
      local market_options=()
      local credit_specification=()
      local run_err="$TMPDIR/run-instances-${WORKER_INDEX}-${candidate_type}-${subnet_id}.err"

      if [[ "$USE_SPOT" == "1" ]]; then
        local spot_options="SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate"
        if [[ -n "$SPOT_MAX_PRICE" ]]; then
          spot_options="${spot_options},MaxPrice=${SPOT_MAX_PRICE}"
        fi
        market_options=(--instance-market-options "MarketType=spot,SpotOptions={${spot_options}}")
      fi

      case "$candidate_type" in
        t2.*|t3.*|t3a.*|t4g.*)
          credit_specification=(--credit-specification "CpuCredits=${BURSTABLE_CPU_CREDITS}")
          ;;
      esac

      if instance_id="$("${AWS[@]}" ec2 run-instances \
        --image-id "$AMI_ID" \
        --instance-type "$candidate_type" \
        "${market_options[@]}" \
        "${credit_specification[@]}" \
        --iam-instance-profile "Name=${PROFILE_NAME}" \
        --subnet-id "$subnet_id" \
        --security-group-ids "$SG_ID" \
        --user-data "file://${user_data}" \
        --instance-initiated-shutdown-behavior terminate \
        --block-device-mappings "DeviceName=/dev/xvda,Ebs={VolumeSize=${WORKER_VOLUME_SIZE_GB},VolumeType=gp3,DeleteOnTermination=true}" \
        --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=ironsmith-card-fixer},{Key=Project,Value=ironsmith-card-fixer},{Key=IronsmithSession,Value=${SESSION_ID}},{Key=IronsmithExpiresAt,Value=${EXPIRES_AT}},{Key=IronsmithMarket,Value=${market_label}},{Key=IronsmithInstanceType,Value=${candidate_type}},{Key=IronsmithSubnet,Value=${subnet_id}}]" \
          "ResourceType=volume,Tags=[{Key=Name,Value=ironsmith-card-fixer},{Key=Project,Value=ironsmith-card-fixer},{Key=IronsmithSession,Value=${SESSION_ID}},{Key=IronsmithExpiresAt,Value=${EXPIRES_AT}},{Key=IronsmithInstanceType,Value=${candidate_type}},{Key=IronsmithSubnet,Value=${subnet_id}}]" \
        --query 'Instances[0].InstanceId' \
        --output text 2>"$run_err")"; then
        printf '%s\t%s\t%s\n' "$instance_id" "$candidate_type" "$subnet_id"
        return 0
      fi

      echo "Could not launch ${candidate_type} in ${subnet_id}; trying next worker placement." >&2
      sed 's/^/  /' "$run_err" >&2 || true
    done
  done

  return 1
}

SELECTED_ROWS=()
if python3 - "$DB_PATH" "$INSTANCE_COUNT" "$RESUME_CARDS_FILE" > "$RESERVED_ROWS_FILE" <<'PY'
import sqlite3
import sys
from pathlib import Path

db_path = sys.argv[1]
requested = int(sys.argv[2])
resume_cards_file = sys.argv[3] if len(sys.argv) > 3 else ""

preferred_cards = []
if resume_cards_file:
    path = Path(resume_cards_file)
    if path.exists():
        seen = set()
        for raw in path.read_text(encoding="utf-8").splitlines():
            card_name = raw.strip()
            if not card_name or card_name in seen:
                continue
            seen.add(card_name)
            preferred_cards.append(card_name)

conn = sqlite3.connect(db_path, timeout=30)
conn.isolation_level = None
conn.execute("PRAGMA busy_timeout = 30000")

def ensure_pr_created_column() -> None:
    columns = {
        row[1]
        for row in conn.execute("PRAGMA table_info(latest_card_observation)")
    }
    if "pr_created" in columns:
        return
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

try:
    conn.execute("BEGIN IMMEDIATE")
    ensure_pr_created_column()
    rows = []
    if preferred_cards:
        placeholders = ",".join("?" for _ in preferred_cards)
        preferred_rows = conn.execute(
            f"""
            SELECT card_name, COALESCE(parse_error, '')
            FROM latest_card_compilation
            WHERE parse_status = 'parse_failed'
              AND COALESCE(agent_running, 0) = 0
              AND COALESCE(pr_created, 0) = 0
              AND card_name IN ({placeholders})
            """,
            preferred_cards,
        ).fetchall()
        by_name = {card_name: (card_name, parse_error) for card_name, parse_error in preferred_rows}
        for card_name in preferred_cards:
            row = by_name.get(card_name)
            if row:
                rows.append(row)
            if len(rows) >= requested:
                break

    if len(rows) < requested:
        already_selected = [card_name for card_name, _ in rows]
        excluded_clause = ""
        params = []
        if already_selected:
            excluded_clause = (
                "AND card_name NOT IN ("
                + ",".join("?" for _ in already_selected)
                + ")"
            )
            params.extend(already_selected)
        params.append(requested - len(rows))
        rows.extend(
            conn.execute(
                f"""
                SELECT card_name, COALESCE(parse_error, '')
                FROM latest_card_compilation
                WHERE parse_status = 'parse_failed'
                  AND COALESCE(agent_running, 0) = 0
                  AND COALESCE(pr_created, 0) = 0
                  {excluded_clause}
                ORDER BY random()
                LIMIT ?{len(params)}
                """,
                params,
            ).fetchall()
        )
    if len(rows) < requested:
        conn.execute("ROLLBACK")
        print(
            f"Only found {len(rows)} parse-failing cards; requested {requested}.",
            file=sys.stderr,
        )
        raise SystemExit(3)

    conn.executemany(
        "UPDATE latest_card_observation SET agent_running = 1 WHERE card_name = ?1",
        [(card_name,) for card_name, _ in rows],
    )
    conn.execute("COMMIT")
finally:
    conn.close()

for card_name, parse_error in rows:
    safe_error = parse_error.replace("\t", " ").replace("\n", " ")
    print(f"{card_name}\t{safe_error}")
PY
then
  :
else
  exit_code=$?
  exit "$exit_code"
fi

while IFS= read -r row; do
  SELECTED_ROWS+=("$row")
done < "$RESERVED_ROWS_FILE"

echo "Selected cards:"
printf '  %s\n' "${SELECTED_ROWS[@]%%$'\t'*}"

if ! "${AWS[@]}" s3api head-bucket --bucket "$BUCKET" >/dev/null 2>&1; then
  if [[ "$AWS_REGION" == "us-east-1" ]]; then
    "${AWS[@]}" s3api create-bucket --bucket "$BUCKET" >/dev/null
  else
    "${AWS[@]}" s3api create-bucket \
      --bucket "$BUCKET" \
      --create-bucket-configuration "LocationConstraint=${AWS_REGION}" >/dev/null
  fi
fi
ironsmith_ensure_bucket_lifecycle "$BUCKET" "$TMPDIR"
ironsmith_publish_cards_asset "$BUCKET" "$CARDS_PATH" "$TMPDIR"

SKILL_NAMES=()
for skill_dir in "$HOME"/.codex/skills/ironsmith-*; do
  [[ -e "$skill_dir" ]] || continue
  SKILL_NAMES+=("$(basename "$skill_dir")")
done
if [[ "${#SKILL_NAMES[@]}" -eq 0 ]]; then
  echo "No local Ironsmith skills found under $HOME/.codex/skills." >&2
  exit 2
fi

REQUIRED_WORKER_SKILLS=(
  "$WORKER_ENTRY_SKILL"
  ironsmith-card-fixer
  ironsmith-card-text
  ironsmith-query
  ironsmith-engine-debug
  ironsmith-text-normalizer
  ironsmith-parser-only-card-fix
  ironsmith-effect-creator
  ironsmith-triggered-ability-fix
  ironsmith-continuous-effects-fix
  ironsmith-costs-and-targeting-fix
  ironsmith-replacement-prevention-fix
  ironsmith-tester
)
for skill_name in "${REQUIRED_WORKER_SKILLS[@]}"; do
  if [[ ! -f "$HOME/.codex/skills/${skill_name}/SKILL.md" ]]; then
    echo "Required worker skill is missing: $HOME/.codex/skills/${skill_name}/SKILL.md" >&2
    exit 2
  fi
done

printf '%s\n' "${SKILL_NAMES[@]}" | sort > "$TMPDIR/ironsmith-skills-manifest.txt"
controller_disk_hygiene pre_launch
tar -C "$HOME/.codex/skills" -czf "$TMPDIR/ironsmith-skills.tar.gz" "${SKILL_NAMES[@]}"
{
  printf 'card_name\tparse_error\n'
  printf '%s\n' "${SELECTED_ROWS[@]}"
} > "$TMPDIR/selected-cards.tsv"
printf '%s\n' "$CARDS_S3_URI" > "$TMPDIR/cards-uri.txt"
"${AWS[@]}" s3 cp "$TMPDIR/ironsmith-skills.tar.gz" "${ASSET_PREFIX}/ironsmith-skills.tar.gz" >/dev/null
"${AWS[@]}" s3 cp "$TMPDIR/ironsmith-skills-manifest.txt" "${ASSET_PREFIX}/ironsmith-skills-manifest.txt" >/dev/null
"${AWS[@]}" s3 cp "$TMPDIR/selected-cards.tsv" "${ASSET_PREFIX}/selected-cards.tsv" >/dev/null
"${AWS[@]}" s3 cp "$TMPDIR/cards-uri.txt" "${ASSET_PREFIX}/cards-uri.txt" >/dev/null

ROLE_EXISTS=0
"${AWS[@]}" iam get-role --role-name "$ROLE_NAME" >/dev/null 2>&1 || ROLE_EXISTS=1
if [[ "$ROLE_EXISTS" -ne 0 ]]; then
  cat > "$TMPDIR/trust-policy.json" <<'JSON'
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": { "Service": "ec2.amazonaws.com" },
      "Action": "sts:AssumeRole"
    }
  ]
}
JSON
  "${AWS[@]}" iam create-role \
    --role-name "$ROLE_NAME" \
    --assume-role-policy-document "file://$TMPDIR/trust-policy.json" >/dev/null
fi

"${AWS[@]}" iam attach-role-policy \
  --role-name "$ROLE_NAME" \
  --policy-arn arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore >/dev/null || true

cat > "$TMPDIR/worker-policy.json" <<JSON
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["s3:GetObject", "s3:PutObject"],
      "Resource": "arn:aws:s3:::${BUCKET}/*"
    },
    {
      "Effect": "Allow",
      "Action": ["ssm:GetParameter"],
      "Resource": "*"
    },
    {
      "Effect": "Allow",
      "Action": ["kms:Decrypt"],
      "Resource": "*"
    }
  ]
}
JSON
"${AWS[@]}" iam put-role-policy \
  --role-name "$ROLE_NAME" \
  --policy-name ironsmith-card-fixer-worker \
  --policy-document "file://$TMPDIR/worker-policy.json" >/dev/null

PROFILE_EXISTS=0
"${AWS[@]}" iam get-instance-profile --instance-profile-name "$PROFILE_NAME" >/dev/null 2>&1 || PROFILE_EXISTS=1
if [[ "$PROFILE_EXISTS" -ne 0 ]]; then
  "${AWS[@]}" iam create-instance-profile \
    --instance-profile-name "$PROFILE_NAME" >/dev/null
fi
"${AWS[@]}" iam add-role-to-instance-profile \
  --instance-profile-name "$PROFILE_NAME" \
  --role-name "$ROLE_NAME" >/dev/null 2>&1 || true

VPC_ID="$("${AWS[@]}" ec2 describe-vpcs \
  --filters Name=is-default,Values=true \
  --query 'Vpcs[0].VpcId' \
  --output text)"
mapfile -t SUBNET_IDS < <("${AWS[@]}" ec2 describe-subnets \
  --filters Name=vpc-id,Values="$VPC_ID" Name=default-for-az,Values=true \
  --query 'sort_by(Subnets,&AvailabilityZone)[].SubnetId' \
  --output text | tr '\t' '\n')
if [[ "${#SUBNET_IDS[@]}" -eq 0 ]]; then
  echo "No default subnets found for ${VPC_ID}." >&2
  exit 2
fi

SG_ID="$("${AWS[@]}" ec2 describe-security-groups \
  --filters Name=vpc-id,Values="$VPC_ID" Name=group-name,Values="$SECURITY_GROUP_NAME" \
  --query 'SecurityGroups[0].GroupId' \
  --output text)"
if [[ "$SG_ID" == "None" || -z "$SG_ID" ]]; then
  SG_ID="$("${AWS[@]}" ec2 create-security-group \
    --group-name "$SECURITY_GROUP_NAME" \
    --description "Ironsmith OpenCode card fixer workers - egress only" \
    --vpc-id "$VPC_ID" \
    --query GroupId \
    --output text)"
fi

BAKED_WORKER_AMI=0
AMI_ID="$WORKER_AMI_ID"
if [[ -z "$AMI_ID" && -n "$WORKER_AMI_SSM_PARAM" ]]; then
  AMI_ID="$("${AWS[@]}" ssm get-parameter \
    --name "$WORKER_AMI_SSM_PARAM" \
    --query 'Parameter.Value' \
    --output text 2>/dev/null || true)"
fi
if [[ -n "$AMI_ID" ]]; then
  AMI_STATE="$("${AWS[@]}" ec2 describe-images \
    --image-ids "$AMI_ID" \
    --query 'Images[0].State' \
    --output text 2>/dev/null || true)"
  if [[ "$AMI_STATE" == "available" ]]; then
    BAKED_WORKER_AMI=1
  elif [[ "$REQUIRE_WORKER_AMI" == "1" ]]; then
    echo "Worker AMI ${AMI_ID} is not available (state: ${AMI_STATE:-missing})." >&2
    exit 2
  else
    echo "Worker AMI ${AMI_ID} is not available (state: ${AMI_STATE:-missing}); falling back to Amazon Linux." >&2
    AMI_ID=""
  fi
fi
if [[ -z "$AMI_ID" ]]; then
  if [[ "$REQUIRE_WORKER_AMI" == "1" ]]; then
    echo "No worker AMI found. Set WORKER_AMI_ID, publish $WORKER_AMI_SSM_PARAM, or unset REQUIRE_WORKER_AMI." >&2
    exit 2
  fi
  AMI_ID="$("${AWS[@]}" ssm get-parameter \
    --name "$SOURCE_AMI_SSM_PARAM" \
    --query 'Parameter.Value' \
    --output text)"
else
  BAKED_WORKER_AMI=1
fi
AMI_ARCH="$("${AWS[@]}" ec2 describe-images \
  --image-ids "$AMI_ID" \
  --query 'Images[0].Architecture' \
  --output text 2>/dev/null || true)"
if [[ "$AMI_ARCH" != "$WORKER_ARCH" ]]; then
  echo "Worker AMI ${AMI_ID} has architecture ${AMI_ARCH:-unknown}; expected ${WORKER_ARCH}." >&2
  echo "Set WORKER_ARCH, WORKER_AMI_ID, or WORKER_AMI_SSM_PARAM so AMI and instance types match." >&2
  exit 2
fi
if [[ "$BAKED_WORKER_AMI" == "1" ]]; then
  echo "Using baked worker AMI: ${AMI_ID}"
else
  echo "Using base Amazon Linux AMI: ${AMI_ID}"
fi

USER_DATA_TEMPLATE="$ROOT/scripts/aws_card_fixers/worker_user_data.sh"
"${AWS[@]}" s3 cp "$USER_DATA_TEMPLATE" "${ASSET_PREFIX}/worker_user_data.sh" >/dev/null
LAUNCHED=()
PROFILE_FLAGS="${AWS_PROFILE:+--profile ${AWS_PROFILE}}"

sleep 15

for row in "${SELECTED_ROWS[@]}"; do
  CARD_NAME="${row%%$'\t'*}"
  PARSE_ERROR="${row#*$'\t'}"
  CARD_NAME_B64="$(printf '%s' "$CARD_NAME" | base64 | tr -d '\n')"
  PARSE_ERROR_B64="$(printf '%s' "$PARSE_ERROR" | base64 | tr -d '\n')"
  WORKER_INDEX="${#LAUNCHED[@]}"
  WORKER_ENV="$TMPDIR/worker-env-${WORKER_INDEX}.sh"
  {
    printf 'export CARD_NAME_B64=%q\n' "$CARD_NAME_B64"
    printf 'export PARSE_ERROR_B64=%q\n' "$PARSE_ERROR_B64"
    printf 'export S3_ASSET_PREFIX=%q\n' "$ASSET_PREFIX"
    printf 'export GITHUB_REPO=%q\n' "$GITHUB_REPO"
    printf 'export BASE_BRANCH=%q\n' "$BASE_BRANCH"
    printf 'export WORKER_ARCH=%q\n' "$WORKER_ARCH"
    printf 'export CARDS_S3_URI=%q\n' "$CARDS_S3_URI"
    printf 'export CARDS_S3_COMPRESSION=%q\n' "$CARDS_S3_COMPRESSION"
    printf 'export GITHUB_TOKEN_SSM_PARAM=%q\n' "$GITHUB_TOKEN_SSM_PARAM"
    printf 'export OPENCODE_AUTH_JSON_SSM_PARAM=%q\n' "$OPENCODE_AUTH_JSON_SSM_PARAM"
    printf 'export OPENAI_API_KEY_SSM_PARAM=%q\n' "$OPENAI_API_KEY_SSM_PARAM"
    printf 'export OPENCODE_VERSION=%q\n' "$OPENCODE_VERSION"
    printf 'export OPENCODE_MODEL=%q\n' "$OPENCODE_MODEL"
    printf 'export OPENCODE_VARIANT=%q\n' "$OPENCODE_VARIANT"
    printf 'export OPENCODE_FAST_REASONING_EFFORT=%q\n' "$OPENCODE_FAST_REASONING_EFFORT"
    printf 'export OPENCODE_FAST_TEXT_VERBOSITY=%q\n' "$OPENCODE_FAST_TEXT_VERBOSITY"
    printf 'export OPENCODE_FAST_SERVICE_TIER=%q\n' "$OPENCODE_FAST_SERVICE_TIER"
    printf 'export OPENCODE_STALE_TIMEOUT_SECONDS=%q\n' "$OPENCODE_STALE_TIMEOUT_SECONDS"
    printf 'export OPENCODE_HEARTBEAT_SECONDS=%q\n' "$OPENCODE_HEARTBEAT_SECONDS"
    printf 'export OPENCODE_NO_COMMIT_RETRIES=%q\n' "$OPENCODE_NO_COMMIT_RETRIES"
    printf 'export POST_PR_STEWARD_MAX_REPAIRS=%q\n' "$POST_PR_STEWARD_MAX_REPAIRS"
    printf 'export WORKER_ENTRY_SKILL=%q\n' "$WORKER_ENTRY_SKILL"
    printf 'export BAKED_WORKER_AMI=%q\n' "$BAKED_WORKER_AMI"
    printf 'export SELF_TERMINATE=%q\n' "$SELF_TERMINATE"
  } > "$WORKER_ENV"
  "${AWS[@]}" s3 cp "$WORKER_ENV" "${ASSET_PREFIX}/worker-env/${WORKER_INDEX}.sh" >/dev/null

  USER_DATA="$TMPDIR/user-data-${WORKER_INDEX}.sh"
  {
    printf '#!/usr/bin/env bash\n'
    printf 'set -euo pipefail\n'
    printf 'exec > >(tee -a /var/log/ironsmith-card-fixer-bootstrap.log) 2>&1\n'
    printf 'command -v aws >/dev/null 2>&1 || dnf install -y awscli\n'
    printf 'aws s3 cp %q /tmp/ironsmith-worker-user-data.sh\n' "${ASSET_PREFIX}/worker_user_data.sh"
    printf 'aws s3 cp %q /tmp/ironsmith-worker-env.sh\n' "${ASSET_PREFIX}/worker-env/${WORKER_INDEX}.sh"
    printf 'chmod 600 /tmp/ironsmith-worker-env.sh\n'
    printf 'chmod +x /tmp/ironsmith-worker-user-data.sh\n'
    printf 'set -a\n'
    printf '. /tmp/ironsmith-worker-env.sh\n'
    printf 'set +a\n'
    printf 'exec /tmp/ironsmith-worker-user-data.sh\n'
  } > "$USER_DATA"

  INSTANCE_RESULT="$(run_worker_instance "$USER_DATA")"
  IFS=$'\t' read -r INSTANCE_ID INSTANCE_TYPE_USED INSTANCE_SUBNET_USED <<< "$INSTANCE_RESULT"
  printf '%s\n' "$CARD_NAME" >> "$LAUNCHED_CARDS_FILE"
  LAUNCHED+=("$INSTANCE_ID:$CARD_NAME")
  if [[ "$USE_SPOT" == "1" ]]; then
    echo "Launched ${INSTANCE_ID} (${INSTANCE_TYPE_USED}, ${INSTANCE_SUBNET_USED}, spot) for ${CARD_NAME}"
  else
    echo "Launched ${INSTANCE_ID} (${INSTANCE_TYPE_USED}, ${INSTANCE_SUBNET_USED}, on-demand) for ${CARD_NAME}"
  fi
done

cat <<EOF

Launched ${#LAUNCHED[@]} workers in ${AWS_REGION}.
Session: ${SESSION_ID}
Assets: ${ASSET_PREFIX}

Instances:
$(printf '  %s\n' "${LAUNCHED[@]}")

Watch cloud-init on an instance with:
  aws ${PROFILE_FLAGS} --region ${AWS_REGION} ssm start-session --target INSTANCE_ID
  sudo tail -f /var/log/ironsmith-card-fixer-worker.log

Terminate the session fleet with:
  aws ${PROFILE_FLAGS} --region ${AWS_REGION} ec2 terminate-instances --instance-ids \\
    $("${AWS[@]}" ec2 describe-instances --filters "Name=tag:IronsmithSession,Values=${SESSION_ID}" "Name=instance-state-name,Values=pending,running,stopping,stopped" --query 'Reservations[].Instances[].InstanceId' --output text)
EOF
