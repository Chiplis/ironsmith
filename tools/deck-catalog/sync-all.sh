#!/usr/bin/env bash
# Refresh the local deck catalog for every supported format. The catalog is
# gitignored, so this is what a checkout runs to obtain decks in the first place
# and what a deployment runs to pick up newer ones.
set -euo pipefail

TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
FORMATS=("$@")
if [[ ${#FORMATS[@]} -eq 0 ]]; then FORMATS=(modern pioneer standard); fi

for format in "${FORMATS[@]}"; do
  echo "Synchronizing ${format}..."
  node "${TOOL_DIR}/sync.mjs" \
    --format "$format" \
    --page 0 \
    --events 5 \
    --limit 24 \
    --collection-limit 12 \
    --recent-events 20 \
    --major-events 5
done
