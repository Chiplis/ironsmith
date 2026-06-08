#!/usr/bin/env bash

ironsmith_sha256_file() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
  else
    shasum -a 256 "$path" | awk '{print $1}'
  fi
}

ironsmith_future_utc() {
  local hours="$1"
  python3 - "$hours" <<'PY'
import datetime as dt
import sys

hours = float(sys.argv[1])
expires_at = dt.datetime.now(dt.timezone.utc) + dt.timedelta(hours=hours)
print(expires_at.replace(microsecond=0).isoformat().replace("+00:00", "Z"))
PY
}

ironsmith_aws() {
  if declare -p AWS >/dev/null 2>&1; then
    "${AWS[@]}" "$@"
  elif [[ -n "${AWS_PROFILE:-}" ]]; then
    aws --profile "$AWS_PROFILE" --region "${AWS_REGION:-us-east-2}" "$@"
  else
    aws --region "${AWS_REGION:-us-east-2}" "$@"
  fi
}

ironsmith_ensure_bucket_lifecycle() {
  local bucket="$1"
  local tmpdir="$2"

  if [[ "${S3_LIFECYCLE_ENABLED:-1}" != "1" ]]; then
    return
  fi

  local sessions_days="${S3_SESSION_EXPIRATION_DAYS:-14}"
  local ami_bakes_days="${S3_AMI_BAKE_EXPIRATION_DAYS:-7}"
  local controller_bootstrap_days="${S3_CONTROLLER_BOOTSTRAP_EXPIRATION_DAYS:-7}"
  local controller_runs_days="${S3_CONTROLLER_RUNS_EXPIRATION_DAYS:-30}"
  local script_updates_days="${S3_SCRIPT_UPDATES_EXPIRATION_DAYS:-30}"
  local shared_cards_days="${S3_SHARED_CARDS_EXPIRATION_DAYS:-30}"
  local lifecycle_file="${tmpdir}/ironsmith-bucket-lifecycle.json"

  python3 - "$lifecycle_file" \
    "$sessions_days" \
    "$ami_bakes_days" \
    "$controller_bootstrap_days" \
    "$controller_runs_days" \
    "$script_updates_days" \
    "$shared_cards_days" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
sessions, ami_bakes, controller_bootstrap, controller_runs, script_updates, shared_cards = [
    int(value)
    for value in sys.argv[2:]
]

rules = [
    ("ironsmith-expire-sessions", "sessions/", sessions),
    ("ironsmith-expire-ami-bakes", "ami-bakes/", ami_bakes),
    ("ironsmith-expire-controller-bootstrap", "controller-bootstrap/", controller_bootstrap),
    ("ironsmith-expire-controller-runs", "controller-runs/", controller_runs),
    ("ironsmith-expire-controller-script-updates", "controller-script-updates/", script_updates),
    ("ironsmith-expire-shared-cards", "shared/cards/", shared_cards),
]

path.write_text(
    json.dumps(
        {
            "Rules": [
                {
                    "ID": rule_id,
                    "Status": "Enabled",
                    "Filter": {"Prefix": prefix},
                    "Expiration": {"Days": days},
                    "AbortIncompleteMultipartUpload": {"DaysAfterInitiation": 1},
                }
                for rule_id, prefix, days in rules
            ]
        },
        indent=2,
        sort_keys=True,
    )
    + "\n",
    encoding="utf-8",
)
PY

  if ! ironsmith_aws s3api put-bucket-lifecycle-configuration \
    --bucket "$bucket" \
    --lifecycle-configuration "file://${lifecycle_file}" >/dev/null; then
    echo "Warning: could not apply S3 lifecycle policy for ${bucket}; continuing." >&2
  fi
}

ironsmith_publish_cards_asset() {
  local bucket="$1"
  local cards_path="$2"
  local tmpdir="$3"

  local cards_sha
  local gz_path
  local key

  cards_sha="$(ironsmith_sha256_file "$cards_path")"
  key="shared/cards/${cards_sha}.json.gz"
  gz_path="${tmpdir}/cards-${cards_sha}.json.gz"

  if ! ironsmith_aws s3api head-object --bucket "$bucket" --key "$key" >/dev/null 2>&1; then
    gzip -c "$cards_path" > "$gz_path"
    ironsmith_aws s3api put-object \
      --bucket "$bucket" \
      --key "$key" \
      --body "$gz_path" \
      --content-type application/json \
      --content-encoding gzip \
      --tagging "IronsmithRetention=shared-cards" >/dev/null
  fi

  CARDS_S3_URI="s3://${bucket}/${key}"
  CARDS_S3_COMPRESSION="gzip"
  export CARDS_S3_URI CARDS_S3_COMPRESSION
}
