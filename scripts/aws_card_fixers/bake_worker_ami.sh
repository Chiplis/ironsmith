#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$ROOT/scripts/aws_card_fixers/cost_controls.sh"
AWS_REGION="${AWS_REGION:-$(aws configure get region 2>/dev/null || true)}"
AWS_REGION="${AWS_REGION:-us-east-2}"
GITHUB_REPO="${GITHUB_REPO:-Chiplis/ironsmith}"
BASE_BRANCH="${BASE_BRANCH:-main}"
CARDS_PATH="${CARDS_PATH:-$ROOT/cards.json}"
OPENCODE_VERSION="${OPENCODE_VERSION:-1.15.9}"
WORKER_ARCH="${WORKER_ARCH:-arm64}"
AMI_BUILDER_INSTANCE_TYPE="${AMI_BUILDER_INSTANCE_TYPE:-}"
if [[ "$WORKER_ARCH" != "x86_64" && "$WORKER_ARCH" != "arm64" ]]; then
  echo "WORKER_ARCH must be x86_64 or arm64." >&2
  exit 2
fi
if [[ -z "${AMI_BUILDER_INSTANCE_TYPES+x}" ]]; then
  if [[ -n "$AMI_BUILDER_INSTANCE_TYPE" ]]; then
    AMI_BUILDER_INSTANCE_TYPES="$AMI_BUILDER_INSTANCE_TYPE"
  elif [[ "$WORKER_ARCH" == "arm64" ]]; then
    AMI_BUILDER_INSTANCE_TYPES="t4g.large c7g.xlarge c6g.xlarge"
  else
    AMI_BUILDER_INSTANCE_TYPES="c7a.large c7i.large c6a.large c6i.large"
  fi
fi
AMI_VOLUME_SIZE="${AMI_VOLUME_SIZE:-60}"
AMI_BUILDER_USE_SPOT="${AMI_BUILDER_USE_SPOT:-1}"
AMI_BUILDER_SPOT_MAX_PRICE="${AMI_BUILDER_SPOT_MAX_PRICE:-}"
AMI_BUILDER_TTL_HOURS="${AMI_BUILDER_TTL_HOURS:-4}"
BUCKET="${BUCKET:-}"
if [[ "$WORKER_ARCH" == "arm64" ]]; then
  DEFAULT_AMI_NAME_PREFIX="ironsmith-card-fixer-worker-arm64"
  DEFAULT_WORKER_AMI_SSM_PARAM="/ironsmith/card-fixer-worker-ami-arm64"
  DEFAULT_WORKER_AMI_METADATA_SSM_PREFIX="/ironsmith/card-fixer-worker-ami-arm64-metadata"
  DEFAULT_SOURCE_AMI_SSM_PARAM="/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64"
else
  DEFAULT_AMI_NAME_PREFIX="ironsmith-card-fixer-worker"
  DEFAULT_WORKER_AMI_SSM_PARAM="/ironsmith/card-fixer-worker-ami"
  DEFAULT_WORKER_AMI_METADATA_SSM_PREFIX="/ironsmith/card-fixer-worker-ami-metadata"
  DEFAULT_SOURCE_AMI_SSM_PARAM="/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64"
fi
AMI_NAME_PREFIX="${AMI_NAME_PREFIX:-$DEFAULT_AMI_NAME_PREFIX}"
WORKER_AMI_SSM_PARAM="${WORKER_AMI_SSM_PARAM:-$DEFAULT_WORKER_AMI_SSM_PARAM}"
WORKER_AMI_METADATA_SSM_PREFIX="${WORKER_AMI_METADATA_SSM_PREFIX:-$DEFAULT_WORKER_AMI_METADATA_SSM_PREFIX}"
SOURCE_AMI_SSM_PARAM="${SOURCE_AMI_SSM_PARAM:-$DEFAULT_SOURCE_AMI_SSM_PARAM}"
SOURCE_AMI_ID="${SOURCE_AMI_ID:-}"
AMI_BUILD_RELEASE_TOOLS="${AMI_BUILD_RELEASE_TOOLS:-1}"
AMI_CARGO_CLEAN_BEFORE_BUILD="${AMI_CARGO_CLEAN_BEFORE_BUILD:-1}"
ROLE_NAME="${ROLE_NAME:-ironsmith-card-fixer-worker-role}"
PROFILE_NAME="${PROFILE_NAME:-ironsmith-card-fixer-worker-profile}"
SECURITY_GROUP_NAME="${SECURITY_GROUP_NAME:-ironsmith-card-fixer-workers}"
AMI_BAKE_TIMEOUT_SECONDS="${AMI_BAKE_TIMEOUT_SECONDS:-7200}"
SSM_ONLINE_TIMEOUT_SECONDS="${SSM_ONLINE_TIMEOUT_SECONDS:-900}"
KEEP_BUILDER_ON_FAILURE="${KEEP_BUILDER_ON_FAILURE:-0}"
DEREGISTER_OLD_AMI="${DEREGISTER_OLD_AMI:-1}"
CLEANUP_OLD_WORKER_AMIS="${CLEANUP_OLD_WORKER_AMIS:-1}"
RETAIN_WORKER_AMIS_PER_ARCH="${RETAIN_WORKER_AMIS_PER_ARCH:-1}"
FORCE_BASE_AMI_IF_SOURCE_VOLUME_GT="${FORCE_BASE_AMI_IF_SOURCE_VOLUME_GT:-1}"

usage() {
  cat <<EOF
Usage:
  AWS_PROFILE=ironsmith-843750990226 \\
  AWS_REGION=us-east-2 \\
  scripts/aws_card_fixers/bake_worker_ami.sh

Optional env:
  GITHUB_REPO=Chiplis/ironsmith
  BASE_BRANCH=main
  CARDS_PATH=cards.json
  OPENCODE_VERSION=1.15.9
  WORKER_ARCH=arm64
  AMI_BUILDER_INSTANCE_TYPES="t4g.large c7g.xlarge c6g.xlarge"
  AMI_BUILDER_INSTANCE_TYPE=t4g.large
  AMI_BUILDER_USE_SPOT=1
  AMI_BUILDER_SPOT_MAX_PRICE=
  AMI_BUILDER_TTL_HOURS=4
  AMI_BUILDER_SUBNET_IDS="subnet-a subnet-b"
  AMI_VOLUME_SIZE=60
  BUCKET=existing-or-new-bucket-name
  AMI_NAME_PREFIX=ironsmith-card-fixer-worker-arm64
  WORKER_AMI_SSM_PARAM=/ironsmith/card-fixer-worker-ami-arm64
  SOURCE_AMI_SSM_PARAM=/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64
  SOURCE_AMI_ID=ami-...
  AMI_BUILD_RELEASE_TOOLS=1
  AMI_CARGO_CLEAN_BEFORE_BUILD=1
  AMI_BAKE_TIMEOUT_SECONDS=7200
  SSM_ONLINE_TIMEOUT_SECONDS=900
  KEEP_BUILDER_ON_FAILURE=0
  DEREGISTER_OLD_AMI=1
  CLEANUP_OLD_WORKER_AMIS=1
  RETAIN_WORKER_AMIS_PER_ARCH=1
  FORCE_BASE_AMI_IF_SOURCE_VOLUME_GT=1

The baked AMI includes system packages, Rust/Cargo, OpenCode, a warm
/opt/ironsmith checkout, prebuilt debug compile_oracle_text, and release helper
binaries by default. Set AMI_BUILD_RELEASE_TOOLS=0 to skip release helpers.
By default, each bake cleans Ironsmith tool/runtime crates before rebuilding
required helper binaries so post-merge AMIs cannot retain stale project
artifacts. Set AMI_CARGO_CLEAN_BEFORE_BUILD=0 to keep all project build caches.
By default, a successful bake deregisters stale tagged worker AMIs and deletes
their snapshots, retaining the current SSM AMI(s) and the newest
RETAIN_WORKER_AMIS_PER_ARCH image(s) per architecture. Set
CLEANUP_OLD_WORKER_AMIS=0 to retain old AMIs.

Set WORKER_ARCH=arm64 to bake an ARM worker AMI. That defaults to:
  AMI_BUILDER_INSTANCE_TYPES="t4g.large c7g.xlarge c6g.xlarge"
  AMI_NAME_PREFIX=ironsmith-card-fixer-worker-arm64
  WORKER_AMI_SSM_PARAM=/ironsmith/card-fixer-worker-ami-arm64
  SOURCE_AMI_SSM_PARAM=/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

AWS=(aws)
if [[ -n "${AWS_PROFILE:-}" ]]; then
  AWS+=(--profile "$AWS_PROFILE")
fi
AWS+=(--region "$AWS_REGION")

TMPDIR="$(mktemp -d)"
BUILDER_INSTANCE_ID=""
AMI_CREATED=0

cleanup() {
  local exit_code=$?
  set +e
  rm -rf "$TMPDIR"
  if [[ "$exit_code" -ne 0 && "$KEEP_BUILDER_ON_FAILURE" != "1" && -n "$BUILDER_INSTANCE_ID" ]]; then
    "${AWS[@]}" ec2 terminate-instances --instance-ids "$BUILDER_INSTANCE_ID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required local command: $1" >&2
    exit 2
  fi
}

require_command aws
require_command git
require_command python3

if [[ ! -f "$CARDS_PATH" ]]; then
  echo "cards.json not found: $CARDS_PATH" >&2
  exit 2
fi

ACCOUNT_ID="$("${AWS[@]}" sts get-caller-identity --query Account --output text)"
BASE_COMMIT="$(git ls-remote "https://github.com/${GITHUB_REPO}.git" "refs/heads/${BASE_BRANCH}" | awk '{print $1}')"
if [[ -z "$BASE_COMMIT" ]]; then
  echo "Could not resolve ${GITHUB_REPO}@${BASE_BRANCH}." >&2
  exit 2
fi
if [[ -z "$BUCKET" ]]; then
  BUCKET="ironsmith-card-fixers-${ACCOUNT_ID}-${AWS_REGION}"
fi
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
SHORT_COMMIT="${BASE_COMMIT:0:12}"
SAFE_BRANCH="$(printf '%s' "$BASE_BRANCH" | tr -c 'A-Za-z0-9._-' '-')"
AMI_NAME="${AMI_NAME_PREFIX}-${SAFE_BRANCH}-${SHORT_COMMIT}-${STAMP}"
ASSET_PREFIX="s3://${BUCKET}/ami-bakes/${STAMP}"

OLD_AMI_ID="$("${AWS[@]}" ssm get-parameter --name "$WORKER_AMI_SSM_PARAM" --query 'Parameter.Value' --output text 2>/dev/null || true)"
SOURCE_AMI_KIND="explicit"
if [[ -z "$SOURCE_AMI_ID" ]]; then
  if [[ -n "$OLD_AMI_ID" && "$OLD_AMI_ID" != "None" ]]; then
    OLD_AMI_STATE="$("${AWS[@]}" ec2 describe-images \
      --image-ids "$OLD_AMI_ID" \
      --query 'Images[0].State' \
      --output text 2>/dev/null || true)"
    if [[ "$OLD_AMI_STATE" == "available" ]]; then
      SOURCE_AMI_ID="$OLD_AMI_ID"
      SOURCE_AMI_KIND="previous-worker"
    fi
  fi
  if [[ -z "$SOURCE_AMI_ID" ]]; then
    SOURCE_AMI_ID="$("${AWS[@]}" ssm get-parameter \
      --name "$SOURCE_AMI_SSM_PARAM" \
      --query 'Parameter.Value' \
      --output text)"
    SOURCE_AMI_KIND="amazon-linux"
  fi
fi
SOURCE_AMI_ARCH="$("${AWS[@]}" ec2 describe-images \
  --image-ids "$SOURCE_AMI_ID" \
  --query 'Images[0].Architecture' \
  --output text 2>/dev/null || true)"
if [[ "$SOURCE_AMI_ARCH" != "$WORKER_ARCH" ]]; then
  echo "Source AMI ${SOURCE_AMI_ID} has architecture ${SOURCE_AMI_ARCH:-unknown}; expected ${WORKER_ARCH}." >&2
  echo "Use SOURCE_AMI_ID or SOURCE_AMI_SSM_PARAM for a matching image." >&2
  exit 2
fi
SOURCE_AMI_VOLUME_SIZE="$("${AWS[@]}" ec2 describe-images \
  --image-ids "$SOURCE_AMI_ID" \
  --query 'Images[0].BlockDeviceMappings[0].Ebs.VolumeSize' \
  --output text 2>/dev/null || true)"
if [[ "$FORCE_BASE_AMI_IF_SOURCE_VOLUME_GT" == "1" \
  && "$SOURCE_AMI_KIND" == "previous-worker" \
  && "$SOURCE_AMI_VOLUME_SIZE" =~ ^[0-9]+$ \
  && "$SOURCE_AMI_VOLUME_SIZE" -gt "$AMI_VOLUME_SIZE" ]]; then
  echo "Previous worker AMI root volume is ${SOURCE_AMI_VOLUME_SIZE} GiB; using Amazon Linux source to shrink to ${AMI_VOLUME_SIZE} GiB."
  SOURCE_AMI_ID="$("${AWS[@]}" ssm get-parameter \
    --name "$SOURCE_AMI_SSM_PARAM" \
    --query 'Parameter.Value' \
    --output text)"
  SOURCE_AMI_KIND="amazon-linux-shrink"
  SOURCE_AMI_ARCH="$("${AWS[@]}" ec2 describe-images \
    --image-ids "$SOURCE_AMI_ID" \
    --query 'Images[0].Architecture' \
    --output text 2>/dev/null || true)"
  if [[ "$SOURCE_AMI_ARCH" != "$WORKER_ARCH" ]]; then
    echo "Shrink source AMI ${SOURCE_AMI_ID} has architecture ${SOURCE_AMI_ARCH:-unknown}; expected ${WORKER_ARCH}." >&2
    exit 2
  fi
fi

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

# IAM instance profile changes can take a few seconds to propagate to EC2.
sleep 15

VPC_ID="$("${AWS[@]}" ec2 describe-vpcs \
  --filters Name=is-default,Values=true \
  --query 'Vpcs[0].VpcId' \
  --output text)"
if [[ -n "${AMI_BUILDER_SUBNET_IDS:-}" ]]; then
  read -r -a BUILDER_SUBNET_IDS <<<"$AMI_BUILDER_SUBNET_IDS"
else
  BUILDER_SUBNET_IDS=()
  while IFS= read -r subnet_id; do
    [[ -n "$subnet_id" && "$subnet_id" != "None" ]] || continue
    BUILDER_SUBNET_IDS+=("$subnet_id")
  done < <("${AWS[@]}" ec2 describe-subnets \
    --filters Name=vpc-id,Values="$VPC_ID" Name=default-for-az,Values=true \
    --query 'sort_by(Subnets,&AvailabilityZone)[].SubnetId' \
    --output text | tr '\t' '\n')
fi
if [[ "${#BUILDER_SUBNET_IDS[@]}" -eq 0 || -z "${BUILDER_SUBNET_IDS[0]}" || "${BUILDER_SUBNET_IDS[0]}" == "None" ]]; then
  echo "Could not find any default subnets in VPC ${VPC_ID}." >&2
  exit 3
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

echo "Baking worker AMI for ${GITHUB_REPO}@${BASE_BRANCH} (${BASE_COMMIT})"
echo "Source AMI: ${SOURCE_AMI_ID} (${SOURCE_AMI_KIND})"
echo "Worker architecture: ${WORKER_ARCH}"
echo "Cards asset: ${CARDS_S3_URI}"
EXPIRES_AT="$(ironsmith_future_utc "$AMI_BUILDER_TTL_HOURS")"

BUILDER_INSTANCE_TYPE_USED=""
BUILDER_SUBNET_ID_USED=""
for candidate_type in $AMI_BUILDER_INSTANCE_TYPES; do
  for subnet_id in "${BUILDER_SUBNET_IDS[@]}"; do
    [[ -n "$subnet_id" && "$subnet_id" != "None" ]] || continue
    echo "Launching builder instance type: ${candidate_type} in subnet ${subnet_id}"
    RUN_INSTANCES_ERR="$TMPDIR/run-instances-${candidate_type}-${subnet_id}.err"
    builder_market_options=()
    if [[ "$AMI_BUILDER_USE_SPOT" == "1" ]]; then
      spot_options="SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate"
      if [[ -n "$AMI_BUILDER_SPOT_MAX_PRICE" ]]; then
        spot_options="${spot_options},MaxPrice=${AMI_BUILDER_SPOT_MAX_PRICE}"
      fi
      builder_market_options=(--instance-market-options "MarketType=spot,SpotOptions={${spot_options}}")
    fi
    if BUILDER_INSTANCE_ID="$("${AWS[@]}" ec2 run-instances \
      --image-id "$SOURCE_AMI_ID" \
      --instance-type "$candidate_type" \
      "${builder_market_options[@]}" \
      --iam-instance-profile "Name=${PROFILE_NAME}" \
      --subnet-id "$subnet_id" \
      --security-group-ids "$SG_ID" \
      --instance-initiated-shutdown-behavior terminate \
      --block-device-mappings "DeviceName=/dev/xvda,Ebs={VolumeSize=${AMI_VOLUME_SIZE},VolumeType=gp3,DeleteOnTermination=true}" \
      --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=ironsmith-card-fixer-ami-builder},{Key=Project,Value=ironsmith-card-fixer},{Key=IronsmithAmiBake,Value=${STAMP}},{Key=IronsmithBaseCommit,Value=${SHORT_COMMIT}},{Key=IronsmithExpiresAt,Value=${EXPIRES_AT}}]" \
        "ResourceType=volume,Tags=[{Key=Name,Value=ironsmith-card-fixer-ami-builder},{Key=Project,Value=ironsmith-card-fixer},{Key=IronsmithAmiBake,Value=${STAMP}},{Key=IronsmithBaseCommit,Value=${SHORT_COMMIT}},{Key=IronsmithExpiresAt,Value=${EXPIRES_AT}}]" \
      --query 'Instances[0].InstanceId' \
      --output text 2>"$RUN_INSTANCES_ERR")"; then
      BUILDER_INSTANCE_TYPE_USED="$candidate_type"
      BUILDER_SUBNET_ID_USED="$subnet_id"
      break 2
    fi
    echo "Could not launch builder type ${candidate_type} in subnet ${subnet_id}; trying next fallback." >&2
    sed 's/^/  /' "$RUN_INSTANCES_ERR" >&2 || true
  done
done
if [[ -z "$BUILDER_INSTANCE_ID" || -z "$BUILDER_INSTANCE_TYPE_USED" ]]; then
  echo "Could not launch any AMI builder instance type/subnet combination: ${AMI_BUILDER_INSTANCE_TYPES} across ${BUILDER_SUBNET_IDS[*]}" >&2
  exit 3
fi
echo "Builder instance: ${BUILDER_INSTANCE_ID} (${BUILDER_INSTANCE_TYPE_USED}, ${BUILDER_SUBNET_ID_USED})"

"${AWS[@]}" ec2 wait instance-running --instance-ids "$BUILDER_INSTANCE_ID"

deadline=$(( $(date +%s) + SSM_ONLINE_TIMEOUT_SECONDS ))
while true; do
  ping_status="$("${AWS[@]}" ssm describe-instance-information \
    --filters "Key=InstanceIds,Values=${BUILDER_INSTANCE_ID}" \
    --query 'InstanceInformationList[0].PingStatus' \
    --output text 2>/dev/null || true)"
  if [[ "$ping_status" == "Online" ]]; then
    break
  fi
  if (( $(date +%s) >= deadline )); then
    echo "Timed out waiting for builder to appear in SSM." >&2
    exit 3
  fi
  sleep 10
done
echo "Builder is online in SSM."

cat > "$TMPDIR/bake-on-instance.sh" <<'REMOTE'
#!/usr/bin/env bash
set -euo pipefail
exec > >(tee -a /var/log/ironsmith-ami-bake.log) 2>&1

export HOME=/root
export PATH="/root/.cargo/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

: "${GITHUB_REPO:?missing GITHUB_REPO}"
: "${BASE_BRANCH:?missing BASE_BRANCH}"
: "${BASE_COMMIT:?missing BASE_COMMIT}"
: "${CARDS_S3_URI:?missing CARDS_S3_URI}"
: "${CARDS_S3_COMPRESSION:=}"
: "${OPENCODE_VERSION:?missing OPENCODE_VERSION}"
: "${AMI_BAKED_AT:?missing AMI_BAKED_AT}"
: "${AMI_BUILD_RELEASE_TOOLS:=1}"
: "${AMI_CARGO_CLEAN_BEFORE_BUILD:=1}"

CURRENT_STAGE=boot
on_remote_error() {
  local exit_code="$1"
  local line="$2"
  echo "AMI bake failed during ${CURRENT_STAGE} at line ${line} with exit ${exit_code}."
  for log in /var/log/ironsmith-ami-bake-*.log /tmp/ironsmith-ami-bake-*.log; do
    [[ -f "$log" ]] || continue
    echo
    echo "===== tail ${log} ====="
    tail -n 220 "$log" || true
  done
  exit "$exit_code"
}
trap 'on_remote_error "$?" "$LINENO"' ERR

CURRENT_STAGE=system_packages
if [[ -f /etc/ironsmith-worker-ami-ready ]] \
  && command -v aws >/dev/null 2>&1 \
  && command -v git >/dev/null 2>&1 \
  && command -v jq >/dev/null 2>&1 \
  && command -v sqlite3 >/dev/null 2>&1 \
  && command -v gcc >/dev/null 2>&1 \
  && command -v g++ >/dev/null 2>&1 \
  && command -v make >/dev/null 2>&1 \
  && command -v perl >/dev/null 2>&1 \
  && command -v pkg-config >/dev/null 2>&1 \
  && command -v python3 >/dev/null 2>&1 \
  && command -v node >/dev/null 2>&1 \
  && command -v npm >/dev/null 2>&1; then
  echo "Using packages from previous worker AMI; skipping dnf update/install."
else
  CURRENT_STAGE=dnf_update
  dnf update -y > /var/log/ironsmith-ami-bake-dnf-update.log 2>&1
  CURRENT_STAGE=dnf_install
  dnf install -y --setopt=install_weak_deps=False \
    awscli \
    git \
    jq \
    sqlite \
    tar \
    gzip \
    gcc \
    gcc-c++ \
    make \
    perl \
    pkgconf-pkg-config \
    openssl-devel \
    python3 \
    nodejs \
    npm > /var/log/ironsmith-ami-bake-dnf-install.log 2>&1
fi

CURRENT_STAGE=swap
if [[ ! -f /swapfile ]]; then
  fallocate -l 12G /swapfile || dd if=/dev/zero of=/swapfile bs=1M count=12288
  chmod 600 /swapfile
  mkswap /swapfile
fi
if ! grep -q '^/swapfile ' /etc/fstab; then
  printf '/swapfile none swap sw 0 0\n' >> /etc/fstab
fi
swapon /swapfile 2>/dev/null || true

CURRENT_STAGE=install_rust
if ! command -v cargo >/dev/null 2>&1; then
  curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal > /var/log/ironsmith-ami-bake-rustup.log 2>&1
elif command -v rustc >/dev/null 2>&1; then
  echo "Using baked Rust toolchain: $(rustc --version)"
fi
command -v cargo >/dev/null 2>&1
CURRENT_STAGE=install_opencode
if command -v opencode >/dev/null 2>&1 && opencode --version 2>/dev/null | grep -Fq "$OPENCODE_VERSION"; then
  echo "Using baked OpenCode: $(opencode --version 2>/dev/null || true)"
else
  npm install -g "opencode-ai@${OPENCODE_VERSION}" > /var/log/ironsmith-ami-bake-opencode.log 2>&1
fi

CURRENT_STAGE=checkout_repo
if [[ -d /opt/ironsmith/.git ]]; then
  cd /opt/ironsmith
  git remote set-url origin "https://github.com/${GITHUB_REPO}.git"
  git fetch origin "$BASE_BRANCH" > /var/log/ironsmith-ami-bake-git-fetch.log 2>&1
else
  rm -rf /opt/ironsmith
  git clone --branch "$BASE_BRANCH" "https://github.com/${GITHUB_REPO}.git" /opt/ironsmith > /var/log/ironsmith-ami-bake-git-clone.log 2>&1
  cd /opt/ironsmith
  git fetch origin "$BASE_BRANCH" >> /var/log/ironsmith-ami-bake-git-clone.log 2>&1
fi
git reset --hard "$BASE_COMMIT"
git clean -ffd
git config --system --add safe.directory /opt/ironsmith || true
CURRENT_STAGE=download_cards
if [[ "$CARDS_S3_COMPRESSION" == "gzip" || "$CARDS_S3_URI" == *.gz ]]; then
  aws s3 cp "$CARDS_S3_URI" /tmp/ironsmith-cards.json.gz > /var/log/ironsmith-ami-bake-download-cards.log 2>&1
  gzip -dc /tmp/ironsmith-cards.json.gz > /opt/ironsmith/cards.json
else
  aws s3 cp "$CARDS_S3_URI" /opt/ironsmith/cards.json > /var/log/ironsmith-ami-bake-download-cards.log 2>&1
fi

if [[ "$AMI_CARGO_CLEAN_BEFORE_BUILD" == "1" ]]; then
  CURRENT_STAGE=cargo_clean_ironsmith_tool_stack
  for package in ironsmith-core ironsmith-compiler ironsmith-registry ironsmith-runtime ironsmith-tools; do
    cargo clean -p "$package" >> /var/log/ironsmith-ami-bake-cargo-clean.log 2>&1
  done
fi

CURRENT_STAGE=cargo_build_debug_compile_oracle_text
cargo build -p ironsmith-tools --bin compile_oracle_text > /var/log/ironsmith-ami-bake-cargo-debug-compile-oracle-text.log 2>&1
if [[ "$AMI_BUILD_RELEASE_TOOLS" == "1" ]]; then
  CURRENT_STAGE=cargo_build_release_compile_oracle_text
  cargo build --release -p ironsmith-tools --bin compile_oracle_text > /var/log/ironsmith-ami-bake-cargo-release-compile-oracle-text.log 2>&1
  CURRENT_STAGE=cargo_build_release_sync_card_status_db
  cargo build --release -p ironsmith-tools --bin sync_card_status_db > /var/log/ironsmith-ami-bake-cargo-release-sync-card-status-db.log 2>&1
fi
CURRENT_STAGE=compile_oracle_text_smoke
target/debug/compile_oracle_text --text "Draw a card." \
  > /tmp/compile_oracle_text-smoke.txt \
  2> /tmp/ironsmith-ami-bake-compile-oracle-text-smoke.log

CURRENT_STAGE=write_metadata
cat > /etc/ironsmith-worker-ami.env <<EOF
IRONSMITH_WORKER_AMI_READY=1
IRONSMITH_WORKER_AMI_BAKED_AT=${AMI_BAKED_AT}
IRONSMITH_WORKER_AMI_GITHUB_REPO=${GITHUB_REPO}
IRONSMITH_WORKER_AMI_BASE_BRANCH=${BASE_BRANCH}
IRONSMITH_WORKER_AMI_BASE_COMMIT=${BASE_COMMIT}
IRONSMITH_WORKER_AMI_ARCH=${WORKER_ARCH}
IRONSMITH_WORKER_AMI_OPENCODE_VERSION=${OPENCODE_VERSION}
IRONSMITH_WORKER_AMI_RELEASE_TOOLS=${AMI_BUILD_RELEASE_TOOLS}
IRONSMITH_WORKER_AMI_CARGO_CLEAN_BEFORE_BUILD=${AMI_CARGO_CLEAN_BEFORE_BUILD}
EOF
touch /etc/ironsmith-worker-ami-ready

CURRENT_STAGE=cleanup
dnf clean all
rm -rf /tmp/* /var/tmp/*
find /var/log -type f -exec truncate -s 0 {} + || true
sync
REMOTE

python3 - "$TMPDIR/bake-on-instance.sh" "$TMPDIR/ssm-params.json" \
  "$GITHUB_REPO" "$BASE_BRANCH" "$BASE_COMMIT" "$CARDS_S3_URI" "$CARDS_S3_COMPRESSION" "$OPENCODE_VERSION" "$STAMP" "$AMI_BAKE_TIMEOUT_SECONDS" "$AMI_BUILD_RELEASE_TOOLS" "$AMI_CARGO_CLEAN_BEFORE_BUILD" "$WORKER_ARCH" <<'PY'
import base64
import json
import pathlib
import shlex
import sys

script_path = pathlib.Path(sys.argv[1])
params_path = pathlib.Path(sys.argv[2])
github_repo, base_branch, base_commit, cards_s3_uri, cards_s3_compression, opencode_version, stamp, timeout, build_release_tools, cargo_clean_before_build, worker_arch = sys.argv[3:]
payload = base64.b64encode(script_path.read_bytes()).decode("ascii")
exports = " ".join(
    f"{name}={shlex.quote(value)}"
    for name, value in [
        ("GITHUB_REPO", github_repo),
        ("BASE_BRANCH", base_branch),
        ("BASE_COMMIT", base_commit),
        ("CARDS_S3_URI", cards_s3_uri),
        ("CARDS_S3_COMPRESSION", cards_s3_compression),
        ("OPENCODE_VERSION", opencode_version),
        ("AMI_BAKED_AT", stamp),
        ("AMI_BUILD_RELEASE_TOOLS", build_release_tools),
        ("AMI_CARGO_CLEAN_BEFORE_BUILD", cargo_clean_before_build),
        ("WORKER_ARCH", worker_arch),
    ]
)
command = (
    f"printf '%s' {shlex.quote(payload)} | base64 -d > /tmp/ironsmith-bake-worker-ami.sh "
    "&& chmod +x /tmp/ironsmith-bake-worker-ami.sh "
    f"&& {exports} /tmp/ironsmith-bake-worker-ami.sh"
)
params_path.write_text(
    json.dumps({"commands": [command], "executionTimeout": [timeout]}),
    encoding="utf-8",
)
PY

COMMAND_ID="$("${AWS[@]}" ssm send-command \
  --instance-ids "$BUILDER_INSTANCE_ID" \
  --document-name AWS-RunShellScript \
  --comment "Bake Ironsmith card fixer worker AMI ${STAMP}" \
  --parameters "file://$TMPDIR/ssm-params.json" \
  --query 'Command.CommandId' \
  --output text)"
echo "SSM command: ${COMMAND_ID}"

while true; do
  status="$("${AWS[@]}" ssm get-command-invocation \
    --command-id "$COMMAND_ID" \
    --instance-id "$BUILDER_INSTANCE_ID" \
    --query 'Status' \
    --output text 2>/dev/null || true)"
  case "$status" in
    Success)
      break
      ;;
    Pending|InProgress|Delayed|"")
      sleep 20
      ;;
    *)
      echo "AMI bake command failed with status: ${status}" >&2
      "${AWS[@]}" ssm get-command-invocation \
        --command-id "$COMMAND_ID" \
        --instance-id "$BUILDER_INSTANCE_ID" \
        --query '{Stdout:StandardOutputContent,Stderr:StandardErrorContent}' \
        --output json >&2 || true
      exit 4
      ;;
  esac
done
echo "AMI bake command completed successfully."

create_image_args=()
if "${AWS[@]}" ec2 stop-instances --instance-ids "$BUILDER_INSTANCE_ID" >/dev/null 2>&1; then
  "${AWS[@]}" ec2 wait instance-stopped --instance-ids "$BUILDER_INSTANCE_ID"
else
  echo "Could not stop builder ${BUILDER_INSTANCE_ID}; creating AMI from the running instance with --no-reboot." >&2
  create_image_args+=(--no-reboot)
fi

IMAGE_ID="$("${AWS[@]}" ec2 create-image \
  --instance-id "$BUILDER_INSTANCE_ID" \
  --name "$AMI_NAME" \
  --description "Ironsmith card fixer worker AMI for ${GITHUB_REPO}@${BASE_BRANCH} ${BASE_COMMIT}" \
  "${create_image_args[@]}" \
  --tag-specifications "ResourceType=image,Tags=[{Key=Name,Value=${AMI_NAME}},{Key=IronsmithWorkerAmi,Value=true},{Key=IronsmithWorkerArch,Value=${WORKER_ARCH}},{Key=IronsmithBaseCommit,Value=${SHORT_COMMIT}},{Key=IronsmithBakedAt,Value=${STAMP}}]" \
                      "ResourceType=snapshot,Tags=[{Key=Name,Value=${AMI_NAME}},{Key=IronsmithWorkerAmi,Value=true},{Key=IronsmithWorkerArch,Value=${WORKER_ARCH}},{Key=IronsmithBaseCommit,Value=${SHORT_COMMIT}},{Key=IronsmithBakedAt,Value=${STAMP}}]" \
  --query ImageId \
  --output text)"
echo "Created AMI: ${IMAGE_ID}"

"${AWS[@]}" ec2 wait image-available --image-ids "$IMAGE_ID"
AMI_CREATED=1

"${AWS[@]}" ssm put-parameter \
  --name "$WORKER_AMI_SSM_PARAM" \
  --type String \
  --overwrite \
  --value "$IMAGE_ID" >/dev/null
"${AWS[@]}" ssm put-parameter \
  --name "${WORKER_AMI_METADATA_SSM_PREFIX}/base-commit" \
  --type String \
  --overwrite \
  --value "$BASE_COMMIT" >/dev/null
"${AWS[@]}" ssm put-parameter \
  --name "${WORKER_AMI_METADATA_SSM_PREFIX}/baked-at" \
  --type String \
  --overwrite \
  --value "$STAMP" >/dev/null
"${AWS[@]}" ssm put-parameter \
  --name "${WORKER_AMI_METADATA_SSM_PREFIX}/name" \
  --type String \
  --overwrite \
  --value "$AMI_NAME" >/dev/null
"${AWS[@]}" ssm put-parameter \
  --name "${WORKER_AMI_METADATA_SSM_PREFIX}/arch" \
  --type String \
  --overwrite \
  --value "$WORKER_ARCH" >/dev/null

if [[ -n "$OLD_AMI_ID" && "$OLD_AMI_ID" != "$IMAGE_ID" && "$DEREGISTER_OLD_AMI" == "1" ]]; then
  OLD_SNAPSHOTS_FILE="$TMPDIR/old-snapshots.txt"
  "${AWS[@]}" ec2 describe-images \
    --image-ids "$OLD_AMI_ID" \
    --query 'Images[0].BlockDeviceMappings[].Ebs.SnapshotId' \
    --output text 2>/dev/null | tr '\t' '\n' > "$OLD_SNAPSHOTS_FILE" || true
  "${AWS[@]}" ec2 deregister-image --image-id "$OLD_AMI_ID" >/dev/null 2>&1 || true
  while IFS= read -r snapshot_id; do
    [[ -n "$snapshot_id" && "$snapshot_id" != "None" ]] || continue
    "${AWS[@]}" ec2 delete-snapshot --snapshot-id "$snapshot_id" >/dev/null 2>&1 || true
  done < "$OLD_SNAPSHOTS_FILE"
  echo "Deregistered previous AMI: ${OLD_AMI_ID}"
fi

if [[ "$CLEANUP_OLD_WORKER_AMIS" == "1" ]]; then
  AWS_PROFILE="${AWS_PROFILE:-}" \
  AWS_REGION="$AWS_REGION" \
  RETAIN_WORKER_AMIS_PER_ARCH="$RETAIN_WORKER_AMIS_PER_ARCH" \
  EXTRA_PROTECTED_AMI_IDS="$IMAGE_ID" \
    "$ROOT/scripts/aws_card_fixers/cleanup_worker_amis.sh"
fi

"${AWS[@]}" ec2 terminate-instances --instance-ids "$BUILDER_INSTANCE_ID" >/dev/null 2>&1 || true
echo "Published ${IMAGE_ID} to ${WORKER_AMI_SSM_PARAM}"
echo "Account: ${ACCOUNT_ID}, region: ${AWS_REGION}"
