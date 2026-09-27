import test from "node:test";
import assert from "node:assert/strict";
import { exportDiagnostics, resetDiagnostics } from "../src/lib/action-diagnostics.js";
import { setJournalPolicy } from "../src/lib/engine-journal.js";
import { matchDisputeDiagnostics } from "../src/lib/match-dispute-diagnostics.js";

test("redacted diagnostics retain the initiating timeout reason and evidence without private payloads", () => {
  resetDiagnostics();
  setJournalPolicy("redacted");
  try {
    const dispute = {
      reason: "Protocol response timeout from Alice; two-player matches require external arbitration.",
      at: 1700000006000,
      accusedPlayers: [],
      evidence: {
        type: "protocol_response_timeout",
        accusedPlayers: [0],
        claim: {
          basisSequence: 516,
          targetPlayerIndex: 0,
          targetPeerId: "peer-alice",
          targetName: "Alice",
          requesterIndex: 1,
          requestType: "action_intent_progress",
          requestId: "action-progress-123",
          responseTimeoutMs: 4000,
          requestedAtMs: 1700000000000,
          eligibleAtMs: 1700000004000,
          requestPayload: { privateHand: ["SECRET_CARD"], opening: "SECRET_OPENING" },
          signature: "SECRET_SIGNATURE",
        },
        command: { privateCard: "SECRET_COMMAND" },
      },
    };
    const report = exportDiagnostics({
      multiplayer: {
        mode: "disputed",
        matchStarted: false,
        lastAppliedSequence: 516,
        matchDisputed: matchDisputeDiagnostics(dispute),
      },
    });
    const saved = JSON.parse(JSON.stringify(report));
    assert.equal(saved.journal.policy, "redacted");
    assert.equal(saved.extra.multiplayer.matchStarted, false);
    assert.deepEqual(saved.extra.multiplayer.matchDisputed, {
      reason: dispute.reason,
      at: dispute.at,
      type: "protocol_response_timeout",
      sequence: 516,
      basisSequence: 516,
      accusedPlayerIndices: "0",
      targetPlayerIndex: 0,
      requestType: "action_intent_progress",
      requestId: "action-progress-123",
      responseTimeoutMs: 4000,
      requestedAtMs: 1700000000000,
      eligibleAtMs: 1700000004000,
      claimedAtMs: null,
    });
    assert.doesNotMatch(JSON.stringify(saved), /SECRET_|requestPayload|signature|privateHand/);
  } finally {
    setJournalPolicy("full");
    resetDiagnostics();
  }
});

test("fork dispute diagnostics retain sequence and accused seats without signed actions", () => {
  const summary = matchDisputeDiagnostics({
    reason: "Conflicting signed actions",
    at: 1234,
    evidence: {
      dispute: {
        type: "action_fork",
        sequence: 517,
        accusedPlayers: [0, 1],
        existingAction: { command: { privateCard: "SECRET_CARD" } },
        conflictingAction: { signature: "SECRET_SIGNATURE" },
      },
    },
  });
  assert.deepEqual(summary, {
    reason: "Conflicting signed actions",
    at: 1234,
    type: "action_fork",
    sequence: 517,
    basisSequence: null,
    accusedPlayerIndices: "0,1",
    requestType: "",
    requestId: "",
    targetPlayerIndex: null,
    responseTimeoutMs: null,
    requestedAtMs: null,
    eligibleAtMs: null,
    claimedAtMs: null,
  });
  assert.equal(matchDisputeDiagnostics(null), null);
});
