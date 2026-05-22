#!/usr/bin/env bash
set -euo pipefail

TEST_FILE="tests/peerjs-resync.e2e.test.js"
CANCEL_PATTERN="Promise resolution is still pending but the event loop has already resolved"

escape_regex() {
  node -e 'console.log(process.argv[1].replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))' "$1"
}

run_test() {
  local label="$1"
  local pattern="$2"
  local retries="${3:-0}"
  local attempt=0

  while true; do
    local log_file
    log_file="$(mktemp)"
    printf '\n▶ %s\n' "$label"
    set +e
    node --test --test-name-pattern "$pattern" "$TEST_FILE" 2>&1 | tee "$log_file"
    local status="${PIPESTATUS[0]}"
    set -e

    if [[ "$status" -eq 0 ]]; then
      rm -f "$log_file"
      return 0
    fi

    if grep -q "$CANCEL_PATTERN" "$log_file" && [[ "$attempt" -lt "$retries" ]]; then
      attempt=$((attempt + 1))
      printf '%s was cancelled by the Node test runner; retrying (%s/%s)\n' "$label" "$attempt" "$retries" >&2
      rm -f "$log_file"
      sleep 5
      continue
    fi

    rm -f "$log_file"
    return "$status"
  done
}

run_test "PeerJS harness tests" "^PeerJS " 0
sleep 5

FULL_UI_TESTS=(
  "full UI PeerJS 60-Mountain match lets both players play hidden-deck lands without opening mismatch"
  "full UI PeerJS real WASM game resumes after 15s reconnects and host takeover"
  "full UI PeerJS Mulligan redraw stays synced"
  "full UI PeerJS guest Mulligan redraw stays synced"
  "full UI PeerJS repeated host Mulligans stay synced"
  "full UI PeerJS both players Mulligan then host remulligans stays synced"
  "full UI PeerJS Gemstone Caverns pregame action stays synced"
  "full UI PeerJS Gemstone Caverns after guest mulligans publishes its opening"
  "full UI PeerJS Gemstone Caverns after both players mulligan remaps pregame source"
  "full UI PeerJS casting Demonic Consultation after playing Swamp keeps hidden openings synced"
  "full UI PeerJS guest casting Demonic Consultation after playing Swamp keeps hidden openings synced"
  "full UI PeerJS guest Demonic Consultation missing name exiles the library without desync"
  "full UI PeerJS host Demonic Consultation missing name after mulligan keeps ziffle openings linked"
  "full UI PeerJS Selvala after host mulligans reveals ziffle libraries without desync"
  "full UI PeerJS Mystical Tutor resolves into a searchable hidden library choice"
  "full UI PeerJS Gitaxian Probe shows the targeted player's hand to the caster"
  "full UI PeerJS Tainted Pact resolution reveals choices and stays synced"
)

for test_name in "${FULL_UI_TESTS[@]}"; do
  run_test "$test_name" "^$(escape_regex "$test_name")$" 2
  sleep 5
done
