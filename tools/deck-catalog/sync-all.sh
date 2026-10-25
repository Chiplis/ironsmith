#!/usr/bin/env bash
# Refresh the local deck catalog for every supported format. The catalog is
# gitignored, so this is what a checkout runs to obtain decks in the first place
# and what a deployment runs to pick up newer ones.
set -euo pipefail

TOOL_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
FORMATS=("$@")
if [[ ${#FORMATS[@]} -eq 0 ]]; then FORMATS=(commander standard vintage legacy pauper pioneer modern paupercommander); fi

status=0
for format in "${FORMATS[@]}"; do
  echo "Synchronizing ${format}..."
  if node "${TOOL_DIR}/ensure-catalog.mjs" "$format"; then
    node "${TOOL_DIR}/enrich.mjs" --format "$format" --offline \
      --output "${DECK_CATALOG_OUTPUT:-${TOOL_DIR}/../../catalog}" || status=1
  else
    status=1
  fi
done

# Make downloaded decks available to an already-running frontend as well.
if [[ -z "${DECK_CATALOG_OUTPUT:-}" ]]; then
  node "${TOOL_DIR}/../../web/ui/scripts/sync-deck-catalog-assets.mjs"
fi
exit "$status"
