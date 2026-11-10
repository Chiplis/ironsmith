#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
node web/ui/scripts/generate-mtg-keyword-rules.mjs
node --test web/ui/tests/official-keyword-collisions.test.js web/ui/tests/keyword-registry-refresh.test.js scripts/custom-mechanics/keyword-collisions.test.mjs
node ~/ironsmith/scripts/sync-home-frontend.mjs &&
(cd ~/home &&
 git add . &&
 git commit -m "update frontend" &&
 git push)
