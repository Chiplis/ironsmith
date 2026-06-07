#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GITHUB_REPO="${GITHUB_REPO:-Chiplis/ironsmith}"
BASE_BRANCH="${BASE_BRANCH:-main}"
AWS_REGION="${AWS_REGION:-us-east-2}"
GITHUB_TOKEN_SSM_PARAM="${GITHUB_TOKEN_SSM_PARAM:-}"
PR_URL_FILE="${PR_URL_FILE:-}"
CODEX_HANDLE_REMAINING_PRS="${CODEX_HANDLE_REMAINING_PRS:-1}"
STEWARD_HANDLE_REMAINING_PRS="${STEWARD_HANDLE_REMAINING_PRS:-$CODEX_HANDLE_REMAINING_PRS}"
STEWARD_COMMAND="${STEWARD_COMMAND:-${CODEX_COMMAND:-opencode}}"
STEWARD_MODEL="${STEWARD_MODEL:-${CODEX_MODEL:-${OPENCODE_MODEL:-}}}"
STEWARD_VARIANT="${STEWARD_VARIANT:-${CODEX_VARIANT:-${OPENCODE_VARIANT:-fast}}}"
STEWARD_OUTPUT_FILE="${STEWARD_OUTPUT_FILE:-${CODEX_OUTPUT_FILE:-}}"
SAFE_MERGE_VERIFY_COMMAND="${SAFE_MERGE_VERIFY_COMMAND:-cargo check --workspace -j ${CARGO_BUILD_JOBS:-1}}"
PUSH_SAFE_MERGES="${PUSH_SAFE_MERGES:-1}"
REQUIRE_CLEAN_TRACKED="${REQUIRE_CLEAN_TRACKED:-1}"
QUALITY_GATE_ALL_PRS="${QUALITY_GATE_ALL_PRS:-0}"

usage() {
  cat <<EOF
Usage:
  PR_URL_FILE=reports/aws-card-fixer-dev-loop/RUN/batch0001-prs.txt \\
  scripts/aws_card_fixers/merge_batch_prs.sh

Or pass PR URLs/numbers as arguments:
  scripts/aws_card_fixers/merge_batch_prs.sh https://github.com/Chiplis/ironsmith/pull/123

Optional env:
  GITHUB_REPO=Chiplis/ironsmith
  BASE_BRANCH=main
  PR_URL_FILE=path/to/pr-urls.txt
  STEWARD_HANDLE_REMAINING_PRS=1
  STEWARD_COMMAND=opencode
  STEWARD_MODEL=
  STEWARD_VARIANT=
  STEWARD_OUTPUT_FILE=
  SAFE_MERGE_VERIFY_COMMAND='cargo check --workspace -j 1'
  PUSH_SAFE_MERGES=1
  REQUIRE_CLEAN_TRACKED=1
  QUALITY_GATE_ALL_PRS=0

This helper first merges open, non-overlapping, conflict-free PRs locally. By
default, clean merges are verified and pushed without another OpenCode pass
because workers run their own post-PR steward review before reporting success.
PRs with overlapping touched files or textual conflicts are still handed to
OpenCode with the Ironsmith PR merge steward skill.
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

require_command gh
require_command git
require_command python3

if [[ "$STEWARD_HANDLE_REMAINING_PRS" == "1" ]]; then
  require_command "$STEWARD_COMMAND"
fi

github_token() {
  if [[ -n "${GH_TOKEN:-}" ]]; then
    printf '%s\n' "$GH_TOKEN"
    return
  fi
  if [[ -n "${GITHUB_TOKEN:-}" ]]; then
    printf '%s\n' "$GITHUB_TOKEN"
    return
  fi
  if [[ -n "$GITHUB_TOKEN_SSM_PARAM" ]] && command -v aws >/dev/null 2>&1; then
    aws_args=(aws --region "$AWS_REGION")
    if [[ -n "${AWS_PROFILE:-}" ]]; then
      aws_args+=(--profile "$AWS_PROFILE")
    fi
    "${aws_args[@]}" ssm get-parameter \
      --name "$GITHUB_TOKEN_SSM_PARAM" \
      --with-decryption \
      --query 'Parameter.Value' \
      --output text 2>/dev/null || true
  fi
}

configure_authenticated_origin() {
  local token
  token="$(github_token)"
  if [[ -z "$token" || "$token" == "None" ]]; then
    return
  fi
  git remote set-url origin "https://x-access-token:${token}@github.com/${GITHUB_REPO}.git"
}

run_rebase_conflict_steward() {
  if [[ "$STEWARD_HANDLE_REMAINING_PRS" != "1" ]]; then
    echo "Rebase conflict needs OpenCode handling, but STEWARD_HANDLE_REMAINING_PRS=0." >&2
    return 1
  fi

  local prompt_file="$TMPDIR/steward-rebase-conflict-prompt.md"
  local merge_steward_skill="/root/.codex/skills/ironsmith-pr-merge-steward/SKILL.md"
  if [[ ! -f "$merge_steward_skill" ]]; then
    merge_steward_skill="$HOME/.codex/skills/ironsmith-pr-merge-steward/SKILL.md"
  fi

  {
    printf 'Read %s first and follow it.\n\n' "$merge_steward_skill"
    printf 'You are running inside %s on `%s` after a push to origin was rejected and `git rebase origin/%s` hit conflicts.\n\n' "$ROOT" "$BASE_BRANCH" "$BASE_BRANCH"
    printf 'Resolve the current rebase conflicts as the Ironsmith PR merge steward.\n\n'
    printf 'Important constraints:\n'
    printf -- '- Do not launch AWS workers, run the dev loop, sync the card status DB, or bake an AMI.\n'
    printf -- '- Inspect the conflict markers, the local commits being replayed, and `origin/%s` before choosing resolutions.\n' "$BASE_BRANCH"
    printf -- '- Preserve mechanically correct worker changes and reject or revert mechanically wrong changes, using the same quality gate as normal PR stewardship.\n'
    printf -- '- Prefer generalized parser/lowering/runtime support over card-specific code.\n'
    printf -- '- If the rebase is still in progress, stage resolved files and run `git rebase --continue` until the rebase completes or you determine it must be aborted.\n'
    printf -- '- Run focused verification for the touched code and affected cards when practical; at minimum run the narrow tests or compile checks needed to validate conflict resolutions.\n'
    printf -- '- Leave the repository on `%s` with no tracked working tree changes, no staged changes, and no active rebase. Do not push; the caller will retry the push.\n' "$BASE_BRANCH"
  } > "$prompt_file"

  echo
  echo "Handing rebase conflict to OpenCode..."
  steward_args=(run --dir "$ROOT" --dangerously-skip-permissions)
  if [[ -n "$STEWARD_MODEL" ]]; then
    steward_args+=(--model "$STEWARD_MODEL")
  fi
  if [[ -n "$STEWARD_VARIANT" ]]; then
    steward_args+=(--variant "$STEWARD_VARIANT")
  fi

  if [[ -n "$STEWARD_OUTPUT_FILE" ]]; then
    "$STEWARD_COMMAND" "${steward_args[@]}" "$(< "$prompt_file")" | tee -a "$STEWARD_OUTPUT_FILE"
  else
    "$STEWARD_COMMAND" "${steward_args[@]}" "$(< "$prompt_file")"
  fi

  if [[ -d .git/rebase-merge || -d .git/rebase-apply ]]; then
    echo "OpenCode returned with a rebase still in progress; refusing to continue." >&2
    git status --short >&2
    return 1
  fi
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "OpenCode returned with tracked working tree changes still present after rebase conflict handling." >&2
    git status --short >&2
    return 1
  fi
}

push_base_branch_with_retry() {
  local attempts="${PUSH_RETRY_ATTEMPTS:-3}"
  local attempt

  configure_authenticated_origin
  for ((attempt = 1; attempt <= attempts; attempt++)); do
    if git push origin "$BASE_BRANCH"; then
      return 0
    fi

    if (( attempt == attempts )); then
      break
    fi

    echo "Push to origin/${BASE_BRANCH} failed; fetching and rebasing before retry ${attempt}/${attempts}." >&2
    git fetch origin "$BASE_BRANCH"
    if ! git rebase "origin/${BASE_BRANCH}"; then
      run_rebase_conflict_steward
    fi
  done

  echo "Push to origin/${BASE_BRANCH} failed after ${attempts} attempt(s)." >&2
  return 1
}

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

RAW_INPUT_FILE="$TMPDIR/raw-pr-input.txt"
PR_NUMBERS_FILE="$TMPDIR/pr-numbers.txt"
METADATA_FILE="$TMPDIR/pr-metadata.jsonl"
SAFE_CANDIDATES_FILE="$TMPDIR/safe-candidates.tsv"
DEFERRED_FILE="$TMPDIR/deferred.tsv"
SKIPPED_FILE="$TMPDIR/skipped.tsv"
CLEAN_MERGED_FILE="$TMPDIR/clean-merged.tsv"
CLEAN_CONFLICTS_FILE="$TMPDIR/clean-conflicts.tsv"
REMAINING_FILE="$TMPDIR/remaining.tsv"

: > "$RAW_INPUT_FILE"
: > "$METADATA_FILE"
: > "$SAFE_CANDIDATES_FILE"
: > "$DEFERRED_FILE"
: > "$SKIPPED_FILE"
: > "$CLEAN_MERGED_FILE"
: > "$CLEAN_CONFLICTS_FILE"
: > "$REMAINING_FILE"

if [[ -n "$PR_URL_FILE" ]]; then
  if [[ ! -f "$PR_URL_FILE" ]]; then
    echo "PR_URL_FILE not found: $PR_URL_FILE" >&2
    exit 2
  fi
  grep -v '^[[:space:]]*$' "$PR_URL_FILE" >> "$RAW_INPUT_FILE" || true
fi

for item in "$@"; do
  printf '%s\n' "$item" >> "$RAW_INPUT_FILE"
done

python3 - "$RAW_INPUT_FILE" > "$PR_NUMBERS_FILE" <<'PY'
import pathlib
import re
import sys

path = pathlib.Path(sys.argv[1])
seen = set()
for raw in path.read_text(encoding="utf-8").splitlines():
    value = raw.strip()
    if not value or value.startswith("#"):
        continue
    match = re.search(r"(?:/pull/|^#?)(\d+)(?:\b|$)", value)
    if not match:
        print(f"Could not parse PR number from: {value}", file=sys.stderr)
        raise SystemExit(2)
    number = match.group(1)
    if number in seen:
        continue
    seen.add(number)
    print(number)
PY

if [[ ! -s "$PR_NUMBERS_FILE" ]]; then
  echo "No PRs to merge."
  exit 0
fi

cd "$ROOT"
configure_authenticated_origin

if [[ "$REQUIRE_CLEAN_TRACKED" == "1" ]]; then
  git update-index -q --refresh || true
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Tracked working tree changes are present; refusing to merge PRs automatically." >&2
    git status --short >&2
    exit 3
  fi
fi

git fetch origin "$BASE_BRANCH"
if [[ "$(git branch --show-current)" != "$BASE_BRANCH" ]]; then
  git checkout "$BASE_BRANCH"
fi
git pull --ff-only origin "$BASE_BRANCH"

echo "Inspecting PR metadata..."
while IFS= read -r pr_number; do
  [[ -n "$pr_number" ]] || continue
  pr_json="$TMPDIR/pr-${pr_number}.json"
  gh pr view "$pr_number" \
    --repo "$GITHUB_REPO" \
    --json number,title,url,headRefName,baseRefName,state,isDraft,files \
    > "$pr_json"
  python3 - "$pr_json" >> "$METADATA_FILE" <<'PY'
import json
import pathlib
import sys

row = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
row["files"] = [
    item["path"] if isinstance(item, dict) else str(item)
    for item in row.get("files", [])
]
print(json.dumps(row, sort_keys=True))
PY
done < "$PR_NUMBERS_FILE"

python3 - "$METADATA_FILE" "$BASE_BRANCH" "$SAFE_CANDIDATES_FILE" "$DEFERRED_FILE" "$SKIPPED_FILE" <<'PY'
from collections import Counter
import json
import pathlib
import sys

metadata_path = pathlib.Path(sys.argv[1])
base_branch = sys.argv[2]
safe_path = pathlib.Path(sys.argv[3])
deferred_path = pathlib.Path(sys.argv[4])
skipped_path = pathlib.Path(sys.argv[5])

rows = [
    json.loads(line)
    for line in metadata_path.read_text(encoding="utf-8").splitlines()
    if line.strip()
]
open_base = [
    row
    for row in rows
    if row.get("state") == "OPEN" and row.get("baseRefName") == base_branch
]
file_counts = Counter(
    path
    for row in open_base
    for path in row.get("files", [])
)

safe_lines = []
deferred_lines = []
skipped_lines = []

for row in rows:
    number = str(row["number"])
    title = row.get("title", "").replace("\t", " ")
    url = row.get("url", "")
    state = row.get("state", "")
    base = row.get("baseRefName", "")
    files = row.get("files", [])

    if state != "OPEN":
        skipped_lines.append(f"{number}\tnot_open:{state}\t{url}\t{title}")
        continue
    if base != base_branch:
        skipped_lines.append(f"{number}\twrong_base:{base}\t{url}\t{title}")
        continue

    overlapping = sorted(path for path in files if file_counts[path] > 1)
    if overlapping:
        preview = ",".join(overlapping[:5])
        if len(overlapping) > 5:
            preview += ",..."
        deferred_lines.append(f"{number}\toverlapping_files:{preview}\t{url}\t{title}")
        continue

    safe_lines.append(f"{number}\t{url}\t{title}")

safe_path.write_text("\n".join(safe_lines) + ("\n" if safe_lines else ""), encoding="utf-8")
deferred_path.write_text("\n".join(deferred_lines) + ("\n" if deferred_lines else ""), encoding="utf-8")
skipped_path.write_text("\n".join(skipped_lines) + ("\n" if skipped_lines else ""), encoding="utf-8")
PY

safe_count="$(wc -l < "$SAFE_CANDIDATES_FILE" | tr -d ' ')"
deferred_count="$(wc -l < "$DEFERRED_FILE" | tr -d ' ')"
skipped_count="$(wc -l < "$SKIPPED_FILE" | tr -d ' ')"
echo "PR classification: safe_candidates=${safe_count}, deferred=${deferred_count}, skipped=${skipped_count}"

if [[ -s "$SKIPPED_FILE" ]]; then
  echo
  echo "Skipped PRs:"
  sed 's/^/  /' "$SKIPPED_FILE"
fi

verification_failed=0

while IFS=$'\t' read -r pr_number pr_url pr_title; do
  [[ -n "${pr_number:-}" ]] || continue
  ref="refs/remotes/origin/pr/${pr_number}"
  echo
  echo "Trying clean merge for PR #${pr_number}: ${pr_title}"
  git fetch origin "+pull/${pr_number}/head:${ref}"
  before_head="$(git rev-parse HEAD)"
  if git merge --no-ff --no-edit "$ref"; then
    after_head="$(git rev-parse HEAD)"
    if [[ "$before_head" != "$after_head" ]]; then
      printf '%s\t%s\t%s\n' "$pr_number" "$pr_url" "$pr_title" >> "$CLEAN_MERGED_FILE"
      echo "Merged PR #${pr_number} cleanly."
    else
      printf '%s\talready_merged\t%s\t%s\n' "$pr_number" "$pr_url" "$pr_title" >> "$SKIPPED_FILE"
      echo "PR #${pr_number} was already merged into ${BASE_BRANCH}."
    fi
  else
    echo "PR #${pr_number} did not merge cleanly; deferring to OpenCode."
    git merge --abort >/dev/null 2>&1 || true
    printf '%s\tmerge_conflict\t%s\t%s\n' "$pr_number" "$pr_url" "$pr_title" >> "$CLEAN_CONFLICTS_FILE"
  fi
done < "$SAFE_CANDIDATES_FILE"

if [[ -s "$CLEAN_MERGED_FILE" && -n "$SAFE_MERGE_VERIFY_COMMAND" ]]; then
  echo
  echo "Verifying clean merges with: ${SAFE_MERGE_VERIFY_COMMAND}"
  if ! bash -lc "$SAFE_MERGE_VERIFY_COMMAND"; then
    verification_failed=1
    echo "Verification failed after clean merges; OpenCode will be asked to repair before pushing." >&2
  fi
fi

cat "$DEFERRED_FILE" "$CLEAN_CONFLICTS_FILE" > "$REMAINING_FILE"

if [[ -s "$CLEAN_MERGED_FILE" && "$verification_failed" == "0" && "$PUSH_SAFE_MERGES" == "1" && "$QUALITY_GATE_ALL_PRS" != "1" ]]; then
  echo
  echo "Pushing clean merges to origin/${BASE_BRANCH}..."
  push_base_branch_with_retry
fi

if [[ ! -s "$REMAINING_FILE" && "$verification_failed" == "0" && "$QUALITY_GATE_ALL_PRS" != "1" ]]; then
  echo
  echo "Merge pass complete without OpenCode conflict handling."
  exit 0
fi

if [[ "$STEWARD_HANDLE_REMAINING_PRS" != "1" ]]; then
  echo "Remaining PRs require conflict/overlap handling, but STEWARD_HANDLE_REMAINING_PRS=0." >&2
  sed 's/^/  /' "$REMAINING_FILE" >&2
  exit 6
fi

PROMPT_FILE="$TMPDIR/steward-merge-prompt.md"
MERGE_STEWARD_SKILL="/root/.codex/skills/ironsmith-pr-merge-steward/SKILL.md"
if [[ ! -f "$MERGE_STEWARD_SKILL" ]]; then
  MERGE_STEWARD_SKILL="$HOME/.codex/skills/ironsmith-pr-merge-steward/SKILL.md"
fi
{
  printf 'Read %s first and follow it.\n\n' "$MERGE_STEWARD_SKILL"
  printf 'You are running inside %s. Act as quality gatekeeper and fixer for automated Ironsmith worker PRs, then merge the acceptable result into `%s`.\n\n' "$ROOT" "$BASE_BRANCH"
  printf 'Important constraints:\n'
  printf -- '- Do not launch AWS workers, run the dev loop, sync the card status DB, or bake an AMI. The caller will do post-merge refresh after you finish.\n'
  printf -- '- Safe non-overlapping PRs may already be merged locally in this worktree, but they are not quality-approved yet. Review their full diff before pushing.\n'
  printf -- '- You may fix, consolidate, or revert worker changes that are mechanically wrong even if they compile or pass semantic_compare.\n'
  printf -- '- For the PRs below, resolve textual conflicts and consolidate overlapping or duplicate implementations into one reusable implementation.\n'
  printf -- '- Prefer generalized parser/lowering/runtime support over card-specific code.\n'
  printf -- '- Treat worker keyword aliases with suspicion: do not accept a new keyword by routing it through an existing keyword action unless official rules make the mechanics identical, compiled text still renders the original keyword name, and all runtime state such as "cost was paid" labels remains correct.\n'
  printf -- '- Do not use semantic_compare normalization to hide a mechanically wrong parser/runtime implementation; semantic normalization may only tolerate surface wording after parser, lowering, runtime behavior, and rendered keyword identity are correct.\n'
  printf -- '- Treat low compile_oracle_text similarity as evidence even when semantic_mismatch=false. Reject, repair, or leave open PRs whose compiled text drops quantities, conditions, durations, object/player identity, paid-cost labels, zone restrictions, or whole clauses.\n'
  printf -- '- For every worker PR, inspect the intended card oracle text and the implementation diff. Verify at least parser/lowering output for that card, plus runtime behavior when the PR touches costs, zones, keyword actions, triggers, replacement effects, or "was paid" state.\n'
  printf -- '- If you reject, revert, or leave unmerged a worker PR because the implementation is mechanically wrong, clear `pr_created` for that card in `reports/engine-status.sqlite3` so a later worker can retry it. Keep `pr_created=1` for report-only PRs that intentionally document a runtime gap.\n'
  printf -- '- Run focused verification for the touched code and affected cards.\n'
  printf -- '- Commit the final coherent merge result on `%s` and push `%s` to origin.\n' "$BASE_BRANCH" "$BASE_BRANCH"
  printf -- '- If a PR is superseded by another merged implementation, leave a clear note in your final output.\n\n'

  if [[ -s "$CLEAN_MERGED_FILE" ]]; then
    printf 'Clean PRs already merged locally%s and requiring quality-gate review:\n' "$([[ "$verification_failed" == "1" ]] && printf ' but not pushed because verification failed' || true)"
    sed 's/^/- /' "$CLEAN_MERGED_FILE"
    printf '\n'
  fi

  if [[ "$verification_failed" == "1" ]]; then
    printf 'The clean-merge verification command failed: `%s`.\n' "$SAFE_MERGE_VERIFY_COMMAND"
    printf 'Inspect and repair the current local merge result before pushing.\n\n'
  fi

  printf 'Remaining PRs needing OpenCode merge stewardship:\n'
  if [[ -s "$REMAINING_FILE" ]]; then
    sed 's/^/- /' "$REMAINING_FILE"
  else
    printf -- '- No additional PRs; quality-review and, if needed, repair the already-merged local set before pushing.\n'
  fi
} > "$PROMPT_FILE"

echo
echo "Handing remaining merge work to OpenCode..."
steward_args=(run --dir "$ROOT" --dangerously-skip-permissions)
if [[ -n "$STEWARD_MODEL" ]]; then
  steward_args+=(--model "$STEWARD_MODEL")
fi
if [[ -n "$STEWARD_VARIANT" ]]; then
  steward_args+=(--variant "$STEWARD_VARIANT")
fi

if [[ -n "$STEWARD_OUTPUT_FILE" ]]; then
  "$STEWARD_COMMAND" "${steward_args[@]}" "$(< "$PROMPT_FILE")" | tee "$STEWARD_OUTPUT_FILE"
else
  "$STEWARD_COMMAND" "${steward_args[@]}" "$(< "$PROMPT_FILE")"
fi

if [[ "$(git branch --show-current)" != "$BASE_BRANCH" ]]; then
  echo "OpenCode returned on a different branch; checking out ${BASE_BRANCH}."
  git checkout "$BASE_BRANCH"
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "OpenCode returned with tracked working tree changes still present; refusing to continue." >&2
  git status --short >&2
  exit 7
fi

git fetch origin "$BASE_BRANCH"
local_head="$(git rev-parse "$BASE_BRANCH")"
remote_head="$(git rev-parse "origin/${BASE_BRANCH}")"
if [[ "$local_head" == "$remote_head" ]]; then
  :
elif git merge-base --is-ancestor "$local_head" "$remote_head"; then
  echo "Fast-forwarding local ${BASE_BRANCH} to OpenCode-pushed origin/${BASE_BRANCH}."
  git merge --ff-only "origin/${BASE_BRANCH}"
elif git merge-base --is-ancestor "$remote_head" "$local_head"; then
  echo "Pushing OpenCode merge result to origin/${BASE_BRANCH}..."
  push_base_branch_with_retry
  git fetch origin "$BASE_BRANCH"
else
  echo "Local ${BASE_BRANCH} and origin/${BASE_BRANCH} diverged after OpenCode merge handling." >&2
  exit 8
fi

echo
echo "Merge pass complete."
