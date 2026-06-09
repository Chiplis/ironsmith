#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT/scripts/aws_card_fixers/cost_controls.sh"

AWS_PROFILE="${AWS_PROFILE:-ironsmith-843750990226}"
AWS_REGION="${AWS_REGION:-us-east-2}"
ACCOUNT_ID="${ACCOUNT_ID:-843750990226}"
BUCKET="${BUCKET:-ironsmith-card-fixers-${ACCOUNT_ID}-${AWS_REGION}}"
CONTROLLER_NAME="${CONTROLLER_NAME:-ironsmith-card-fixer-controller}"
CONTROLLER_ARCH="${CONTROLLER_ARCH:-arm64}"
CONTROLLER_INSTANCE_TYPE="${CONTROLLER_INSTANCE_TYPE:-t4g.large}"
CONTROLLER_USE_SPOT="${CONTROLLER_USE_SPOT:-0}"
CONTROLLER_SPOT_MAX_PRICE="${CONTROLLER_SPOT_MAX_PRICE:-}"
CONTROLLER_TTL_HOURS="${CONTROLLER_TTL_HOURS:-120}"
CONTROLLER_SELF_TERMINATE="${CONTROLLER_SELF_TERMINATE:-1}"
CONTROLLER_PROFILE_NAME="${CONTROLLER_PROFILE_NAME:-ironsmith-card-fixer-controller-profile}"
CONTROLLER_SECURITY_GROUP_NAME="${CONTROLLER_SECURITY_GROUP_NAME:-ironsmith-card-fixer-controller}"
CONTROLLER_VOLUME_SIZE_GB="${CONTROLLER_VOLUME_SIZE_GB:-60}"
CONTROLLER_SWAP_GB="${CONTROLLER_SWAP_GB:-16}"
CONTROLLER_CARGO_JOBS="${CONTROLLER_CARGO_JOBS:-1}"
CONTROLLER_REPLACE_UNHEALTHY="${CONTROLLER_REPLACE_UNHEALTHY:-1}"
CONTROLLER_REPLACE_WRONG_TYPE="${CONTROLLER_REPLACE_WRONG_TYPE:-1}"
BASE_BRANCH="${BASE_BRANCH:-main}"
GITHUB_REPO="${GITHUB_REPO:-Chiplis/ironsmith}"
GITHUB_TOKEN_SSM_PARAM="${GITHUB_TOKEN_SSM_PARAM:-/ironsmith/github-token}"
RUN_ID="${RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)-controller-dev-loop}"
SSM_ONLINE_TIMEOUT_SECONDS="${SSM_ONLINE_TIMEOUT_SECONDS:-900}"
COMMAND_TIMEOUT_SECONDS="${COMMAND_TIMEOUT_SECONDS:-3600}"
BOOTSTRAP_PREFIX="${BOOTSTRAP_PREFIX:-s3://${BUCKET}/controller-bootstrap/${RUN_ID}}"

usage() {
  cat <<EOF
Usage:
  scripts/aws_card_fixers/run_dev_loop_on_controller.sh

Launches or reuses an EC2 controller with an instance profile, uploads the local
fleet scripts/status DB/skills, and starts the full dev-loop pipeline detached
on the controller. Local AWS SSO is only needed for this handoff.

Optional env:
  AWS_PROFILE=$AWS_PROFILE
  AWS_REGION=$AWS_REGION
  RUN_ID=$RUN_ID
  CONTROLLER_ARCH=$CONTROLLER_ARCH
  CONTROLLER_INSTANCE_TYPE=$CONTROLLER_INSTANCE_TYPE
  CONTROLLER_USE_SPOT=$CONTROLLER_USE_SPOT
  CONTROLLER_SPOT_MAX_PRICE=$CONTROLLER_SPOT_MAX_PRICE
  CONTROLLER_TTL_HOURS=$CONTROLLER_TTL_HOURS
  CONTROLLER_SELF_TERMINATE=$CONTROLLER_SELF_TERMINATE
  CONTROLLER_VOLUME_SIZE_GB=$CONTROLLER_VOLUME_SIZE_GB
  CONTROLLER_SWAP_GB=$CONTROLLER_SWAP_GB
  CONTROLLER_CARGO_JOBS=$CONTROLLER_CARGO_JOBS
  CONTROLLER_DISK_HYGIENE=${CONTROLLER_DISK_HYGIENE:-1}
  CONTROLLER_MIN_FREE_GB=${CONTROLLER_MIN_FREE_GB:-10}
  CONTROLLER_AGGRESSIVE_CLEAN_FREE_GB=${CONTROLLER_AGGRESSIVE_CLEAN_FREE_GB:-18}
  CONTROLLER_CARGO_CLEAN_AFTER_REFRESH=${CONTROLLER_CARGO_CLEAN_AFTER_REFRESH:-1}
  CONTROLLER_VACUUM_DB_AFTER_SYNC=${CONTROLLER_VACUUM_DB_AFTER_SYNC:-1}
  CONTROLLER_REPLACE_UNHEALTHY=$CONTROLLER_REPLACE_UNHEALTHY
  CONTROLLER_REPLACE_WRONG_TYPE=$CONTROLLER_REPLACE_WRONG_TYPE
  CONTROLLER_PROFILE_NAME=$CONTROLLER_PROFILE_NAME
  CONTROLLER_SECURITY_GROUP_NAME=$CONTROLLER_SECURITY_GROUP_NAME
  BASE_BRANCH=$BASE_BRANCH
  GITHUB_REPO=$GITHUB_REPO
  GITHUB_TOKEN_SSM_PARAM=$GITHUB_TOKEN_SSM_PARAM
  MAX_BATCHES=${MAX_BATCHES:-0}
  DRY_RUN=${DRY_RUN:-0}

After handoff, connect with:
  aws --profile "$AWS_PROFILE" --region "$AWS_REGION" ssm start-session --target INSTANCE_ID
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 2
  fi
}

aws_local() {
  aws --profile "$AWS_PROFILE" --region "$AWS_REGION" "$@"
}

require_command aws
require_command tar

if [[ ! -f "$ROOT/reports/engine-status.sqlite3" ]]; then
  echo "Missing status DB: $ROOT/reports/engine-status.sqlite3" >&2
  exit 2
fi

if [[ ! -d "$ROOT/scripts/aws_card_fixers" ]]; then
  echo "Missing fleet scripts directory: $ROOT/scripts/aws_card_fixers" >&2
  exit 2
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
CONTROLLER_EXPIRES_AT="$(ironsmith_future_utc "$CONTROLLER_TTL_HOURS")"

find_controller_instance() {
  aws_local ec2 describe-instances \
    --filters \
      "Name=tag:Name,Values=${CONTROLLER_NAME}" \
      "Name=tag:Role,Values=ironsmith-card-fixer-controller" \
      "Name=instance-state-name,Values=pending,running,stopping,stopped" \
    --query 'Reservations[].Instances[] | sort_by(@,&LaunchTime)[-1].[InstanceId,State.Name,InstanceType]' \
    --output text 2>/dev/null || true
}

ssm_ping_status_for_instance() {
  local id="$1"
  aws_local ssm describe-instance-information \
    --filters "Key=InstanceIds,Values=${id}" \
    --query 'InstanceInformationList[0].PingStatus' \
    --output text 2>/dev/null || true
}

latest_al2023_ami() {
  case "$CONTROLLER_ARCH" in
    arm64)
      aws_local ssm get-parameter \
        --name /aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64 \
        --query 'Parameter.Value' \
        --output text
      ;;
    x86_64)
      aws_local ssm get-parameter \
        --name /aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64 \
        --query 'Parameter.Value' \
        --output text
      ;;
    *)
      echo "CONTROLLER_ARCH must be x86_64 or arm64." >&2
      exit 2
      ;;
  esac
}

default_subnet_id() {
  aws_local ec2 describe-subnets \
    --filters Name=default-for-az,Values=true \
    --query 'sort_by(Subnets,&AvailabilityZone)[0].SubnetId' \
    --output text
}

security_group_id() {
  aws_local ec2 describe-security-groups \
    --filters "Name=group-name,Values=${CONTROLLER_SECURITY_GROUP_NAME}" \
    --query 'SecurityGroups[0].GroupId' \
    --output text
}

controller_info="$(find_controller_instance)"
instance_id="$(awk '{print $1}' <<<"$controller_info")"
instance_state="$(awk '{print $2}' <<<"$controller_info")"
instance_type="$(awk '{print $3}' <<<"$controller_info")"

if [[ -n "${instance_id:-}" && "$instance_id" != "None" && "$instance_state" =~ ^(pending|running)$ ]]; then
  replace_reason=""
  if [[ "$CONTROLLER_REPLACE_WRONG_TYPE" == "1" && -n "${instance_type:-}" && "$instance_type" != "$CONTROLLER_INSTANCE_TYPE" ]]; then
    replace_reason="instance type is ${instance_type}, desired ${CONTROLLER_INSTANCE_TYPE}"
  elif [[ "$CONTROLLER_REPLACE_UNHEALTHY" == "1" && "$instance_state" == "running" ]]; then
    ping_status="$(ssm_ping_status_for_instance "$instance_id")"
    if [[ "$ping_status" != "Online" ]]; then
      replace_reason="SSM ping status is ${ping_status:-missing}"
    fi
  fi

  if [[ -n "$replace_reason" ]]; then
    echo "Replacing controller ${instance_id}: ${replace_reason}."
    aws_local ec2 terminate-instances --instance-ids "$instance_id" >/dev/null
    aws_local ec2 wait instance-terminated --instance-ids "$instance_id"
    instance_id=""
    instance_state=""
    instance_type=""
  fi
fi

if [[ -z "${instance_id:-}" || "$instance_id" == "None" ]]; then
  ami_id="$(latest_al2023_ami)"
  subnet_id="$(default_subnet_id)"
  sg_id="$(security_group_id)"
  echo "Launching controller ${CONTROLLER_NAME} (${CONTROLLER_INSTANCE_TYPE})..."
  controller_market_options=()
  if [[ "$CONTROLLER_USE_SPOT" == "1" ]]; then
    spot_options="SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate"
    if [[ -n "$CONTROLLER_SPOT_MAX_PRICE" ]]; then
      spot_options="${spot_options},MaxPrice=${CONTROLLER_SPOT_MAX_PRICE}"
    fi
    controller_market_options=(--instance-market-options "MarketType=spot,SpotOptions={${spot_options}}")
  fi
  instance_id="$(
    aws_local ec2 run-instances \
      --image-id "$ami_id" \
      --instance-type "$CONTROLLER_INSTANCE_TYPE" \
      ${controller_market_options[@]+"${controller_market_options[@]}"} \
      --iam-instance-profile "Name=${CONTROLLER_PROFILE_NAME}" \
      --subnet-id "$subnet_id" \
      --security-group-ids "$sg_id" \
      --instance-initiated-shutdown-behavior terminate \
      --block-device-mappings "DeviceName=/dev/xvda,Ebs={VolumeSize=${CONTROLLER_VOLUME_SIZE_GB},VolumeType=gp3,DeleteOnTermination=true}" \
      --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=${CONTROLLER_NAME}},{Key=Role,Value=ironsmith-card-fixer-controller},{Key=Project,Value=ironsmith-card-fixer},{Key=RunId,Value=${RUN_ID}},{Key=IronsmithExpiresAt,Value=${CONTROLLER_EXPIRES_AT}}]" \
        "ResourceType=volume,Tags=[{Key=Name,Value=${CONTROLLER_NAME}},{Key=Role,Value=ironsmith-card-fixer-controller},{Key=Project,Value=ironsmith-card-fixer},{Key=RunId,Value=${RUN_ID}},{Key=IronsmithExpiresAt,Value=${CONTROLLER_EXPIRES_AT}}]" \
      --query 'Instances[0].InstanceId' \
      --output text
  )"
elif [[ "$instance_state" == "stopped" ]]; then
  echo "Starting stopped controller ${instance_id}..."
  aws_local ec2 start-instances --instance-ids "$instance_id" >/dev/null
elif [[ "$instance_state" == "stopping" ]]; then
  echo "Controller ${instance_id} is stopping; wait for it to stop or terminate it before retrying." >&2
  exit 3
else
  echo "Reusing controller ${instance_id} (${instance_state})."
fi

echo "Waiting for controller ${instance_id} to run..."
aws_local ec2 wait instance-running --instance-ids "$instance_id"
aws_local ec2 create-tags \
  --resources "$instance_id" \
  --tags "Key=RunId,Value=${RUN_ID}" "Key=IronsmithExpiresAt,Value=${CONTROLLER_EXPIRES_AT}" >/dev/null
controller_volume_ids=()
while IFS= read -r volume_id; do
  [[ -n "$volume_id" && "$volume_id" != "None" ]] || continue
  controller_volume_ids+=("$volume_id")
done < <(aws_local ec2 describe-instances \
  --instance-ids "$instance_id" \
  --query 'Reservations[].Instances[].BlockDeviceMappings[].Ebs.VolumeId' \
  --output text | tr '\t' '\n')
if [[ "${#controller_volume_ids[@]}" -gt 0 ]]; then
  aws_local ec2 create-tags \
    --resources "${controller_volume_ids[@]}" \
    --tags "Key=Name,Value=${CONTROLLER_NAME}" \
           "Key=Role,Value=ironsmith-card-fixer-controller" \
           "Key=Project,Value=ironsmith-card-fixer" \
           "Key=RunId,Value=${RUN_ID}" \
           "Key=IronsmithExpiresAt,Value=${CONTROLLER_EXPIRES_AT}" >/dev/null
fi

deadline=$(( $(date +%s) + SSM_ONLINE_TIMEOUT_SECONDS ))
while true; do
  ping_status="$(
    aws_local ssm describe-instance-information \
      --filters "Key=InstanceIds,Values=${instance_id}" \
      --query 'InstanceInformationList[0].PingStatus' \
      --output text 2>/dev/null || true
  )"
  if [[ "$ping_status" == "Online" ]]; then
    break
  fi
  if (( $(date +%s) >= deadline )); then
    echo "Timed out waiting for ${instance_id} to appear in SSM." >&2
    exit 4
  fi
  sleep 10
done

overlay_tar="$tmpdir/controller-overlay.tar.gz"
skills_tar="$tmpdir/ironsmith-skills.tar.gz"
bootstrap_script="$tmpdir/controller-bootstrap.sh"
env_file="$tmpdir/controller-env.sh"

tar -C "$ROOT" -czf "$overlay_tar" \
  scripts/aws_card_fixers \
  cards.json \
  reports/engine-status.sqlite3

skill_args=()
if [[ -d "$HOME/.codex/skills" ]]; then
  while IFS= read -r skill_dir; do
    skill_args+=("$(basename "$skill_dir")")
  done < <(find "$HOME/.codex/skills" -maxdepth 1 -type d -name 'ironsmith-*' | sort)
fi
if [[ "${#skill_args[@]}" -gt 0 ]]; then
  tar -C "$HOME/.codex/skills" -czf "$skills_tar" "${skill_args[@]}"
else
  tar -czf "$skills_tar" --files-from /dev/null
fi

cat > "$env_file" <<EOF
export AWS_REGION=$(printf '%q' "$AWS_REGION")
export BASE_BRANCH=$(printf '%q' "$BASE_BRANCH")
export GITHUB_REPO=$(printf '%q' "$GITHUB_REPO")
export GITHUB_TOKEN_SSM_PARAM=$(printf '%q' "$GITHUB_TOKEN_SSM_PARAM")
export OPENAI_API_KEY_SSM_PARAM=$(printf '%q' "${OPENAI_API_KEY_SSM_PARAM:-}")
EOF
if [[ -n "${OPENAI_API_KEY_SSM_PARAM:-}" ]]; then
  printf 'export OPENCODE_AUTH_JSON_SSM_PARAM=%q\n' "${OPENCODE_AUTH_JSON_SSM_PARAM:-}" >> "$env_file"
else
  printf 'export OPENCODE_AUTH_JSON_SSM_PARAM=%q\n' "${OPENCODE_AUTH_JSON_SSM_PARAM:-/ironsmith/opencode-auth-json}" >> "$env_file"
fi
cat >> "$env_file" <<EOF
export OPENCODE_VERSION=$(printf '%q' "${OPENCODE_VERSION:-1.15.9}")
export RUN_ID=$(printf '%q' "$RUN_ID")
export RUN_DIR=$(printf '%q' "/opt/ironsmith/reports/aws-card-fixer-dev-loop/${RUN_ID}")
export LOG_FILE=$(printf '%q' "/opt/ironsmith/reports/aws-card-fixer-dev-loop/${RUN_ID}/dev-loop-${RUN_ID}.log")
export CONTROLLER_LOG_S3_PREFIX=$(printf '%q' "s3://${BUCKET}/controller-runs/${RUN_ID}")
export IRONSMITH_CONTROLLER=1
export CONTROLLER_SWAP_GB=$(printf '%q' "$CONTROLLER_SWAP_GB")
export CONTROLLER_CARGO_JOBS=$(printf '%q' "$CONTROLLER_CARGO_JOBS")
export CONTROLLER_SELF_TERMINATE=$(printf '%q' "$CONTROLLER_SELF_TERMINATE")
export CARGO_BUILD_JOBS=$(printf '%q' "$CONTROLLER_CARGO_JOBS")
export CARGO_INCREMENTAL=$(printf '%q' "${CARGO_INCREMENTAL:-0}")
export CONTROLLER_DISK_HYGIENE=$(printf '%q' "${CONTROLLER_DISK_HYGIENE:-1}")
export CONTROLLER_MIN_FREE_GB=$(printf '%q' "${CONTROLLER_MIN_FREE_GB:-10}")
export CONTROLLER_AGGRESSIVE_CLEAN_FREE_GB=$(printf '%q' "${CONTROLLER_AGGRESSIVE_CLEAN_FREE_GB:-18}")
export CONTROLLER_CARGO_CLEAN_AFTER_REFRESH=$(printf '%q' "${CONTROLLER_CARGO_CLEAN_AFTER_REFRESH:-1}")
export CONTROLLER_VACUUM_DB_AFTER_SYNC=$(printf '%q' "${CONTROLLER_VACUUM_DB_AFTER_SYNC:-1}")
export CONTROLLER_DISK_LOG_DU=$(printf '%q' "${CONTROLLER_DISK_LOG_DU:-1}")
export BATCH_SIZE=$(printf '%q' "${BATCH_SIZE:-8}")
export WORKER_ARCH=$(printf '%q' "${WORKER_ARCH:-arm64}")
export INSTANCE_TYPE=$(printf '%q' "${INSTANCE_TYPE:-t4g.medium}")
export INSTANCE_TYPES=$(printf '%q' "${INSTANCE_TYPES:-t4g.medium t4g.large c8g.large c7g.large c6g.large m7g.large m6g.large r7g.large r6g.large}")
export USE_SPOT=$(printf '%q' "${USE_SPOT:-1}")
export USE_EC2_FLEET=$(printf '%q' "${USE_EC2_FLEET:-1}")
export SPOT_MAX_PRICE=$(printf '%q' "${SPOT_MAX_PRICE:-}")
export BURSTABLE_CPU_CREDITS=$(printf '%q' "${BURSTABLE_CPU_CREDITS:-standard}")
export MAX_BATCHES=$(printf '%q' "${MAX_BATCHES:-0}")
export MAX_TOTAL_CARDS=$(printf '%q' "${MAX_TOTAL_CARDS:-0}")
export POLL_INTERVAL=$(printf '%q' "${POLL_INTERVAL:-60}")
export BATCH_TIMEOUT_SECONDS=$(printf '%q' "${BATCH_TIMEOUT_SECONDS:-28800}")
export STOP_ON_FAILED=$(printf '%q' "${STOP_ON_FAILED:-0}")
export DRY_RUN=$(printf '%q' "${DRY_RUN:-0}")
export MERGE_AFTER_BATCH=$(printf '%q' "${MERGE_AFTER_BATCH:-1}")
export POST_BATCH_PIPELINE=$(printf '%q' "${POST_BATCH_PIPELINE:-1}")
export REVIEW_PRS_AS_READY=$(printf '%q' "${REVIEW_PRS_AS_READY:-1}")
export REVIEW_OPEN_PRS_ON_START=$(printf '%q' "${REVIEW_OPEN_PRS_ON_START:-1}")
export REFRESH_AFTER_MERGE=$(printf '%q' "${REFRESH_AFTER_MERGE:-1}")
export BAKE_EVERY_MERGED_PRS=$(printf '%q' "${BAKE_EVERY_MERGED_PRS:-1}")
export BAKE_ON_STOP=$(printf '%q' "${BAKE_ON_STOP:-0}")
export REFRESH_AFTER_BATCH_MERGE=$(printf '%q' "${REFRESH_AFTER_BATCH_MERGE:-1}")
export BAKE_AFTER_BATCH_MERGE=$(printf '%q' "${BAKE_AFTER_BATCH_MERGE:-1}")
export CODEX_HANDLE_REMAINING_PRS=$(printf '%q' "${CODEX_HANDLE_REMAINING_PRS:-1}")
export STEWARD_HANDLE_REMAINING_PRS=$(printf '%q' "${STEWARD_HANDLE_REMAINING_PRS:-1}")
export STEWARD_COMMAND=$(printf '%q' "${STEWARD_COMMAND:-opencode}")
export STEWARD_MODEL=$(printf '%q' "${STEWARD_MODEL:-${OPENCODE_MODEL:-openai/gpt-5.5-fast}}")
export STEWARD_VARIANT=$(printf '%q' "${STEWARD_VARIANT:-${OPENCODE_VARIANT:-fast}}")
export SAFE_MERGE_VERIFY_COMMAND=$(printf '%q' "${SAFE_MERGE_VERIFY_COMMAND:-cargo check --workspace -j ${CONTROLLER_CARGO_JOBS}}")
export PARALLEL_SYNC_AND_BAKE=$(printf '%q' "${PARALLEL_SYNC_AND_BAKE:-0}")
export OPENCODE_MODEL=$(printf '%q' "${OPENCODE_MODEL:-openai/gpt-5.5-fast}")
export OPENCODE_VARIANT=$(printf '%q' "${OPENCODE_VARIANT:-fast}")
export OPENCODE_FAST_REASONING_EFFORT=$(printf '%q' "${OPENCODE_FAST_REASONING_EFFORT:-high}")
export OPENCODE_FAST_TEXT_VERBOSITY=$(printf '%q' "${OPENCODE_FAST_TEXT_VERBOSITY:-low}")
export OPENCODE_FAST_SERVICE_TIER=$(printf '%q' "${OPENCODE_FAST_SERVICE_TIER:-priority}")
export OPENCODE_STALE_TIMEOUT_SECONDS=$(printf '%q' "${OPENCODE_STALE_TIMEOUT_SECONDS:-1800}")
export OPENCODE_HEARTBEAT_SECONDS=$(printf '%q' "${OPENCODE_HEARTBEAT_SECONDS:-60}")
export OPENCODE_NO_COMMIT_RETRIES=$(printf '%q' "${OPENCODE_NO_COMMIT_RETRIES:-1}")
export POST_PR_STEWARD_MAX_REPAIRS=$(printf '%q' "${POST_PR_STEWARD_MAX_REPAIRS:-3}")
export WORKER_ENTRY_SKILL=$(printf '%q' "${WORKER_ENTRY_SKILL:-ironsmith-aws-card-fixer-fleet}")
export WORKER_AMI_SSM_PARAM=$(printf '%q' "${WORKER_AMI_SSM_PARAM:-}")
export SOURCE_AMI_SSM_PARAM=$(printf '%q' "${SOURCE_AMI_SSM_PARAM:-}")
export REQUIRE_WORKER_AMI=$(printf '%q' "${REQUIRE_WORKER_AMI:-1}")
export AMI_BUILD_RELEASE_TOOLS=$(printf '%q' "${AMI_BUILD_RELEASE_TOOLS:-1}")
export AMI_CARGO_CLEAN_BEFORE_BUILD=$(printf '%q' "${AMI_CARGO_CLEAN_BEFORE_BUILD:-1}")
export DEREGISTER_OLD_AMI=$(printf '%q' "${DEREGISTER_OLD_AMI:-1}")
export CLEANUP_OLD_WORKER_AMIS=$(printf '%q' "${CLEANUP_OLD_WORKER_AMIS:-1}")
export RETAIN_WORKER_AMIS_PER_ARCH=$(printf '%q' "${RETAIN_WORKER_AMIS_PER_ARCH:-1}")
export INSTANCE_TTL_HOURS=$(printf '%q' "${INSTANCE_TTL_HOURS:-6}")
export SELF_TERMINATE=$(printf '%q' "${SELF_TERMINATE:-1}")
export S3_SESSION_PREFIX=$(printf '%q' "${S3_SESSION_PREFIX:-sessions}")
export USE_INSTANCE_PROFILE=1
EOF

cat > "$bootstrap_script" <<'REMOTE'
#!/usr/bin/env bash
set -euo pipefail

aws_region="${AWS_REGION:-us-east-2}"
asset_prefix="${1:?missing bootstrap asset prefix}"
workdir="/opt/ironsmith"
log_dir="/var/log/ironsmith-controller"
mkdir -p "$log_dir" /root/.codex/skills

dnf install -y git jq tmux python3 sqlite tar gzip rust cargo nodejs npm >/var/log/ironsmith-controller/dnf-install.log 2>&1

swap_gb="${CONTROLLER_SWAP_GB:-16}"
if [[ "$swap_gb" =~ ^[0-9]+$ && "$swap_gb" -gt 0 ]]; then
  if [[ ! -f /swapfile ]]; then
    fallocate -l "${swap_gb}G" /swapfile || dd if=/dev/zero of=/swapfile bs=1M count=$((swap_gb * 1024))
    chmod 600 /swapfile
    mkswap /swapfile
  fi
  if ! grep -q '^/swapfile ' /etc/fstab; then
    printf '/swapfile none swap sw 0 0\n' >> /etc/fstab
  fi
  swapon /swapfile 2>/dev/null || true
  sysctl vm.swappiness=20 >/dev/null 2>&1 || true
fi

if ! command -v gh >/dev/null 2>&1; then
  dnf install -y 'dnf-command(config-manager)' >/dev/null 2>&1 || true
  dnf config-manager --add-repo https://cli.github.com/packages/rpm/gh-cli.repo >/dev/null 2>&1 || true
  dnf install -y gh >/var/log/ironsmith-controller/gh-install.log 2>&1 || true
fi

if ! command -v opencode >/dev/null 2>&1; then
  npm install -g "opencode-ai@${OPENCODE_VERSION:-1.15.9}" >/var/log/ironsmith-controller/opencode-install.log 2>&1 || true
fi

aws s3 cp "${asset_prefix}/controller-overlay.tar.gz" /tmp/controller-overlay.tar.gz
aws s3 cp "${asset_prefix}/ironsmith-skills.tar.gz" /tmp/ironsmith-skills.tar.gz
aws s3 cp "${asset_prefix}/controller-env.sh" /tmp/controller-env.sh
source /tmp/controller-env.sh
unset AWS_PROFILE AWS_DEFAULT_PROFILE

if [[ -n "${OPENCODE_VARIANT:-}" ]]; then
  python3 - "$OPENCODE_MODEL" "$OPENCODE_VARIANT" "$OPENCODE_FAST_REASONING_EFFORT" "$OPENCODE_FAST_TEXT_VERBOSITY" "$OPENCODE_FAST_SERVICE_TIER" <<'PY'
import json
import os
import pathlib
import sys

model, variant, reasoning_effort, text_verbosity, service_tier = sys.argv[1:]
if "/" not in model:
    raise SystemExit(0)
provider, model_id = model.split("/", 1)
config_dir = pathlib.Path(os.environ.get("OPENCODE_CONFIG_DIR", pathlib.Path.home() / ".config" / "opencode"))
config_dir.mkdir(parents=True, exist_ok=True)
config_path = config_dir / "opencode.json"

config = {}
if config_path.exists():
    try:
        config = json.loads(config_path.read_text(encoding="utf-8"))
    except Exception:
        backup = config_path.with_suffix(".json.invalid")
        config_path.replace(backup)
        config = {}

config.setdefault("$schema", "https://opencode.ai/config.json")
provider_config = config.setdefault("provider", {}).setdefault(provider, {})
model_config = provider_config.setdefault("models", {}).setdefault(model_id, {})
variant_config = model_config.setdefault("variants", {}).setdefault(variant, {})

if reasoning_effort:
    variant_config["reasoningEffort"] = reasoning_effort
if text_verbosity:
    variant_config["textVerbosity"] = text_verbosity
if service_tier:
    variant_config["serviceTier"] = service_tier

config_path.write_text(json.dumps(config, indent=2, sort_keys=True) + "\n", encoding="utf-8")
print(f"Configured OpenCode variant {provider}/{model_id}#{variant} in {config_path}")
PY
fi

github_token="$(aws --region "$aws_region" ssm get-parameter --name "$GITHUB_TOKEN_SSM_PARAM" --with-decryption --query 'Parameter.Value' --output text)"
if command -v gh >/dev/null 2>&1; then
  printf '%s' "$github_token" | gh auth login --with-token >/dev/null 2>&1 || true
  gh auth setup-git >/dev/null 2>&1 || true
fi

if [[ ! -d "$workdir/.git" ]]; then
  rm -rf "$workdir"
  git clone "https://x-access-token:${github_token}@github.com/${GITHUB_REPO}.git" "$workdir"
fi
unset github_token

cd "$workdir"
git remote set-url origin "https://github.com/${GITHUB_REPO}.git"
git fetch origin "$BASE_BRANCH"
git checkout "$BASE_BRANCH"
git reset --hard "origin/${BASE_BRANCH}"

tar -C "$workdir" -xzf /tmp/controller-overlay.tar.gz
tar -C /root/.codex/skills -xzf /tmp/ironsmith-skills.tar.gz
chmod +x "$workdir"/scripts/aws_card_fixers/*.sh
"$workdir/scripts/aws_card_fixers/controller_disk_hygiene.sh" startup

run_dir="$workdir/reports/aws-card-fixer-dev-loop/${RUN_ID}"
mkdir -p "$run_dir"
cat >/tmp/ironsmith-controller-run.sh <<'RUN'
#!/usr/bin/env bash
set -euo pipefail
source /tmp/controller-env.sh
export HOME=/root
unset AWS_PROFILE AWS_DEFAULT_PROFILE
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-${CONTROLLER_CARGO_JOBS:-2}}"
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
if [[ -n "${GITHUB_TOKEN_SSM_PARAM:-}" ]]; then
  export GH_TOKEN="$(aws --region "${AWS_REGION:-us-east-2}" ssm get-parameter --name "$GITHUB_TOKEN_SSM_PARAM" --with-decryption --query 'Parameter.Value' --output text)"
fi
if [[ -n "${OPENCODE_AUTH_JSON_SSM_PARAM:-}" ]]; then
  mkdir -p /root/.local/share/opencode
  aws --region "${AWS_REGION:-us-east-2}" ssm get-parameter \
    --name "$OPENCODE_AUTH_JSON_SSM_PARAM" \
    --with-decryption \
    --query 'Parameter.Value' \
    --output text > /root/.local/share/opencode/auth.json
  chmod 600 /root/.local/share/opencode/auth.json
fi
if [[ -n "${OPENAI_API_KEY_SSM_PARAM:-}" ]]; then
  export OPENAI_API_KEY="$(aws --region "${AWS_REGION:-us-east-2}" ssm get-parameter --name "$OPENAI_API_KEY_SSM_PARAM" --with-decryption --query 'Parameter.Value' --output text)"
fi
cd /opt/ironsmith
mkdir -p "$(dirname "$LOG_FILE")"
scripts/aws_card_fixers/run_dev_loop_full_pipeline.sh &
loop_pid="$!"

if [[ -n "${CONTROLLER_LOG_S3_PREFIX:-}" ]]; then
  (
    while kill -0 "$loop_pid" 2>/dev/null; do
      if [[ -f "$LOG_FILE" ]]; then
        aws --region "${AWS_REGION:-us-east-2}" s3 cp "$LOG_FILE" "${CONTROLLER_LOG_S3_PREFIX}/$(basename "$LOG_FILE")" >/dev/null 2>&1 || true
      fi
      sleep 60
    done
  ) &
  sync_pid="$!"
else
  sync_pid=""
fi

status=0
wait "$loop_pid" || status="$?"
if [[ -n "${sync_pid:-}" ]]; then
  wait "$sync_pid" 2>/dev/null || true
fi
if [[ -n "${CONTROLLER_LOG_S3_PREFIX:-}" && -f "$LOG_FILE" ]]; then
  aws --region "${AWS_REGION:-us-east-2}" s3 cp "$LOG_FILE" "${CONTROLLER_LOG_S3_PREFIX}/$(basename "$LOG_FILE")" >/dev/null 2>&1 || true
fi
if [[ "${CONTROLLER_SELF_TERMINATE:-1}" == "1" ]]; then
  sync || true
  shutdown -h now || true
fi
exit "$status"
RUN
chmod +x /tmp/ironsmith-controller-run.sh

session_name="ironsmith-${RUN_ID//[^A-Za-z0-9_-]/-}"
if tmux has-session -t "$session_name" 2>/dev/null; then
  echo "tmux session already exists: $session_name"
else
  tmux new-session -d -s "$session_name" "/tmp/ironsmith-controller-run.sh 2>&1 | tee -a '${run_dir}/controller-${RUN_ID}.log'"
fi

cat <<EOF
Controller handoff complete.
Workdir: $workdir
tmux session: $session_name
Controller log: ${run_dir}/controller-${RUN_ID}.log
Unified log: ${LOG_FILE}
Unified log S3 copy: ${CONTROLLER_LOG_S3_PREFIX}/$(basename "$LOG_FILE")
Attach: tmux attach -t $session_name
EOF
REMOTE

echo "Uploading controller bootstrap assets to ${BOOTSTRAP_PREFIX}..."
ironsmith_ensure_bucket_lifecycle "$BUCKET" "$tmpdir"
aws_local s3 cp "$overlay_tar" "${BOOTSTRAP_PREFIX}/controller-overlay.tar.gz" >/dev/null
aws_local s3 cp "$skills_tar" "${BOOTSTRAP_PREFIX}/ironsmith-skills.tar.gz" >/dev/null
aws_local s3 cp "$env_file" "${BOOTSTRAP_PREFIX}/controller-env.sh" >/dev/null
aws_local s3 cp "$bootstrap_script" "${BOOTSTRAP_PREFIX}/controller-bootstrap.sh" >/dev/null

ssm_parameters="$tmpdir/ssm-parameters.json"
python3 - "$ssm_parameters" "$BOOTSTRAP_PREFIX" "$AWS_REGION" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
asset_prefix = sys.argv[2]
aws_region = sys.argv[3]
path.write_text(
    json.dumps(
        {
            "commands": [
                f"aws s3 cp {asset_prefix}/controller-bootstrap.sh /tmp/controller-bootstrap.sh",
                "chmod +x /tmp/controller-bootstrap.sh",
                f"AWS_REGION={aws_region} /tmp/controller-bootstrap.sh {asset_prefix}",
            ]
        }
    ),
    encoding="utf-8",
)
PY

echo "Starting detached dev loop on controller ${instance_id}..."
command_id="$(
  aws_local ssm send-command \
    --instance-ids "$instance_id" \
    --document-name AWS-RunShellScript \
    --comment "Start Ironsmith dev loop ${RUN_ID}" \
    --parameters "file://${ssm_parameters}" \
    --timeout-seconds "$COMMAND_TIMEOUT_SECONDS" \
    --query 'Command.CommandId' \
    --output text
)"

echo "SSM command: ${command_id}"
echo "Waiting for bootstrap command to finish..."
bootstrap_deadline=$(( $(date +%s) + COMMAND_TIMEOUT_SECONDS ))
while true; do
  command_status="$(
    aws_local ssm get-command-invocation \
      --command-id "$command_id" \
      --instance-id "$instance_id" \
      --query 'Status' \
      --output text 2>/dev/null || true
  )"
  case "$command_status" in
    Success|Cancelled|TimedOut|Failed|Cancelling)
      break
      ;;
  esac
  if (( $(date +%s) >= bootstrap_deadline )); then
    echo "Bootstrap command is still ${command_status:-unknown} after ${COMMAND_TIMEOUT_SECONDS}s; continuing to print current handoff details." >&2
    break
  fi
  sleep 15
done
aws_local ssm get-command-invocation \
  --command-id "$command_id" \
  --instance-id "$instance_id" \
  --query '{Status:Status,ResponseCode:ResponseCode,Stdout:StandardOutputContent,Stderr:StandardErrorContent}' \
  --output json

cat <<EOF

Controller instance: $instance_id
Bootstrap assets:     $BOOTSTRAP_PREFIX
Run ID:               $RUN_ID
Unified log on host:  /opt/ironsmith/reports/aws-card-fixer-dev-loop/${RUN_ID}/dev-loop-${RUN_ID}.log
Unified log in S3:    s3://${BUCKET}/controller-runs/${RUN_ID}/dev-loop-${RUN_ID}.log

The dev loop is detached on the controller. Local SSO may expire now without
stopping it.
EOF
