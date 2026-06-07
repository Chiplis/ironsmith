#!/usr/bin/env bash
set -euo pipefail

exec > >(tee -a /var/log/ironsmith-card-fixer-worker.log) 2>&1

export HOME=/root
export PATH="/root/.cargo/bin:/usr/local/bin:/usr/bin:/bin:$PATH"

if [[ -f /etc/ironsmith-worker-ami.env ]]; then
  # shellcheck disable=SC1091
  source /etc/ironsmith-worker-ami.env
fi

: "${CARD_NAME_B64:?missing CARD_NAME_B64}"
: "${PARSE_ERROR_B64:?missing PARSE_ERROR_B64}"
: "${S3_ASSET_PREFIX:?missing S3_ASSET_PREFIX}"
: "${CARDS_S3_URI:=}"
: "${CARDS_S3_COMPRESSION:=}"
: "${GITHUB_REPO:?missing GITHUB_REPO}"
: "${BASE_BRANCH:=main}"
: "${GITHUB_TOKEN_SSM_PARAM:=}"
: "${OPENCODE_AUTH_JSON_SSM_PARAM:=}"
: "${OPENAI_API_KEY_SSM_PARAM:=}"
: "${OPENCODE_VERSION:=1.15.9}"
: "${OPENCODE_MODEL:=openai/gpt-5.5-fast}"
: "${OPENCODE_VARIANT:=fast}"
: "${OPENCODE_FAST_REASONING_EFFORT:=high}"
: "${OPENCODE_FAST_TEXT_VERBOSITY:=low}"
: "${OPENCODE_FAST_SERVICE_TIER:=priority}"
: "${OPENCODE_STALE_TIMEOUT_SECONDS:=1800}"
: "${OPENCODE_HEARTBEAT_SECONDS:=60}"
: "${OPENCODE_NO_COMMIT_RETRIES:=1}"
: "${PRE_PR_STEWARD_MAX_REPAIRS:=1}"
: "${POST_PR_STEWARD_MAX_REPAIRS:=3}"
: "${WORKER_ENTRY_SKILL:=ironsmith-aws-card-fixer-fleet}"
: "${BAKED_WORKER_AMI:=0}"
: "${SELF_TERMINATE:=1}"

decode_b64() {
  python3 -c 'import base64,sys; print(base64.b64decode(sys.argv[1]).decode())' "$1"
}

slugify() {
  python3 -c 'import re,sys; s=re.sub(r"[^a-z0-9]+","-",sys.argv[1].lower()).strip("-"); print(s[:48] or "card")' "$1"
}

metadata_get() {
  local path="$1"
  local token
  token="$(curl -fsS --connect-timeout 2 --max-time 5 \
    -X PUT "http://169.254.169.254/latest/api/token" \
    -H "X-aws-ec2-metadata-token-ttl-seconds: 21600" 2>/dev/null || true)"
  if [[ -n "$token" ]]; then
    curl -fsS --connect-timeout 2 --max-time 5 \
      -H "X-aws-ec2-metadata-token: ${token}" \
      "http://169.254.169.254/latest/meta-data/${path}"
  else
    curl -fsS --connect-timeout 2 --max-time 5 \
      "http://169.254.169.254/latest/meta-data/${path}"
  fi
}

CARD_NAME="$(decode_b64 "$CARD_NAME_B64")"
PARSE_ERROR="$(decode_b64 "$PARSE_ERROR_B64")"
INSTANCE_ID="$(metadata_get instance-id || hostname)"
CARD_SLUG="$(slugify "$CARD_NAME")"
BRANCH="codex/aws-card-fix-${CARD_SLUG}"
WORKDIR=/opt/ironsmith
COMPILE_ORACLE_TEXT_BIN="${WORKDIR}/target/debug/compile_oracle_text"
COMPILE_ORACLE_TEXT_CMD="compile_oracle_text_worker"
STATUS_DIR=/tmp/ironsmith-card-fixer-status
STATUS_FILE="${STATUS_DIR}/${INSTANCE_ID}.json"
EVENTS_FILE="${STATUS_DIR}/${INSTANCE_ID}.jsonl"
CURRENT_STEP=boot
PR_URL=""
PR_NUMBER=""
SPOT_INTERRUPTION_HANDLED=0
RESUMING_EXISTING_PR=0
WORK_START_HEAD=""
WORKER_BASE_HEAD=""

configure_opencode_fast_variant() {
  if [[ -z "${OPENCODE_VARIANT:-}" ]]; then
    return
  fi

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
}

report_status() {
  local state="$1"
  local step="$2"
  local message="${3:-}"
  local ts
  ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  mkdir -p "$STATUS_DIR"
  python3 - "$STATUS_FILE" "$EVENTS_FILE" "$ts" "$state" "$step" "$message" \
    "$INSTANCE_ID" "$CARD_NAME" "$BRANCH" "${OPENCODE_STATUS:-}" "${PR_URL:-}" <<'PY'
import json
import sys

status_file, events_file, ts, state, step, message, instance_id, card_name, branch, opencode_status, pr_url = sys.argv[1:]
payload = {
    "timestamp": ts,
    "state": state,
    "step": step,
    "message": message,
    "instance_id": instance_id,
    "card_name": card_name,
    "branch": branch,
}
if opencode_status:
    payload["opencode_status"] = int(opencode_status)
if pr_url:
    payload["pr_url"] = pr_url
with open(status_file, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
with open(events_file, "a", encoding="utf-8") as handle:
    json.dump(payload, handle, sort_keys=True)
    handle.write("\n")
PY
  aws s3 cp "$STATUS_FILE" "${S3_ASSET_PREFIX}/status/${INSTANCE_ID}.json" >/dev/null 2>&1 || true
  aws s3 cp "$EVENTS_FILE" "${S3_ASSET_PREFIX}/events/${INSTANCE_ID}.jsonl" >/dev/null 2>&1 || true
}

upload_artifacts() {
  local artifact_dir="${STATUS_DIR}/artifacts"
  rm -rf "$artifact_dir"
  mkdir -p "$artifact_dir"

  if [[ -d "$WORKDIR/.git" ]]; then
    (
      cd "$WORKDIR"
      git status --short --branch > "${artifact_dir}/git-status.txt" 2>&1 || true
      git log --oneline --decorate -20 > "${artifact_dir}/git-log.txt" 2>&1 || true
      git diff --stat > "${artifact_dir}/git-diff-stat.txt" 2>&1 || true
      git diff > "${artifact_dir}/git-diff.patch" 2>&1 || true
      git diff "origin/${BASE_BRANCH}...HEAD" > "${artifact_dir}/branch-diff.patch" 2>&1 || true
      [[ -f reports/opencode-run.jsonl ]] && cp reports/opencode-run.jsonl "${artifact_dir}/opencode-run.jsonl"
      [[ -f reports/selected-card-status.txt ]] && cp reports/selected-card-status.txt "${artifact_dir}/selected-card-status.txt"
    )
  fi

  if [[ -n "${PROMPT_FILE:-}" && -f "$PROMPT_FILE" ]]; then
    cp "$PROMPT_FILE" "${artifact_dir}/prompt.md"
  fi
  if [[ -n "${POST_PR_STEWARD_PROMPT_FILE:-}" && -f "$POST_PR_STEWARD_PROMPT_FILE" ]]; then
    cp "$POST_PR_STEWARD_PROMPT_FILE" "${artifact_dir}/post-pr-steward-prompt.md"
  fi
  if [[ -n "${POST_PR_REPAIR_PROMPT_FILE:-}" && -f "$POST_PR_REPAIR_PROMPT_FILE" ]]; then
    cp "$POST_PR_REPAIR_PROMPT_FILE" "${artifact_dir}/post-pr-repair-prompt.md"
  fi
  if [[ -f "$WORKDIR/reports/post-pr-steward-result.json" ]]; then
    cp "$WORKDIR/reports/post-pr-steward-result.json" "${artifact_dir}/post-pr-steward-result.json"
  fi
  if [[ -f "$WORKDIR/reports/post-pr-repair-result.json" ]]; then
    cp "$WORKDIR/reports/post-pr-repair-result.json" "${artifact_dir}/post-pr-repair-result.json"
  fi
  if [[ -f /var/log/ironsmith-card-fixer-worker.log ]]; then
    cp /var/log/ironsmith-card-fixer-worker.log "${artifact_dir}/worker.log" || true
  fi

  aws s3 cp --recursive "$artifact_dir/" "${S3_ASSET_PREFIX}/artifacts/${INSTANCE_ID}/" >/dev/null 2>&1 || true
}

github_api() {
  local method="$1"
  local path="$2"
  local data_file="${3:-}"
  local args=(
    -fsS
    -X "$method"
    -H "Authorization: Bearer ${GITHUB_TOKEN}"
    -H "Accept: application/vnd.github+json"
    -H "X-GitHub-Api-Version: 2022-11-28"
  )
  if [[ -n "$data_file" ]]; then
    args+=(--data "@${data_file}")
  fi
  curl "${args[@]}" "https://api.github.com/repos/${GITHUB_REPO}${path}"
}

comment_on_pr() {
  local body="$1"
  if [[ -z "${PR_NUMBER:-}" ]]; then
    return
  fi
  body="$(printf '%b' "$body")"

  jq -n --arg body "$body" '{body:$body}' > /tmp/pr-comment.json
  github_api POST "/issues/${PR_NUMBER}/comments" /tmp/pr-comment.json >/dev/null || true
}

lookup_existing_pr_for_branch() {
  local owner="${GITHUB_REPO%%/*}"
  local encoded_head
  encoded_head="$(python3 - "$owner" "$BRANCH" <<'PY'
from urllib.parse import quote
import sys
print(quote(f"{sys.argv[1]}:{sys.argv[2]}", safe=""))
PY
)"
  github_api GET "/pulls?state=open&head=${encoded_head}&base=${BASE_BRANCH}&per_page=1" \
    > reports/existing-pr-response.json || true
  PR_URL="$(jq -r '.[0].html_url // empty' reports/existing-pr-response.json 2>/dev/null || true)"
  PR_NUMBER="$(jq -r '.[0].number // empty' reports/existing-pr-response.json 2>/dev/null || true)"
}

create_or_reuse_pr() {
  git remote set-url origin "https://x-access-token:${GITHUB_TOKEN}@github.com/${GITHUB_REPO}.git"
  git push -u origin "$BRANCH"

  lookup_existing_pr_for_branch
  if [[ -n "$PR_URL" && -n "$PR_NUMBER" ]]; then
    report_status succeeded create_pr "$PR_URL"
    comment_on_pr "Worker ${INSTANCE_ID} is resuming work on this card from EC2 Spot instance \`${INSTANCE_ID}\`.\n\nStatus/artifacts prefix: \`${S3_ASSET_PREFIX}\`"
    return
  fi

  PR_BODY_FILE=/tmp/ironsmith-card-fixer-pr-body.md
  cat > "$PR_BODY_FILE" <<EOF
Automated Ironsmith card-fixer worker.

Card: ${CARD_NAME}
Initial parse error:
\`\`\`
${PARSE_ERROR}
\`\`\`

Current worker: ${INSTANCE_ID}
Branch: ${BRANCH}
Status/artifacts prefix: ${S3_ASSET_PREFIX}
EOF

  jq -n \
    --arg title "Fix Ironsmith parse failure: ${CARD_NAME}" \
    --arg head "$BRANCH" \
    --arg base "$BASE_BRANCH" \
    --rawfile body "$PR_BODY_FILE" \
    '{title:$title, head:$head, base:$base, body:$body, draft:true}' \
    > /tmp/pr.json

  github_api POST "/pulls" /tmp/pr.json | tee reports/pr-response.json
  PR_URL="$(jq -r '.html_url // empty' reports/pr-response.json)"
  PR_NUMBER="$(jq -r '.number // empty' reports/pr-response.json)"
  if [[ -z "$PR_URL" || -z "$PR_NUMBER" ]]; then
    report_status failed create_pr "GitHub did not return a PR URL/number"
    upload_artifacts
    exit 1
  fi
  report_status succeeded create_pr "$PR_URL"
  comment_on_pr "Worker ${INSTANCE_ID} started work on EC2 Spot instance \`${INSTANCE_ID}\`."
}

spot_instance_action() {
  metadata_get "spot/instance-action" 2>/dev/null || true
}

check_spot_or_exit() {
  local notice
  notice="$(spot_instance_action)"
  if [[ -n "$notice" ]]; then
    handle_spot_interruption "$notice"
    exit 125
  fi
}

commit_and_push_wip_snapshot() {
  local message="$1"
  local pushed=0

  git remote set-url origin "https://x-access-token:${GITHUB_TOKEN}@github.com/${GITHUB_REPO}.git"
  if [[ -n "$(git status --porcelain)" ]]; then
    git add -A -- . \
      ':!reports/opencode-run.jsonl' \
      ':!reports/selected-card-status.txt' \
      ':!reports/post-pr-steward-result.json' \
      ':!reports/post-pr-repair-result.json'
    if ! git diff --cached --quiet; then
      git commit -m "$message" || true
    fi
  fi
  if [[ -n "$(git log "origin/${BRANCH}..HEAD" --oneline 2>/dev/null || true)" ]]; then
    git push origin "$BRANCH" && pushed=1
  fi
  printf '%s\n' "$pushed"
}

handle_spot_interruption() {
  local notice="${1:-}"
  if [[ "$SPOT_INTERRUPTION_HANDLED" == "1" ]]; then
    return
  fi
  SPOT_INTERRUPTION_HANDLED=1
  CURRENT_STEP=spot_interruption_notice
  echo "Spot interruption notice received: ${notice}"

  local pushed=0
  if [[ -d "$WORKDIR/.git" ]]; then
    (
      cd "$WORKDIR"
      pushed="$(commit_and_push_wip_snapshot "WIP before Spot interruption: ${CARD_NAME}")"
      printf '%s\n' "$pushed" > /tmp/ironsmith-spot-wip-pushed
    ) || true
    pushed="$(cat /tmp/ironsmith-spot-wip-pushed 2>/dev/null || printf '0')"
  fi

  upload_artifacts
  comment_on_pr "EC2 Spot interruption notice received on worker \`${INSTANCE_ID}\`.\n\nThe worker is stopping and has uploaded current artifacts under:\n\`${S3_ASSET_PREFIX}/artifacts/${INSTANCE_ID}/\`\n\nCurrent branch push attempted: ${pushed}."
  report_status failed "$CURRENT_STEP" "Spot interruption notice; artifacts uploaded; branch push attempted=${pushed}"
}

has_substantive_branch_commit() {
  git log "origin/${BASE_BRANCH}..HEAD" --format=%s 2>/dev/null \
    | grep -v -F "Start Ironsmith parse fix: ${CARD_NAME}" \
    | grep -q .
}

on_error() {
  local exit_code=$?
  upload_artifacts
  report_status failed "$CURRENT_STEP" "worker failed at line ${BASH_LINENO[0]} with exit ${exit_code}"
  exit "$exit_code"
}

on_exit() {
  local exit_code=$?
  if [[ "$SELF_TERMINATE" == "1" ]]; then
    echo "Worker exiting with status ${exit_code}; shutting down for EC2 termination."
    sync || true
    shutdown -h now || true
  fi
}

trap on_error ERR
trap 'handle_spot_interruption "termination signal received"; exit 125' TERM INT
trap on_exit EXIT

echo "Starting Ironsmith card fixer worker for: ${CARD_NAME}"

CURRENT_STEP=install_dependencies
if [[ "$BAKED_WORKER_AMI" == "1" ]] \
  && command -v git >/dev/null 2>&1 \
  && command -v jq >/dev/null 2>&1 \
  && command -v sqlite3 >/dev/null 2>&1 \
  && command -v node >/dev/null 2>&1 \
  && command -v npm >/dev/null 2>&1; then
  report_status succeeded "$CURRENT_STEP" "using baked system packages"
else
  dnf update -y
  dnf install -y \
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
    npm
  report_status succeeded "$CURRENT_STEP" "system packages installed"
fi

if [[ ! -f /swapfile ]]; then
  fallocate -l 8G /swapfile || dd if=/dev/zero of=/swapfile bs=1M count=8192
  chmod 600 /swapfile
  mkswap /swapfile
fi
if ! grep -q '^/swapfile ' /etc/fstab; then
  printf '/swapfile none swap sw 0 0\n' >> /etc/fstab
fi
swapon /swapfile 2>/dev/null || true

CURRENT_STEP=install_rust
if ! command -v cargo >/dev/null 2>&1; then
  curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
rustup default stable
report_status succeeded "$CURRENT_STEP" "$(rustc --version 2>/dev/null || true)"

CURRENT_STEP=install_opencode
if [[ "$BAKED_WORKER_AMI" != "1" || ! -x "$(command -v opencode 2>/dev/null || true)" ]]; then
  npm install -g "opencode-ai@${OPENCODE_VERSION}"
fi
report_status succeeded "$CURRENT_STEP" "$(opencode --version 2>/dev/null || true)"

CURRENT_STEP=opencode_auth
if [[ -n "$OPENCODE_AUTH_JSON_SSM_PARAM" ]]; then
  mkdir -p /root/.local/share/opencode
  aws ssm get-parameter \
    --name "$OPENCODE_AUTH_JSON_SSM_PARAM" \
    --with-decryption \
    --query 'Parameter.Value' \
    --output text > /root/.local/share/opencode/auth.json
  chmod 600 /root/.local/share/opencode/auth.json
elif [[ -n "$OPENAI_API_KEY_SSM_PARAM" ]]; then
  export OPENAI_API_KEY
  OPENAI_API_KEY="$(aws ssm get-parameter \
    --name "$OPENAI_API_KEY_SSM_PARAM" \
    --with-decryption \
    --query 'Parameter.Value' \
    --output text)"
else
  echo "No OpenCode auth SSM parameter was provided; worker is provisioned but cannot run OpenCode."
  report_status failed "$CURRENT_STEP" "missing OpenCode auth SSM parameter"
  exit 20
fi
opencode auth list >/tmp/opencode-auth-list.txt
configure_opencode_fast_variant
report_status succeeded "$CURRENT_STEP" "OpenCode auth configured"

CURRENT_STEP=github_auth
GITHUB_TOKEN=""
if [[ -n "$GITHUB_TOKEN_SSM_PARAM" ]]; then
  GITHUB_TOKEN="$(aws ssm get-parameter \
    --name "$GITHUB_TOKEN_SSM_PARAM" \
    --with-decryption \
    --query 'Parameter.Value' \
    --output text)"
else
  echo "No GitHub token SSM parameter was provided; worker cannot push or open a PR."
  report_status failed "$CURRENT_STEP" "missing GitHub token SSM parameter"
  exit 21
fi
report_status succeeded "$CURRENT_STEP" "GitHub token retrieved"

CURRENT_STEP=clone_repo
if [[ "$BAKED_WORKER_AMI" == "1" && -d "$WORKDIR/.git" ]]; then
  cd "$WORKDIR"
  git remote set-url origin "https://github.com/${GITHUB_REPO}.git"
  git fetch origin "$BASE_BRANCH"
  git reset --hard "origin/${BASE_BRANCH}"
  git clean -ffd
else
  rm -rf "$WORKDIR"
  git clone "https://github.com/${GITHUB_REPO}.git" "$WORKDIR"
fi
cd "$WORKDIR"
WORKER_BASE_HEAD="$(git rev-parse "origin/${BASE_BRANCH}")"
report_status succeeded "$CURRENT_STEP" "repository ready"

CURRENT_STEP=download_assets
if [[ -z "$CARDS_S3_URI" ]]; then
  CARDS_S3_URI="$(aws s3 cp "${S3_ASSET_PREFIX}/cards-uri.txt" - 2>/dev/null || true)"
fi
if [[ -n "$CARDS_S3_URI" ]]; then
  if [[ "$CARDS_S3_COMPRESSION" == "gzip" || "$CARDS_S3_URI" == *.gz ]]; then
    aws s3 cp "$CARDS_S3_URI" /tmp/ironsmith-cards.json.gz
    gzip -dc /tmp/ironsmith-cards.json.gz > cards.json
  else
    aws s3 cp "$CARDS_S3_URI" cards.json
  fi
else
  aws s3 cp "${S3_ASSET_PREFIX}/cards.json" cards.json
fi
aws s3 cp "${S3_ASSET_PREFIX}/ironsmith-skills.tar.gz" /tmp/ironsmith-skills.tar.gz
mkdir -p /root/.codex/skills reports
tar -xzf /tmp/ironsmith-skills.tar.gz -C /root/.codex/skills
mkdir -p /Users/chiplis/.codex
ln -sfn /root/.codex/skills /Users/chiplis/.codex/skills
if [[ ! -f "/root/.codex/skills/${WORKER_ENTRY_SKILL}/SKILL.md" ]]; then
  echo "Missing worker entry skill: /root/.codex/skills/${WORKER_ENTRY_SKILL}/SKILL.md"
  report_status failed "$CURRENT_STEP" "missing worker entry skill ${WORKER_ENTRY_SKILL}"
  exit 22
fi
if [[ ! -f /root/.codex/skills/ironsmith-card-fixer/SKILL.md ]]; then
  echo "Missing required skill: /root/.codex/skills/ironsmith-card-fixer/SKILL.md"
  report_status failed "$CURRENT_STEP" "missing ironsmith-card-fixer skill"
  exit 22
fi
report_status succeeded "$CURRENT_STEP" "cards.json and skills downloaded"

cat > reports/selected-card-status.txt <<STATUS
card_name: ${CARD_NAME}
parse_error: ${PARSE_ERROR}
selected_from: reports/engine-status.sqlite3 on launcher
STATUS

git config user.name "ironsmith-card-fixer"
git config user.email "ironsmith-card-fixer@users.noreply.github.com"
CURRENT_STEP=create_branch
if git ls-remote --exit-code --heads origin "$BRANCH" >/dev/null 2>&1; then
  git fetch origin "$BRANCH"
  git checkout -B "$BRANCH" "origin/${BRANCH}"
  RESUMING_EXISTING_PR=1
  report_status succeeded "$CURRENT_STEP" "resuming ${BRANCH}"
else
  git checkout -b "$BRANCH" "origin/${BASE_BRANCH}"
  git commit --allow-empty -m "Start Ironsmith parse fix: ${CARD_NAME}"
  report_status succeeded "$CURRENT_STEP" "$BRANCH"
fi

CURRENT_STEP=create_pr
create_or_reuse_pr
WORK_START_HEAD="$(git rev-parse HEAD)"
check_spot_or_exit

CURRENT_STEP=prebuild_tools
cat > /usr/local/bin/compile_oracle_text_worker <<'HELPER'
#!/usr/bin/env bash
set -euo pipefail
cd /opt/ironsmith
bin=target/debug/compile_oracle_text
needs_build=0
if [[ ! -x "$bin" ]]; then
  needs_build=1
elif find . \
    -path ./target -prune -o \
    -path ./.git -prune -o \
    \( -name '*.rs' -o -name Cargo.toml -o -name Cargo.lock \) \
    -newer "$bin" -print -quit | grep -q .; then
  needs_build=1
fi
if [[ "$needs_build" == "1" ]]; then
  cargo build -p ironsmith-tools --bin compile_oracle_text
fi
exec "$bin" "$@"
HELPER
chmod +x /usr/local/bin/compile_oracle_text_worker

if [[ -x "$COMPILE_ORACLE_TEXT_BIN" ]] \
  && { [[ -z "${IRONSMITH_WORKER_AMI_BASE_COMMIT:-}" ]] || [[ "${IRONSMITH_WORKER_AMI_BASE_COMMIT}" == "$WORKER_BASE_HEAD" ]]; }; then
  report_status succeeded "$CURRENT_STEP" "using baked compile_oracle_text for ${WORKER_BASE_HEAD:0:12}"
else
  report_status running "$CURRENT_STEP" "building compile_oracle_text for ${WORKER_BASE_HEAD:0:12}"
  cargo build -p ironsmith-tools --bin compile_oracle_text
  report_status succeeded "$CURRENT_STEP" "compile_oracle_text ready"
fi
check_spot_or_exit

PROMPT_FILE=/tmp/ironsmith-card-fixer-prompt.md
cat > "$PROMPT_FILE" <<EOF
Read /root/.codex/skills/${WORKER_ENTRY_SKILL}/SKILL.md first and follow its worker contract.
Then read /root/.codex/skills/ironsmith-card-fixer/SKILL.md and use it as the single-card
implementation workflow.

When either skill mentions another Ironsmith skill such as \$ironsmith-parser-only-card-fix,
read the corresponding /root/.codex/skills/<skill-name>/SKILL.md file directly. The worker has
those skill files on disk, even though OpenCode does not expose a native skill picker. If a skill
file references /Users/chiplis/.codex/skills, use the equivalent /root/.codex/skills path; a
compatibility symlink is also present.

This is a normal coding task in a checked-out repository. The launcher and monitor own card
reservation, agent_running, and pr_created state; do not update reports/engine-status.sqlite3.
Fix exactly the assigned card on the current branch.

This worker creates or reuses a draft PR before implementation begins.
PR: ${PR_URL}
PR number: ${PR_NUMBER}
Branch: ${BRANCH}
Resuming existing PR: ${RESUMING_EXISTING_PR}
If this is a resumed PR, inspect the current branch history and existing PR context before changing
code; prior Spot-interrupted workers may have pushed WIP commits and artifact links in PR comments.

Do not ask the user for clarification or permission. If a narrow reusable parser, lowering,
runtime, or text-rendering change is the obvious next step for this assigned card, implement it.
Only stop with a clean working tree when you have a concrete technical blocker that makes a safe
single-card-worker fix impossible.

Parser/lowering shape generalization is in scope for a single-card worker when it uses existing
runtime effects. Do not classify a parser helper refactor, effect-chain split, or "this parser path
currently returns one effect but needs sibling effects" as out of scope by itself. Implement that
reusable parser/lowering change and tests. Treat it as a blocker only if the card truly needs a new
runtime effect executor, event type, state model, or cross-card campaign beyond this worker.

Use tools to edit files and run verification. The expected deliverable is a focused,
commit-worthy code/test diff that the wrapper can commit, push, and open as a PR. Do not create a
report-only change under reports/cards/. If you determine code cannot be changed safely for this
card, leave the working tree clean and explain the blocker in your final response. Do not answer
that you are ready to proceed, ask to continue, or request permission; this worker run is already
authorized to make the change.

Run one complete Ironsmith single-card fix pass for this parse-failing card:

Card: ${CARD_NAME}
Current parse error from reports/engine-status.sqlite3:
${PARSE_ERROR}

Target: get the card to parse strictly and reach semantic similarity >= 0.99 if feasible.

Follow the ironsmith-card-fixer workflow. Use the checked-out repo and cards.json in the repo root.
Use live verification with:
${COMPILE_ORACLE_TEXT_CMD} --name "${CARD_NAME}" --compare-text

Make the narrowest reusable parser/lowering/runtime/text-rendering fix. Do not hardcode this card's
name or oracle text.

Tests are mandatory for a code fix. Add a complete card-specific regression set for "${CARD_NAME}",
not just a generic helper test:
- a strict parser/oracle-text regression that names "${CARD_NAME}" and proves the card parses;
- a compiled-text comparison or assertion that covers the clause or mechanic this fix adds;
- runtime behavior tests for every game-state-dependent effect touched by the card, especially
  costs, alternative/additional costs, targets, zones, triggers, replacement/prevention effects,
  counters, conditional effects, or "was paid" labels;
- negative or branch-condition coverage when the oracle text has a meaningful "if", "unless",
  target legality, optional cost, zone, timing, or mode boundary.

Do not leave a code-fix diff that lacks the relevant card tests. If the needed change is outside
single-card scope, leave the working tree clean; do not turn the run into a report-only PR.

Do not commit, push, create a PR, sync the DB, bake an AMI, or launch AWS workers. When done, run
git status and leave a useful uncommitted code/test diff for the wrapper to review, commit, push,
and publish as a PR.
EOF

parse_latest_opencode_error() {
  python3 - reports/opencode-run.jsonl <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
if not path.exists():
    raise SystemExit(0)

latest = ""
for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
    line = line.strip()
    if not line:
        continue
    try:
        event = json.loads(line)
    except json.JSONDecodeError:
        continue
    if event.get("type") != "error":
        continue
    error = event.get("error")
    if isinstance(error, dict):
        data = error.get("data")
        if isinstance(data, dict) and data.get("message"):
            latest = str(data["message"])
        elif error.get("message"):
            latest = str(error["message"])
        else:
            latest = json.dumps(error, sort_keys=True)
    else:
        latest = str(error)

if latest:
    print(latest[:500])
PY
}

run_opencode_pass() {
  local prompt_path="$1"
  local pass_label="$2"

  set +e
  CURRENT_STEP=opencode_run
  report_status running "$CURRENT_STEP" "starting OpenCode card-fixer pass (${pass_label})"
  OPENCODE_STALE_MARKER=/tmp/ironsmith-opencode-stale
  OPENCODE_SPOT_MARKER=/tmp/ironsmith-opencode-spot-interruption
  rm -f "$OPENCODE_STALE_MARKER"
  rm -f "$OPENCODE_SPOT_MARKER"
  local opencode_args=(
    run
    --dir "$WORKDIR"
    --model "$OPENCODE_MODEL"
    --dangerously-skip-permissions
    --format json
  )
  if [[ -n "${OPENCODE_VARIANT:-}" ]]; then
    opencode_args+=(--variant "$OPENCODE_VARIANT")
  fi
  opencode "${opencode_args[@]}" "$(< "$prompt_path")" > >(tee -a reports/opencode-run.jsonl) 2>&1 &
  OPENCODE_PID=$!

  (
    last_size=-1
    last_progress="$(date +%s)"
    while kill -0 "$OPENCODE_PID" >/dev/null 2>&1; do
      now="$(date +%s)"
      current_size=0
      if [[ -f reports/opencode-run.jsonl ]]; then
        current_size="$(stat -c %s reports/opencode-run.jsonl 2>/dev/null || printf '0')"
      fi
      if [[ "$current_size" != "$last_size" ]]; then
        last_size="$current_size"
        last_progress="$now"
      fi
      idle_seconds=$((now - last_progress))
      report_status running "$CURRENT_STEP" "OpenCode running (${pass_label}); output idle ${idle_seconds}s"
      spot_notice="$(spot_instance_action)"
      if [[ -n "$spot_notice" ]]; then
        printf '%s\n' "$spot_notice" > "$OPENCODE_SPOT_MARKER"
        kill "$OPENCODE_PID" >/dev/null 2>&1 || true
        sleep 5
        kill -KILL "$OPENCODE_PID" >/dev/null 2>&1 || true
        break
      fi
      if (( OPENCODE_STALE_TIMEOUT_SECONDS > 0 && idle_seconds >= OPENCODE_STALE_TIMEOUT_SECONDS )); then
        printf 'OpenCode output made no progress for %ss; marking worker failed.\n' "$idle_seconds" > "$OPENCODE_STALE_MARKER"
        kill "$OPENCODE_PID" >/dev/null 2>&1 || true
        sleep 10
        kill -KILL "$OPENCODE_PID" >/dev/null 2>&1 || true
        break
      fi
      sleep "$OPENCODE_HEARTBEAT_SECONDS"
    done
  ) &
  OPENCODE_WATCHDOG_PID=$!

  wait "$OPENCODE_PID"
  OPENCODE_STATUS=$?
  if kill -0 "$OPENCODE_WATCHDOG_PID" >/dev/null 2>&1; then
    kill "$OPENCODE_WATCHDOG_PID" >/dev/null 2>&1 || true
  fi
  wait "$OPENCODE_WATCHDOG_PID" >/dev/null 2>&1 || true
  set -e

  if [[ -f "$OPENCODE_SPOT_MARKER" ]]; then
    handle_spot_interruption "$(< "$OPENCODE_SPOT_MARKER")"
    exit 125
  fi

  if [[ -f "$OPENCODE_STALE_MARKER" ]]; then
    OPENCODE_STALE_MESSAGE="$(< "$OPENCODE_STALE_MARKER")"
    echo "$OPENCODE_STALE_MESSAGE"
    upload_artifacts
    report_status failed no_progress "$OPENCODE_STALE_MESSAGE"
    exit 124
  fi

  OPENCODE_ERROR_MESSAGE="$(parse_latest_opencode_error)"
  if [[ -n "$OPENCODE_ERROR_MESSAGE" ]]; then
    OPENCODE_STATUS=1
    echo "OpenCode emitted an error: ${OPENCODE_ERROR_MESSAGE}"
    report_status failed "$CURRENT_STEP" "OpenCode error: ${OPENCODE_ERROR_MESSAGE:0:220}"
  elif [[ "$OPENCODE_STATUS" -ne 0 ]]; then
    echo "OpenCode exited with status ${OPENCODE_STATUS}; preserving logs and attempting to commit any useful artifacts."
    report_status failed "$CURRENT_STEP" "OpenCode exited with ${OPENCODE_STATUS}"
  else
    report_status succeeded "$CURRENT_STEP" "OpenCode completed (${pass_label})"
  fi
}

: > reports/opencode-run.jsonl
run_opencode_pass "$PROMPT_FILE" "primary"

retry_index=0
while [[ -z "$(git status --porcelain)" && "$OPENCODE_STATUS" -eq 0 && "$retry_index" -lt "$OPENCODE_NO_COMMIT_RETRIES" ]]; do
  retry_index=$((retry_index + 1))
  RETRY_PROMPT_FILE="/tmp/ironsmith-card-fixer-retry-${retry_index}.md"
  cat "$PROMPT_FILE" > "$RETRY_PROMPT_FILE"
  cat >> "$RETRY_PROMPT_FILE" <<EOF

Important retry instruction:

Your previous OpenCode pass exited successfully but left no working-tree changes.
Continue from the current repository state. Do not ask for clarification or permission. Either make
the focused reusable parser/runtime/test changes needed for the assigned card and verify them, or
leave the working tree clean and explain the concrete technical blocker in your final response.
Parser/lowering shape generalization with existing runtime effects is in scope; do not stop merely
because a helper currently returns one effect but needs to return or split sibling effects. Do not
answer that you are ready to proceed or ask whether to continue. Do not create a report-only change
under reports/cards/. The wrapper can only
commit and open a PR when this pass leaves a commit-worthy change.
EOF
  report_status running no_commit_retry "retrying OpenCode after no-diff pass ${retry_index}/${OPENCODE_NO_COMMIT_RETRIES}"
  run_opencode_pass "$RETRY_PROMPT_FILE" "no-diff retry ${retry_index}/${OPENCODE_NO_COMMIT_RETRIES}"
done

run_dirty_worktree_steward() {
  if [[ -z "$(git status --porcelain)" ]]; then
    return
  fi

  CURRENT_STEP=local_steward_review
  report_status running "$CURRENT_STEP" "starting local dirty-worktree steward review"

  STEWARD_PROMPT_FILE=/tmp/ironsmith-card-fixer-steward-prompt.md
  cat > "$STEWARD_PROMPT_FILE" <<EOF
Read /root/.codex/skills/ironsmith-pr-merge-steward/SKILL.md first and apply its quality-gate rules
to this worker's dirty worktree.

You are running inside ${WORKDIR} on a worker branch for exactly one card, before any commit or PR
has been created.

Card: ${CARD_NAME}
Original parse error:
${PARSE_ERROR}

Review the uncommitted diff as if it were a worker PR:
- Inspect the intended oracle text for "${CARD_NAME}" and the current dirty diff.
- Reject report-only changes under reports/cards/; this worker should not create gap-report PRs.
- Reject mechanically wrong shortcuts, card-name hardcoding, broad semantic normalization that hides
  missing behavior, keyword aliases that erase keyword identity, or diffs that only make the text
  look closer while parser/lowering/runtime semantics are wrong.
- Require a complete card-specific test set for code changes: strict parser/oracle coverage,
  compiled-text coverage, and runtime or branch-condition tests for game-state-dependent behavior.
- Run or preserve focused verification, including:
  ${COMPILE_ORACLE_TEXT_CMD} --name "${CARD_NAME}" --compare-text
- If the dirty worktree is acceptable, repair or tighten it as needed and leave the focused
  uncommitted changes in place.
- If it is not acceptable but the correct reusable repair is apparent, make that repair now and
  leave the focused uncommitted changes in place.
- Revert the worker's changes and leave the working tree clean only when no safe single-card-worker
  repair can be made. Do not commit, push, create a PR, sync the DB, bake an AMI, or launch AWS
  workers.
EOF

  run_opencode_pass "$STEWARD_PROMPT_FILE" "local steward review"

  if [[ -z "$(git status --porcelain)" ]]; then
    report_status running "$CURRENT_STEP" "local steward rejected dirty worktree; repair may follow"
  else
    report_status succeeded "$CURRENT_STEP" "local steward left commit-worthy changes"
  fi
}

run_dirty_worktree_repair_once() {
  local attempt="$1"
  CURRENT_STEP=local_steward_repair
  report_status running "$CURRENT_STEP" "starting local steward repair ${attempt}/${PRE_PR_STEWARD_MAX_REPAIRS}"

  LOCAL_REPAIR_PROMPT_FILE="/tmp/ironsmith-card-fixer-local-steward-repair-${attempt}.md"
  cat > "$LOCAL_REPAIR_PROMPT_FILE" <<EOF
Read /root/.codex/skills/${WORKER_ENTRY_SKILL}/SKILL.md first, then
/root/.codex/skills/ironsmith-card-fixer/SKILL.md. Use the narrowest applicable Ironsmith
subsystem skill for the repair.

You are running inside ${WORKDIR} on a worker branch for exactly one card, before any commit or PR
has been created.

Card: ${CARD_NAME}
Local repair attempt: ${attempt} of ${PRE_PR_STEWARD_MAX_REPAIRS}
Original parse error:
${PARSE_ERROR}

The previous OpenCode pass or local dirty-worktree steward review left no commit-worthy changes.
Review reports/opencode-run.jsonl for the prior conversation, no-diff outcome, or steward feedback,
then proceed without asking for clarification.

Fix exactly this card on this same branch. Make the narrowest reusable parser/lowering/runtime/text
rendering repair. Do not hardcode this card's name or oracle text. Do not create a report-only
change under reports/cards/.

Parser/lowering shape generalization is in scope when it uses existing runtime effects. If the prior
pass stopped because a parser path currently returns one effect but this card needs sibling effects,
implement the reusable chain/split/lowering change and tests rather than calling that out of scope.
Do not ask whether to continue; either leave a verified code/test diff or a concrete blocker that
requires a new runtime effect executor, event type, or state model.

Keep or add a complete card-specific test set for "${CARD_NAME}":
- strict parser/oracle coverage;
- compiled-text coverage for the fixed clause or mechanic;
- runtime or branch-condition tests for game-state-dependent behavior.

Run focused verification, including:
${COMPILE_ORACLE_TEXT_CMD} --name "${CARD_NAME}" --compare-text

When done, leave a commit-worthy code/test diff in the working tree. If you cannot make a safe
single-card-worker repair, leave the working tree clean and explain the concrete blocker.
EOF

  run_opencode_pass "$LOCAL_REPAIR_PROMPT_FILE" "local steward repair ${attempt}/${PRE_PR_STEWARD_MAX_REPAIRS}"

  if [[ -n "$(git status --porcelain)" ]]; then
    run_dirty_worktree_steward
  fi
}

run_dirty_worktree_steward

local_repair_attempt=0
while [[ -z "$(git status --porcelain)" && "$OPENCODE_STATUS" -eq 0 && "$local_repair_attempt" -lt "$PRE_PR_STEWARD_MAX_REPAIRS" ]]; do
  local_repair_attempt=$((local_repair_attempt + 1))
  run_dirty_worktree_repair_once "$local_repair_attempt"
done

CURRENT_STEP=commit_changes
if [[ -n "$(git status --porcelain)" ]]; then
  git add -u -- .
  git ls-files --others --exclude-standard -z | xargs -0 -r git add --
  git commit -m "Fix Ironsmith parse failure: ${CARD_NAME}" || true
  report_status succeeded "$CURRENT_STEP" "$(git log -1 --oneline 2>/dev/null || true)"
else
  echo "No working tree changes to commit for ${CARD_NAME}."
fi

if ! has_substantive_branch_commit; then
  echo "No implementation commit was created for ${CARD_NAME}; leaving the early draft PR open for inspection."
  if [[ "$OPENCODE_STATUS" -ne 0 ]]; then
    report_status failed opencode_run "OpenCode failed before creating changes: ${OPENCODE_ERROR_MESSAGE:-status ${OPENCODE_STATUS}}"
  else
    report_status failed no_commit "no implementation commit was created"
  fi
  comment_on_pr "Worker ${INSTANCE_ID} stopped without creating an implementation commit. The early draft PR is left open so a replacement worker can resume the card on the same branch."
  upload_artifacts
  exit "$OPENCODE_STATUS"
fi

CURRENT_STEP=push_branch
git remote set-url origin "https://x-access-token:${GITHUB_TOKEN}@github.com/${GITHUB_REPO}.git"
git push origin "$BRANCH"
report_status succeeded "$CURRENT_STEP" "$BRANCH"

parse_json_field() {
  local path="$1"
  local field="$2"
  local fallback="$3"
  python3 - "$path" "$field" "$fallback" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
field = sys.argv[2]
fallback = sys.argv[3]
if not path.exists():
    print(fallback)
    raise SystemExit(0)
try:
    row = json.loads(path.read_text(encoding="utf-8"))
except Exception:
    print(fallback)
    raise SystemExit(0)
value = str(row.get(field, "")).strip()
print(value if value else fallback)
PY
}

commit_and_push_branch_changes() {
  local message="$1"
  if [[ -z "$(git status --porcelain)" ]]; then
    return 1
  fi

  git add -A -- . \
    ':!reports/opencode-run.jsonl' \
    ':!reports/selected-card-status.txt' \
    ':!reports/post-pr-steward-result.json' \
    ':!reports/post-pr-repair-result.json'
  if git diff --cached --quiet; then
    return 1
  fi

  git commit -m "$message" || true
  if [[ -n "$(git log "origin/${BRANCH}..HEAD" --oneline 2>/dev/null || true)" ]]; then
    git push origin "$BRANCH"
  fi
  return 0
}

run_post_pr_steward_review_once() {
  local attempt="$1"
  CURRENT_STEP=post_pr_steward_review
  report_status running "$CURRENT_STEP" "starting worker-local PR steward review ${attempt}"

  POST_PR_STEWARD_PROMPT_FILE=/tmp/ironsmith-card-fixer-post-pr-steward-prompt.md
  cat > "$POST_PR_STEWARD_PROMPT_FILE" <<EOF
Read /root/.codex/skills/ironsmith-pr-merge-steward/SKILL.md first and apply its quality-gate rules
as a worker-local review of this single draft PR. This pass is review-only.

You are running inside ${WORKDIR} on the same EC2 worker that created this PR.

PR: ${PR_URL}
PR number: ${PR_NUMBER}
Branch: ${BRANCH}
Base branch: ${BASE_BRANCH}
Card: ${CARD_NAME}
Review attempt: ${attempt}
Original parse error:
${PARSE_ERROR}

Review exactly this PR's diff against origin/${BASE_BRANCH}. Do not merge this PR, push main,
sync reports/engine-status.sqlite3, bake an AMI, launch AWS workers, review other PRs, close this PR,
or edit files in this review pass.

Apply the merge-steward quality gate to this branch:
- Inspect the intended oracle text for "${CARD_NAME}" and this PR's implementation diff.
- Reject report-only changes under reports/cards/.
- Reject mechanically wrong shortcuts, card-name hardcoding, broad semantic normalization that hides
  missing behavior, keyword aliases that erase keyword identity, or diffs that only make text look
  closer while parser/lowering/runtime semantics are wrong.
- Require a complete card-specific test set for code changes.
- Run or preserve focused verification, including:
  ${COMPILE_ORACLE_TEXT_CMD} --name "${CARD_NAME}" --compare-text

If the PR is acceptable, write:
  reports/post-pr-steward-result.json
with {"decision":"approved","summary":"..."}.

If the PR is not acceptable yet, write clear actionable feedback for the next repair session:
  reports/post-pr-steward-result.json
with {"decision":"rejected","summary":"..."}.
EOF

  rm -f reports/post-pr-steward-result.json
  run_opencode_pass "$POST_PR_STEWARD_PROMPT_FILE" "post-PR steward review"

  if [[ "$OPENCODE_STATUS" -ne 0 ]]; then
    report_status failed "$CURRENT_STEP" "post-PR steward OpenCode failed"
    upload_artifacts
    exit "$OPENCODE_STATUS"
  fi

  POST_PR_STEWARD_DECISION="$(parse_json_field reports/post-pr-steward-result.json decision missing)"
  POST_PR_STEWARD_DECISION="$(printf '%s' "$POST_PR_STEWARD_DECISION" | tr '[:upper:]' '[:lower:]')"
  POST_PR_STEWARD_SUMMARY="$(parse_json_field reports/post-pr-steward-result.json summary "no steward summary")"
}

run_post_pr_repair_once() {
  local attempt="$1"
  local feedback="$2"
  CURRENT_STEP=post_pr_repair
  report_status running "$CURRENT_STEP" "starting post-PR repair ${attempt}/${POST_PR_STEWARD_MAX_REPAIRS}"

  POST_PR_REPAIR_PROMPT_FILE="/tmp/ironsmith-card-fixer-post-pr-repair-${attempt}.md"
  cat > "$POST_PR_REPAIR_PROMPT_FILE" <<EOF
Read /root/.codex/skills/${WORKER_ENTRY_SKILL}/SKILL.md first, then
/root/.codex/skills/ironsmith-card-fixer/SKILL.md. Use the narrowest applicable Ironsmith
subsystem skill for the repair.

You are running inside ${WORKDIR} on the worker branch for one draft PR.

PR: ${PR_URL}
PR number: ${PR_NUMBER}
Branch: ${BRANCH}
Base branch: ${BASE_BRANCH}
Card: ${CARD_NAME}
Repair attempt: ${attempt} of ${POST_PR_STEWARD_MAX_REPAIRS}
Original parse error:
${PARSE_ERROR}

The worker-local PR steward rejected the previous revision with this feedback:
${feedback}

Fix the PR on this same branch until the steward feedback is addressed. Do not close the PR, create
another PR, merge this PR, push main, sync reports/engine-status.sqlite3, bake an AMI, launch AWS
workers, or edit unrelated cards.

Make the narrowest reusable parser/lowering/runtime/text-rendering repair. Do not hardcode this
card's name or oracle text. Keep or add complete card-specific tests for "${CARD_NAME}" and run
focused verification, including:
${COMPILE_ORACLE_TEXT_CMD} --name "${CARD_NAME}" --compare-text

When done, leave a commit-worthy code/test diff in the working tree and write:
  reports/post-pr-repair-result.json
with {"decision":"fixed","summary":"..."}.

If you cannot make a safe single-card-worker repair, leave the working tree clean and write:
  reports/post-pr-repair-result.json
with {"decision":"blocked","summary":"..."}.
EOF

  rm -f reports/post-pr-repair-result.json
  run_opencode_pass "$POST_PR_REPAIR_PROMPT_FILE" "post-PR repair ${attempt}/${POST_PR_STEWARD_MAX_REPAIRS}"

  if [[ "$OPENCODE_STATUS" -ne 0 ]]; then
    report_status failed "$CURRENT_STEP" "post-PR repair OpenCode failed"
    upload_artifacts
    exit "$OPENCODE_STATUS"
  fi

  POST_PR_REPAIR_DECISION="$(parse_json_field reports/post-pr-repair-result.json decision missing)"
  POST_PR_REPAIR_DECISION="$(printf '%s' "$POST_PR_REPAIR_DECISION" | tr '[:upper:]' '[:lower:]')"
  POST_PR_REPAIR_SUMMARY="$(parse_json_field reports/post-pr-repair-result.json summary "no repair summary")"

  if commit_and_push_branch_changes "Address PR steward feedback: ${CARD_NAME}"; then
    report_status succeeded "$CURRENT_STEP" "repair pushed: ${POST_PR_REPAIR_SUMMARY}"
    return 0
  fi

  report_status failed "$CURRENT_STEP" "repair produced no commit-worthy changes: ${POST_PR_REPAIR_SUMMARY}"
  return 1
}

post_pr_steward_review() {
  local repair_attempt=0

  while true; do
    run_post_pr_steward_review_once "$((repair_attempt + 1))"

    case "$POST_PR_STEWARD_DECISION" in
      approved)
        report_status succeeded post_pr_steward_review "approved: ${POST_PR_STEWARD_SUMMARY}"
        return 0
        ;;
      rejected)
        if (( repair_attempt >= POST_PR_STEWARD_MAX_REPAIRS )); then
          comment_on_pr "Worker-local post-PR steward still rejects this draft PR after ${POST_PR_STEWARD_MAX_REPAIRS} repair attempt(s). The PR is intentionally left open for inspection.\n\nLatest feedback:\n${POST_PR_STEWARD_SUMMARY}"
          report_status failed post_pr_steward_review "rejected after repairs: ${POST_PR_STEWARD_SUMMARY}"
          upload_artifacts
          exit 1
        fi
        repair_attempt=$((repair_attempt + 1))
        if ! run_post_pr_repair_once "$repair_attempt" "$POST_PR_STEWARD_SUMMARY"; then
          comment_on_pr "Worker-local post-PR repair attempt ${repair_attempt}/${POST_PR_STEWARD_MAX_REPAIRS} did not produce a commit-worthy fix. The PR is intentionally left open for inspection.\n\nSteward feedback:\n${POST_PR_STEWARD_SUMMARY}\n\nRepair result:\n${POST_PR_REPAIR_SUMMARY:-no repair summary}"
          upload_artifacts
          exit 1
        fi
        ;;
      *)
        if (( repair_attempt >= POST_PR_STEWARD_MAX_REPAIRS )); then
          comment_on_pr "Worker-local post-PR steward did not write a valid approval/rejection after ${POST_PR_STEWARD_MAX_REPAIRS} repair attempt(s). The PR is intentionally left open for inspection.\n\nLatest result: ${POST_PR_STEWARD_DECISION}\n${POST_PR_STEWARD_SUMMARY}"
          report_status failed post_pr_steward_review "invalid steward decision: ${POST_PR_STEWARD_DECISION}"
          upload_artifacts
          exit 1
        fi
        repair_attempt=$((repair_attempt + 1))
        if ! run_post_pr_repair_once "$repair_attempt" "The steward review did not write a valid reports/post-pr-steward-result.json decision. Re-check the PR against the quality gate and fix any issues you find."; then
          upload_artifacts
          exit 1
        fi
        ;;
    esac
  done
}

post_pr_steward_review

echo "Worker complete for ${CARD_NAME}"
CURRENT_STEP=complete
report_status succeeded "$CURRENT_STEP" "worker complete"
