// View helpers for end-of-match disclosure verdicts (verified multiplayer):
// when the match ends every player opens its remaining hidden hand cards,
// face-down cards, and claimed cards that entered its library; each peer's
// verdicts live in multiplayer.endOfMatchDisclosure (see
// hooks/peer-lobby/end-of-match-disclosure.js). Shared by the desktop
// game-over panel and the mobile game-over dock.

export function endOfMatchDisclosureEntries(multiplayer) {
  return Object.entries(multiplayer?.endOfMatchDisclosure?.byPlayer || {})
    .map(([player, entry]) => ({ player, ...entry }))
    .sort((left, right) => Number(left.player) - Number(right.player));
}

export function endOfMatchDisclosureStatusLabel(entry) {
  switch (entry?.status) {
    case "verified":
      return entry.reason === "sent" ? "disclosed" : "verified";
    case "cheat_detected":
      return `cheat detected${entry.reason ? ` (${entry.reason})` : ""}`;
    case "missing":
      return "disclosure missing";
    default:
      return "awaiting disclosure";
  }
}
