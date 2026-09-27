import { compactMatchDisputeEvidence } from "../hooks/peer-lobby/match-lifecycle.js";

// Keep this summary flat: exportDiagnostics compacts deeply nested metadata.
// Never copy the evidence/claim wholesale. Requests and signed fork evidence
// can include private card identities, openings, or encrypted payloads.
export function matchDisputeDiagnostics(dispute) {
  if (!dispute || typeof dispute !== "object") return null;
  const { accusedPlayers, ...evidence } = compactMatchDisputeEvidence(dispute.evidence);
  const accused = Array.isArray(dispute.accusedPlayers) && dispute.accusedPlayers.length
    ? dispute.accusedPlayers : accusedPlayers;
  return {
    reason: typeof dispute.reason === "string" ? dispute.reason : "",
    at: Number.isFinite(dispute.at) ? dispute.at : null,
    ...evidence,
    // A scalar survives export compaction even inside extra.multiplayer.
    accusedPlayerIndices: accused.filter((index) => Number.isInteger(index) && index >= 0).join(","),
  };
}
