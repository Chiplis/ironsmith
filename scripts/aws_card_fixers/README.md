# AWS Ironsmith Card Fixer Workers

This launches one EC2 worker per random parse-failing card from
`reports/engine-status.sqlite3` where `agent_running` and `pr_created` are not
set. Card selection is reserved transactionally in SQLite before instances are
launched, so concurrent launchers do not pick the same card from the same local
DB. Workers can use a baked AMI with Rust/Cargo, Node, OpenCode, a warm
`Chiplis/ironsmith` checkout, and the card compilation tool already built. Each
worker resets that checkout to the current base branch, downloads the local
`cards.json` snapshot from S3, creates or reuses the card's draft PR on a stable
per-card branch, then runs the Ironsmith card-fixer fleet and card-fixer skill
prompts through `opencode run`.

The launcher does not embed local GitHub or OpenCode credentials into user data.
Workers read credentials from SSM SecureString parameters through their instance
role.

## One-Time Setup

Authenticate AWS first:

```bash
aws sso login --profile ironsmith-843750990226
```

Create a GitHub token parameter. The token needs `repo` scope for branch pushes
and PR creation:

```bash
GITHUB_TOKEN="$(gh auth token)"
aws --profile ironsmith-843750990226 --region us-east-2 ssm put-parameter \
  --name /ironsmith/github-token \
  --type SecureString \
  --overwrite \
  --value "$GITHUB_TOKEN"
unset GITHUB_TOKEN
```

Create an OpenCode OAuth auth parameter from the local subscription login. If
`opencode run` reports `Token refresh failed: 401`, refresh the local login first:

```bash
opencode auth login -p openai -m "ChatGPT Pro/Plus (headless)"

aws --profile ironsmith-843750990226 --region us-east-2 ssm put-parameter \
  --name /ironsmith/opencode-auth-json \
  --type SecureString \
  --overwrite \
  --value "$(cat ~/.local/share/opencode/auth.json)"
```

## Bake Worker AMI

After a set of worker PRs has been merged, refresh the local status DB and bake a
new worker AMI from the merged `main`:

```bash
AWS_PROFILE=ironsmith-843750990226 \
AWS_REGION=us-east-2 \
scripts/aws_card_fixers/refresh_after_merge.sh
```

That script runs:

```bash
cargo run --release -p ironsmith-tools --bin sync_card_status_db -- --db-path reports/engine-status.sqlite3
```

It also bakes an AMI and publishes the AMI id to
`/ironsmith/card-fixer-worker-ami-arm64` in SSM Parameter Store for the default
ARM worker fleet. The DB sync and AMI bake run serially by default to avoid
controller memory pressure; set
`PARALLEL_SYNC_AND_BAKE=1` on a larger machine if you want to run them in
parallel.

The baked image includes system packages, Rust/Cargo, OpenCode, a warm
`/opt/ironsmith` checkout, prebuilt debug `compile_oracle_text`, and release
helper binaries by default. Workers use a small `compile_oracle_text_worker`
wrapper that runs the baked debug binary directly when sources are unchanged,
and performs an incremental rebuild only after the worker edits relevant
Rust/Cargo files or the AMI base is stale. By default, the baker uses the
previous worker AMI from `/ironsmith/card-fixer-worker-ami-arm64` as its source
image, so repeated bakes can reuse installed packages, Rust, OpenCode, the git
checkout, and Cargo build artifacts. If the previous AMI root volume is larger
than `AMI_VOLUME_SIZE`, the baker falls back to Amazon Linux once so the next
snapshot can shrink. Set `SOURCE_AMI_ID=ami-...` to force a specific source AMI;
when no previous worker AMI is available, the baker falls back to the latest
Amazon Linux 2023 AMI.

After publishing a new worker AMI, the baker deregisters stale tagged worker
AMIs and deletes their snapshots by default, while retaining the current SSM
AMI(s) and the newest `RETAIN_WORKER_AMIS_PER_ARCH=1` image per architecture.
Set `CLEANUP_OLD_WORKER_AMIS=0` only when you intentionally want to retain older
images for rollback or debugging.

The ARM AMI builder tries cheap Spot `t4g.large` first, then `c7g.xlarge` and
`c6g.xlarge`. Override the ordered fallback list with
`AMI_BUILDER_INSTANCE_TYPES="t4g.large c7g.xlarge"`, or set the legacy
`AMI_BUILDER_INSTANCE_TYPE=t4g.large` for a single builder type. Builder
instances are TTL-tagged with `AMI_BUILDER_TTL_HOURS=4`. Because `cards.json` is
local-only, the bake uploads a gzip-compressed, content-addressed shared
`CARDS_PATH` asset to S3 and downloads it into `/opt/ironsmith/cards.json`
before compiling.

Release `compile_oracle_text` and release `sync_card_status_db` are built by
default. Set `AMI_BUILD_RELEASE_TOOLS=0` only when you intentionally want a
shorter bake and do not need those release helpers warmed.

To bake only the AMI:

```bash
AWS_PROFILE=ironsmith-843750990226 \
AWS_REGION=us-east-2 \
scripts/aws_card_fixers/bake_worker_ami.sh
```

## Launch

```bash
AWS_PROFILE=ironsmith-843750990226 \
AWS_REGION=us-east-2 \
INSTANCE_COUNT=5 \
WORKER_ARCH=arm64 \
INSTANCE_TYPE=t4g.medium \
INSTANCE_TYPES="t4g.medium t4g.large c8g.large c7g.large c6g.large m7g.large m6g.large r7g.large r6g.large" \
GITHUB_TOKEN_SSM_PARAM=/ironsmith/github-token \
OPENCODE_AUTH_JSON_SSM_PARAM=/ironsmith/opencode-auth-json \
OPENCODE_MODEL=openai/gpt-5.5-fast \
OPENCODE_STALE_TIMEOUT_SECONDS=1800 \
OPENCODE_HEARTBEAT_SECONDS=60 \
OPENCODE_NO_COMMIT_RETRIES=1 \
POST_PR_STEWARD_MAX_REPAIRS=3 \
WORKER_ENTRY_SKILL=ironsmith-aws-card-fixer-fleet \
WORKER_AMI_SSM_PARAM=/ironsmith/card-fixer-worker-ami-arm64 \
REQUIRE_WORKER_AMI=1 \
USE_SPOT=1 \
USE_EC2_FLEET=1 \
INSTANCE_TTL_HOURS=6 \
SELF_TERMINATE=1 \
scripts/aws_card_fixers/launch_fleet.sh
```

If `/ironsmith/card-fixer-worker-ami-arm64` exists, the launcher uses it
automatically. `REQUIRE_WORKER_AMI=1` is the default so a missing AMI fails fast
instead of spending worker time on package installs and builds. Set
`WORKER_AMI_ID=ami-...` to override the SSM parameter for a single launch.

`t4g.medium` is the default 4 GiB worker type. Worker launches use EC2 Fleet
`price-capacity-optimized` Spot allocation by default across multiple ARM
families and Availability Zones, with the old ordered `run-instances` fallback
kept for troubleshooting (`USE_EC2_FLEET=0`). Burstable workers use
`BURSTABLE_CPU_CREDITS=standard` to avoid surprise CPU-credit charges.

Check the quota request:

```bash
aws --profile ironsmith-843750990226 --region us-east-2 \
  service-quotas get-requested-service-quota-change \
  --request-id ecdd03c58227433f91573e4fa37482bbjVt2mPxA
```

## Monitor

The launcher prints the S3 asset prefix. Workers publish two observability
objects under that prefix:

- `status/INSTANCE_ID.json`: latest state, step, message, branch, card, and PR URL.
- `events/INSTANCE_ID.jsonl`: checkpoint history for that worker.
- `artifacts/INSTANCE_ID/`: failure artifacts such as the OpenCode JSON event log,
  prompt, worker log, and git status/diff snapshots.
- `ironsmith-skills-manifest.txt`: the skill directories bundled into the worker.

Poll the whole fleet:

```bash
AWS_PROFILE=ironsmith-843750990226 \
AWS_REGION=us-east-2 \
S3_ASSET_PREFIX=s3://BUCKET/SESSION_ID \
WATCH_INTERVAL=30 \
scripts/aws_card_fixers/monitor_fleet.sh
```

By default the monitor also marks local DB rows as `pr_created = 1` for any
worker status that reaches `complete` with a PR URL, clears `agent_running` for
terminal workers, requests termination for complete/failed instances, and
terminates any session instances whose `IronsmithExpiresAt` tag is past due.
Set `SYNC_PR_CREATED=0`, `TERMINATE_TERMINAL_INSTANCES=0`,
`CLEANUP_EXPIRED_INSTANCES=0`, or `CLEANUP_EXPIRED_PROJECT_INSTANCES=0` to
disable those behaviors. The global project cleanup terminates expired workers,
AMI builders, and controllers that carry an `IronsmithExpiresAt` tag.

Workers also run with EC2 `instance-initiated-shutdown-behavior=terminate` and
shut themselves down after success or failure when `SELF_TERMINATE=1`.
During `opencode_run`, workers publish heartbeat status every
`OPENCODE_HEARTBEAT_SECONDS`. If the OpenCode JSON event log does not grow for
`OPENCODE_STALE_TIMEOUT_SECONDS`, the worker preserves artifacts, reports
`failed/no_progress`, and exits with the early draft PR left open for inspection.
If OpenCode exits successfully but leaves no working-tree changes, the worker
tries one stricter no-diff retry by default. If that retry still leaves no
working-tree changes, the worker reports `failed/no_commit`. Set
`OPENCODE_NO_COMMIT_RETRIES=0` to avoid spending model time on that retry.

Workers poll the EC2 Spot interruption metadata endpoint during OpenCode runs.
When a Spot interruption notice arrives, the worker stops OpenCode, commits and
pushes the current tree to the card PR branch when possible, comments on the PR
with the interruption and artifact path, uploads logs/diffs under
`artifacts/INSTANCE_ID/`, and reports `failed/spot_interruption_notice`. Because
that terminal state does not mark `pr_created = 1`, the rolling controller can
retry the same card. The replacement worker reuses the existing stable branch
and draft PR, then comments that work resumed from the new EC2 instance.

After creating a draft PR, the same worker runs a second OpenCode session with
the `ironsmith-pr-merge-steward` quality-gate prompt scoped to that single PR.
If that review rejects the PR, the worker starts a fresh OpenCode repair session
with the steward feedback, pushes a repair commit to the same branch, and repeats
review/repair until the PR is approved or `POST_PR_STEWARD_MAX_REPAIRS` is
exhausted. Exhausted PRs are left open with a comment and the worker reports
failure so the monitor does not mark `pr_created`.

Before committing, the worker also runs a local dirty-worktree steward review.
If the initial pass, no-diff retry, or pre-PR review leaves no committable diff,
the worker runs up to `PRE_PR_STEWARD_MAX_REPAIRS` fresh repair pass(es) using
the prior OpenCode conversation and steward feedback before reporting
`failed/no_commit`.

Expected steps are:

```text
install_dependencies -> install_rust -> install_opencode -> opencode_auth
-> github_auth -> clone_repo -> download_assets -> create_branch -> create_pr
-> prebuild_tools -> opencode_run -> local_steward_review
-> commit_changes -> push_branch -> post_pr_steward_review
-> complete
```

On a baked AMI, the install steps are quick verification/skips, `clone_repo`
resets the warm `/opt/ironsmith` checkout to `origin/main`, and `prebuild_tools`
uses the baked `compile_oracle_text` when its recorded base commit still matches
the worker base. If the base changed or the binary is missing, the worker
rebuilds it once before handing the card to OpenCode.

The worker prompt starts from `/root/.codex/skills/ironsmith-aws-card-fixer-fleet/SKILL.md`,
then delegates the actual single-card implementation to
`/root/.codex/skills/ironsmith-card-fixer/SKILL.md`. The launcher validates the
required worker skill directories before any instance is started, then uploads
all local `ironsmith-*` skill directories into the worker bundle.

For a detailed per-instance log, connect with SSM:

```bash
aws --profile ironsmith-843750990226 --region us-east-2 ssm start-session \
  --target INSTANCE_ID
sudo tail -f /var/log/ironsmith-card-fixer-worker.log
```

Terminate the session fleet when done using the command printed by the launcher.

## Continuous Loop

The continuous dev loop uses rolling merge-and-refill scheduling. It keeps a
fixed worker window full, sends successful worker PRs into `merge_batch_prs.sh`,
refreshes the local status DB after each merge group, and launches replacement
workers from the refreshed eligible set:

```bash
AWS_PROFILE=ironsmith-843750990226 \
AWS_REGION=us-east-2 \
MAX_ACTIVE_WORKERS=8 \
STOP_ON_FAILED=0 \
GITHUB_TOKEN_SSM_PARAM=/ironsmith/github-token \
OPENCODE_AUTH_JSON_SSM_PARAM=/ironsmith/opencode-auth-json \
WORKER_ENTRY_SKILL=ironsmith-aws-card-fixer-fleet \
WORKER_AMI_SSM_PARAM=/ironsmith/card-fixer-worker-ami-arm64 \
scripts/aws_card_fixers/run_dev_loop.sh
```

`MAX_TOTAL_CARDS=0` means the loop continues until no eligible parse-failing
cards remain. Use `DRY_RUN=1` to preview the next random worker selection without
launching AWS workers. By default, the loop continues after terminal worker
failures; set `STOP_ON_FAILED=1` to restore fail-fast behavior. Legacy
`BATCH_SIZE` is accepted only as the default value for `MAX_ACTIVE_WORKERS`.

The simple full-pipeline wrapper sets those defaults for the current AWS account
and SSM parameter names:

```bash
scripts/aws_card_fixers/run_dev_loop_full_pipeline.sh
```

To hand the full pipeline off to an AWS controller instance instead of keeping a
local SSO session alive, use:

```bash
scripts/aws_card_fixers/run_dev_loop_on_controller.sh
```

The controller wrapper launches or reuses the `ironsmith-card-fixer-controller`
EC2 instance, uploads the local fleet scripts, status DB, and Ironsmith skills,
then starts `run_dev_loop_full_pipeline.sh` detached under `tmux` with
`USE_INSTANCE_PROFILE=1`. Local AWS credentials are only needed for the handoff;
the controller and workers use instance-profile credentials after that.

The controller defaults are sized for merge-steward and AMI-refresh work while
still keeping idle spend bounded: `CONTROLLER_INSTANCE_TYPE=t4g.large`,
`CONTROLLER_VOLUME_SIZE_GB=60`, `CONTROLLER_SWAP_GB=16`,
`CONTROLLER_CARGO_JOBS=1`, `CONTROLLER_TTL_HOURS=120`, and
`CONTROLLER_SELF_TERMINATE=1`. If an existing controller has lost SSM
connectivity or uses a different instance type, the wrapper replaces it by
default (`CONTROLLER_REPLACE_UNHEALTHY=1` and
`CONTROLLER_REPLACE_WRONG_TYPE=1`). Override those values only when you
intentionally want to reuse a smaller or longer-lived controller.

It prints the run directory and unified log path before starting. The current
terminal streams the same output, and a second terminal can follow it with:

```bash
tail -f reports/aws-card-fixer-dev-loop/RUN_ID/dev-loop-RUN_ID.log
```

Every dev-loop run creates one unified log at
`RUN_DIR/dev-loop-DATE.log` and routes launcher, monitor, merge, Codex,
sync, and AMI bake output through it. The monitor rows identify each worker by
`instance_id`, `card_name`, `state`, `step`, `message`, `pr_url`, and timestamp.
For deeper per-worker history, use the printed S3 session prefix: each worker
writes `status/INSTANCE_ID.json`, `events/INSTANCE_ID.jsonl`, and failure
artifacts under `artifacts/INSTANCE_ID/`. The loop bakes and publishes a fresh
ARM worker AMI after every merge group by default (`BAKE_EVERY_MERGED_PRS=1`)
before launching replacement workers. The bake cleans Ironsmith tool/runtime
crates, rebuilds the required helper binaries, updates
`/ironsmith/card-fixer-worker-ami-arm64`, deregisters the previous AMI, and
deletes stale worker AMI snapshots. Set `BAKE_EVERY_MERGED_PRS=0` to disable
rolling bakes, or `BAKE_ON_STOP=1` to bake once before exiting if merges landed
since the previous bake.

The launcher and baker install a bucket lifecycle policy by default. Session
assets under `sessions/` expire after `S3_SESSION_EXPIRATION_DAYS=14`; AMI-bake
bootstrap assets under `ami-bakes/` expire after
`S3_AMI_BAKE_EXPIRATION_DAYS=7`; controller bootstrap assets expire after 7
days; controller logs and script-update bundles expire after 30 days; shared
compressed `cards.json` objects expire after `S3_SHARED_CARDS_EXPIRATION_DAYS=30`.
Set `S3_LIFECYCLE_ENABLED=0` only when you need to manage bucket retention
manually.

Install budget and anomaly guardrails once with:

```bash
AWS_PROFILE=ironsmith-843750990226 \
BUDGET_ALERT_EMAIL=you@example.com \
scripts/aws_card_fixers/ensure_cost_guardrails.sh
```

`merge_batch_prs.sh` first merges the easy PRs locally: open PRs targeting
`main` that do not touch any file touched by another PR in the same batch and
merge without text conflicts. Because each successful worker has already run a
single-PR steward review, the default `QUALITY_GATE_ALL_PRS=0` pushes those
clean merges after `SAFE_MERGE_VERIFY_COMMAND` passes (default
`cargo check --workspace -j 1`). PRs with overlapping files, text conflicts, or
verification failures are still handed to `opencode run` with the
`ironsmith-pr-merge-steward` skill prompt so Codex can resolve conflicts,
consolidate duplicate implementations, repair or reject mechanically wrong
work, and push a coherent `main`. Set `QUALITY_GATE_ALL_PRS=1` to restore the
older controller-side review of every worker PR.
