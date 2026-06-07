#!/usr/bin/env bash
set -euo pipefail

AWS_REGION="${AWS_REGION:-$(aws configure get region 2>/dev/null || true)}"
AWS_REGION="${AWS_REGION:-us-east-2}"
RETAIN_WORKER_AMIS_PER_ARCH="${RETAIN_WORKER_AMIS_PER_ARCH:-1}"
DRY_RUN="${DRY_RUN:-0}"
WORKER_AMI_SSM_PARAMS="${WORKER_AMI_SSM_PARAMS:-/ironsmith/card-fixer-worker-ami /ironsmith/card-fixer-worker-ami-arm64}"
EXTRA_PROTECTED_AMI_IDS="${EXTRA_PROTECTED_AMI_IDS:-}"

AWS=(aws)
if [[ -n "${AWS_PROFILE:-}" ]]; then
  AWS+=(--profile "$AWS_PROFILE")
fi
AWS+=(--region "$AWS_REGION")

usage() {
  cat <<EOF
Usage:
  scripts/aws_card_fixers/cleanup_worker_amis.sh

Optional env:
  AWS_PROFILE=
  AWS_REGION=us-east-2
  RETAIN_WORKER_AMIS_PER_ARCH=1
  WORKER_AMI_SSM_PARAMS="/ironsmith/card-fixer-worker-ami /ironsmith/card-fixer-worker-ami-arm64"
  EXTRA_PROTECTED_AMI_IDS="ami-..."
  DRY_RUN=0

Deletes tagged Ironsmith worker AMIs and snapshots except AMIs protected by SSM
parameters, EXTRA_PROTECTED_AMI_IDS, and the newest RETAIN_WORKER_AMIS_PER_ARCH
AMI(s) for each architecture.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

protected=()
for param in $WORKER_AMI_SSM_PARAMS; do
  value="$("${AWS[@]}" ssm get-parameter \
    --name "$param" \
    --query 'Parameter.Value' \
    --output text 2>/dev/null || true)"
  if [[ -n "$value" && "$value" != "None" ]]; then
    protected+=("$value")
  fi
done
for image_id in $EXTRA_PROTECTED_AMI_IDS; do
  protected+=("$image_id")
done

images_json="$TMPDIR/images.json"
delete_tsv="$TMPDIR/delete.tsv"
"${AWS[@]}" ec2 describe-images \
  --owners self \
  --filters "Name=tag:IronsmithWorkerAmi,Values=true" \
  --output json > "$images_json"

python3 - "$images_json" "$delete_tsv" "$RETAIN_WORKER_AMIS_PER_ARCH" "${protected[@]}" <<'PY'
import json
import pathlib
import sys
from collections import defaultdict

images_path = pathlib.Path(sys.argv[1])
delete_path = pathlib.Path(sys.argv[2])
retain_per_arch = int(sys.argv[3])
protected = set(sys.argv[4:])

images = json.loads(images_path.read_text(encoding="utf-8")).get("Images", [])
by_arch = defaultdict(list)
for image in images:
    by_arch[image.get("Architecture", "unknown")].append(image)

keep = set(protected)
for arch_images in by_arch.values():
    arch_images.sort(key=lambda row: row.get("CreationDate", ""), reverse=True)
    for image in arch_images[:retain_per_arch]:
        keep.add(image["ImageId"])

lines = []
for image in sorted(images, key=lambda row: row.get("CreationDate", "")):
    image_id = image["ImageId"]
    if image_id in keep:
        continue
    snapshots = [
        mapping.get("Ebs", {}).get("SnapshotId", "")
        for mapping in image.get("BlockDeviceMappings", [])
        if mapping.get("Ebs", {}).get("SnapshotId")
    ]
    name = image.get("Name", "")
    arch = image.get("Architecture", "")
    created = image.get("CreationDate", "")
    lines.append("\t".join([image_id, ",".join(snapshots), arch, created, name]))

delete_path.write_text("\n".join(lines) + ("\n" if lines else ""), encoding="utf-8")
print(f"Worker AMIs found: {len(images)}")
print(f"Worker AMIs protected: {len(keep)}")
print(f"Worker AMIs selected for deletion: {len(lines)}")
PY

if [[ ! -s "$delete_tsv" ]]; then
  echo "No stale worker AMIs to delete."
  exit 0
fi

while IFS=$'\t' read -r image_id snapshots arch created name; do
  [[ -n "${image_id:-}" ]] || continue
  echo "Deleting stale worker AMI ${image_id} (${arch}, ${created}, ${name})"
  if [[ "$DRY_RUN" == "1" ]]; then
    echo "  DRY_RUN: would deregister ${image_id}"
  else
    "${AWS[@]}" ec2 deregister-image --image-id "$image_id" >/dev/null 2>&1 || true
  fi
  IFS=',' read -r -a snapshot_ids <<< "$snapshots"
  for snapshot_id in "${snapshot_ids[@]}"; do
    [[ -n "$snapshot_id" ]] || continue
    if [[ "$DRY_RUN" == "1" ]]; then
      echo "  DRY_RUN: would delete snapshot ${snapshot_id}"
    else
      "${AWS[@]}" ec2 delete-snapshot --snapshot-id "$snapshot_id" >/dev/null 2>&1 || true
      echo "  Deleted snapshot ${snapshot_id}"
    fi
  done
done < "$delete_tsv"
