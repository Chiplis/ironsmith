export function isMatchDisputed(session) {
  return session?.mode === "disputed" || Boolean(session?.matchDisputed);
}

export function assertMatchNotDisputed(session, label = "Action") {
  if (!isMatchDisputed(session)) return;
  const reason = String(session?.matchDisputed?.reason || "Match transcript is disputed");
  const error = new Error(`${label} rejected: match disputed. ${reason}`);
  error.code = "MATCH_DISPUTED";
  throw error;
}

// Diagnostic exports are shareable. Keep timing and transcript coordinates,
// never the request, command, openings, private proofs, or other card payloads.
export function compactMatchDisputeEvidence(evidence = {}) {
  const claim = evidence?.claim || {};
  const dispute = evidence?.dispute || {};
  const finite = (value) => value != null && Number.isFinite(Number(value)) ? Number(value) : null;
  const accused = evidence?.accusedPlayers || dispute?.accusedPlayers || [];
  return {
    type: String(evidence?.type || dispute?.type || "transcript_fork"),
    sequence: finite(evidence?.sequence ?? dispute?.sequence ?? claim?.basisSequence),
    accusedPlayers: (Array.isArray(accused) ? accused : [])
      .map(finite).filter((value) => value != null),
    requestType: String(claim?.requestType || ""),
    requestId: String(claim?.requestId || ""),
    basisSequence: finite(claim?.basisSequence),
    targetPlayerIndex: finite(claim?.targetPlayerIndex),
    responseTimeoutMs: finite(claim?.responseTimeoutMs),
    requestedAtMs: finite(claim?.requestedAtMs),
    eligibleAtMs: finite(claim?.eligibleAtMs),
    claimedAtMs: finite(claim?.claimedAtMs),
  };
}
