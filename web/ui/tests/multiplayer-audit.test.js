import test from "node:test";
import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import {
  auditStateHash,
  actionQuorumThreshold,
  assertCurrentAuditPlayerCount,
  buildActionForkDisputeEvidence,
  buildSignedDisconnectForfeitVote,
  buildSignedMatchGenesis,
  buildSignedActionEnvelope,
  buildSignedActionQuorumVote,
  buildSignedPlayerGenesis,
  buildSignedResyncEnvelope,
  buildDeckSlotOpening,
  buildPrivateDeckManifest,
  canonicalJson,
  CURRENT_AUDIT_PROTOCOL_VERSION,
  decklistHashForCards,
  publicDeckManifest,
  createAuditSessionKey,
  createAuditEncryptionKey,
  encryptPrivateAuditPayload,
  exportAuditEncryptionPublicKey,
  exportAuditPublicKey,
  importAuditPublicKey,
  rngCommitmentPayload,
  rngRevealPayload,
  publicCheckpointHash,
  sha256Hex,
  signAuditPayload,
  verifyCardOpeningAgainstManifest,
  verifyAuditPayload,
  verifyActionQuorumCertificate,
  verifyDisconnectForfeitCertificate,
  verifyLiveAuditTranscript,
  verifyPrivateViewDisclosure,
  verifySignedMatchGenesis,
  verifySignedResyncEnvelope,
} from "../src/lib/multiplayer-audit.js";

function cloneTestPayload(value) {
  return JSON.parse(JSON.stringify(value));
}

async function buildCurrentProtocolTranscript({
  matchId,
  players,
  actions,
  deckAuditManifests = [],
  initialPublicCheckpointHash = "initial-public-checkpoint",
  playerCount = null,
}) {
  const suppliedPlayers = players.map((player, offset) => ({
    ...player,
    index: Number(player.index ?? player.seat ?? offset),
  }));
  const highestSeat = suppliedPlayers.reduce(
    (max, player) => Math.max(max, Number(player.index)),
    -1,
  );
  const targetPlayerCount = Math.max(
    2,
    Number(playerCount || 0),
    highestSeat + 1,
    suppliedPlayers.length,
  );
  assertCurrentAuditPlayerCount(targetPlayerCount, "Test transcript");
  const sourceBySeat = new Map(suppliedPlayers.map((player) => [Number(player.index), player]));
  for (let index = 0; index < targetPlayerCount; index += 1) {
    if (!sourceBySeat.has(index)) {
      sourceBySeat.set(index, { index });
    }
  }
  const privatePlayers = [];
  const normalizedDeckAuditManifests = [];
  for (const index of [...sourceBySeat.keys()].sort((left, right) => left - right)) {
    const player = sourceBySeat.get(index);
    const keyPair = player.keyPair || await createAuditSessionKey(webcrypto);
    const encryptionKeyPair = player.encryptionKeyPair || await createAuditEncryptionKey(webcrypto);
    const auditPublicKey = player.auditPublicKey || await exportAuditPublicKey(keyPair, webcrypto);
    const auditEncryptionPublicKey =
      player.auditEncryptionPublicKey
      || await exportAuditEncryptionPublicKey(encryptionKeyPair, webcrypto);
    const manifest = publicDeckManifest(
      deckAuditManifests[index]
      || player.deckAuditManifest
      || await buildPrivateDeckManifest({
        matchId,
        owner: index,
        deck: [],
        saltForSlot: (slot) => `test-empty-deck-${index}-${slot}`,
      }, webcrypto)
    );
    normalizedDeckAuditManifests[index] = manifest;
    const entry = {
      peerId: String(player.peerId || `peer-${index}`),
      name: String(player.name || `Player ${index + 1}`),
      index,
      auditPublicKey,
      auditEncryptionPublicKey,
      deckAuditManifest: manifest,
      ziffleKey: player.ziffleKey || {
        player: index,
        publicKeyHex: `test-ziffle-key-${index}`,
        ownershipProofHex: `test-ziffle-proof-${index}`,
      },
      deckCount: Number(player.deckCount || manifest?.deckCount || 0),
      sideboardCount: Number(player.sideboardCount || manifest?.sideboardCount || 0),
      commanderCount: Number(player.commanderCount || manifest?.commanderCount || 0),
      keyPair,
    };
    entry.playerGenesisSignature = await buildSignedPlayerGenesis({
      keyPair,
      matchId,
      protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
      timeoutMs: 300000,
      player: entry,
    }, webcrypto);
    privatePlayers.push(entry);
  }
  const normalizedPlayers = privatePlayers.map((player) => {
    const entry = { ...player };
    delete entry.keyPair;
    return entry;
  });
  const ziffleKeys = normalizedPlayers.map((player) => player.ziffleKey);
  const ziffleCeremonies = normalizedPlayers.map((player) => ({
    owner: Number(player.index),
    deckCount: Number(player.deckCount || 0),
    context: matchId,
    keyContext: matchId,
    keys: ziffleKeys,
    steps: normalizedPlayers.map((shuffler) => ({
      shuffler: Number(shuffler.index),
      deckHex: `test-deck-${player.index}-${shuffler.index}`,
      proofHex: `test-proof-${player.index}-${shuffler.index}`,
    })),
    deckHash: `test-ziffle-deck-${player.index}`,
  }));
  const host = privatePlayers[0];
  const match = {
    protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
    auditMatchId: matchId,
    lobbyId: matchId,
    hostPeerId: host.peerId,
    format: "normal",
    startingLife: 20,
    openingHandSize: 7,
    seed: 1,
    timeoutMs: 300000,
    initialPublicCheckpointHash,
    matchClockPolicy: {
      type: "per_player_match_clock_v1",
      initialMs: 300000,
      graceMs: 2000,
    },
    players: normalizedPlayers,
    deckAuditManifests: normalizedDeckAuditManifests,
    ziffleKeys,
    ziffleCeremonies,
  };
  match.genesis = await buildSignedMatchGenesis({
    keyPair: host.keyPair,
    match,
    hostSeat: host.index,
  }, webcrypto);
  const actionsWithQuorum = await Promise.all(actions.map(async (entry) => {
    const action = cloneTestPayload(entry);
    if (!action?.audit || action.audit.quorumCertificate || action.quorumCertificate) {
      return action;
    }
    const threshold = actionQuorumThreshold(privatePlayers.length);
    if (threshold <= 0) return action;
    const quorumVoters = privatePlayers.slice(0, threshold);
    const votes = await Promise.all(quorumVoters.map((player) =>
      buildSignedActionQuorumVote({
        keyPair: player.keyPair,
        action,
        voter: player.index,
      }, webcrypto)
    ));
    action.audit.quorumCertificate = {
      type: "ironsmith-action-quorum-v1",
      matchId: String(action.audit.matchId || ""),
      seq: Number(action.audit.seq || action.seq || 0),
      actor: Number(action.audit.actor ?? action.actorIndex ?? 0),
      prevStateHash: String(action.audit.prevStateHash || ""),
      nextStateHash: String(action.audit.nextStateHash || ""),
      publicCheckpointHash: String(action.audit.publicCheckpointHash || ""),
      actionSignature: String(action.audit.signature || ""),
      threshold,
      voters: votes.map((vote) => Number(vote.voter)),
      votes,
    };
    return action;
  }));
  return {
    kind: "ironsmith-live-browser-audit-v1",
    match,
    matchId,
    lobbyId: matchId,
    protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
    signatureAlgorithm: "ecdsa-p256-sha256",
    genesis: match.genesis,
    initialStateHash: "0".repeat(64),
    initialPublicCheckpointHash,
    actions: actionsWithQuorum,
  };
}

test("canonicalJson sorts object keys recursively", () => {
  assert.equal(
    canonicalJson({ b: 2, a: { d: 4, c: 3 } }),
    "{\"a\":{\"c\":3,\"d\":4},\"b\":2}",
  );
});

test("auditStateHash is stable across key insertion order", async () => {
  const left = await auditStateHash({
    matchId: "m",
    seq: 1,
    prevStateHash: "p",
    command: { type: "pass_priority", actor: 0 },
  }, webcrypto);
  const right = await auditStateHash({
    command: { actor: 0, type: "pass_priority" },
    prevStateHash: "p",
    seq: 1,
    matchId: "m",
  }, webcrypto);
  assert.equal(left, right);
});

test("public checkpoint hashes ignore transient worker metadata", async () => {
  const baseCheckpoint = {
    players: [{ id: 0, handCount: 7 }],
    hiddenZones: [{ owner: 0, zone: "hand", count: 7 }],
  };
  assert.equal(
    await publicCheckpointHash(baseCheckpoint, webcrypto),
    await publicCheckpointHash({
      ...baseCheckpoint,
      __perf: { totalWorkerMs: 12.34 },
      hiddenZones: [
        {
          ...baseCheckpoint.hiddenZones[0],
          __perf: { totalWorkerMs: 56.78 },
        },
      ],
    }, webcrypto),
  );
});

test("signed audit envelopes verify and reject tampering", async () => {
  const keyPair = await createAuditSessionKey(webcrypto);
  const envelope = await buildSignedActionEnvelope({
    keyPair,
    matchId: "m",
    seq: 1,
    actor: 0,
    prevStateHash: "p",
    command: { type: "pass_priority" },
  }, webcrypto);
  const publicKeyHex = await exportAuditPublicKey(keyPair, webcrypto);
  const publicKey = await importAuditPublicKey(publicKeyHex, webcrypto);
  const payload = {
    matchId: envelope.matchId,
    seq: envelope.seq,
    actor: envelope.actor,
    signer: envelope.signer,
    prevStateHash: envelope.prevStateHash,
    command: envelope.command,
    openings: envelope.openings,
    rngReveals: envelope.rngReveals,
    shuffleProofs: envelope.shuffleProofs,
    privateViewProofs: envelope.privateViewProofs,
    nextStateHash: envelope.nextStateHash,
  };

  assert.equal(
    await verifyAuditPayload(publicKey, payload, envelope.signature, webcrypto),
    true,
  );
  assert.equal(
    await verifyAuditPayload(
      publicKey,
      { ...payload, command: { type: "draw_cards", player: 0, count: 1 } },
      envelope.signature,
      webcrypto,
    ),
    false,
  );
});

test("action quorum certificates require 2-of-3 or 3-of-4 votes", async () => {
  assert.equal(actionQuorumThreshold(2), 0);
  assert.equal(actionQuorumThreshold(3), 2);
  assert.equal(actionQuorumThreshold(4), 3);
  assert.throws(() => actionQuorumThreshold(5), /requires 2, 3, or 4 players/);

  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
  ]);
  const players = await Promise.all(keys.map(async (keyPair, index) => ({
    index,
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto),
  })));
  const command = { type: "priority_action", action_index: 0 };
  const audit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId: "m-quorum",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    publicCheckpointHash: "public-checkpoint-after-action",
  }, webcrypto);
  const action = { seq: 1, actorIndex: 0, command, audit };
  const votes = await Promise.all([0, 1, 2].map((voter) =>
    buildSignedActionQuorumVote({
      keyPair: keys[voter],
      action,
      voter,
    }, webcrypto)
  ));
  audit.quorumCertificate = {
    type: "ironsmith-action-quorum-v1",
    matchId: audit.matchId,
    seq: audit.seq,
    actor: audit.actor,
    prevStateHash: audit.prevStateHash,
    nextStateHash: audit.nextStateHash,
    publicCheckpointHash: audit.publicCheckpointHash,
    actionSignature: audit.signature,
    threshold: 3,
    voters: [0, 1, 2],
    votes,
  };

  assert.equal(
    (await verifyActionQuorumCertificate({
      certificate: audit.quorumCertificate,
      action,
      players,
    }, webcrypto)).valid,
    true,
  );
  assert.equal(
    (await verifyLiveAuditTranscript(await buildCurrentProtocolTranscript({
      matchId: "m-quorum",
      players: players.map((player, index) => ({
        index: player.index,
        keyPair: keys[index],
        auditPublicKey: player.auditPublicKey,
      })),
      actions: [action],
    }), webcrypto)).valid,
    true,
  );
  await assert.rejects(
    () => verifyActionQuorumCertificate({
      certificate: {
        ...audit.quorumCertificate,
        votes: votes.slice(0, 2),
      },
      action,
      players,
    }, webcrypto),
    /expected at least 3/,
  );
});

test("three-player live transcripts verify with a 2-of-3 action quorum", async () => {
  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
  ]);
  const players = await Promise.all(keys.map(async (keyPair, index) => ({
    index,
    keyPair,
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto),
  })));
  const command = { type: "priority_action", action_index: 0 };
  const audit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId: "m-three-player-two-of-three",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    publicCheckpointHash: "public-checkpoint-after-three-player-action",
  }, webcrypto);
  const action = { seq: 1, actorIndex: 0, command, audit };
  const votes = await Promise.all([0, 2].map((voter) =>
    buildSignedActionQuorumVote({
      keyPair: keys[voter],
      action,
      voter,
    }, webcrypto)
  ));
  action.audit.quorumCertificate = {
    type: "ironsmith-action-quorum-v1",
    matchId: audit.matchId,
    seq: audit.seq,
    actor: audit.actor,
    prevStateHash: audit.prevStateHash,
    nextStateHash: audit.nextStateHash,
    publicCheckpointHash: audit.publicCheckpointHash,
    actionSignature: audit.signature,
    threshold: 2,
    voters: [0, 2],
    votes,
  };

  assert.equal(
    (await verifyLiveAuditTranscript(await buildCurrentProtocolTranscript({
      matchId: "m-three-player-two-of-three",
      players,
      playerCount: 3,
      actions: [action],
    }), webcrypto)).valid,
    true,
  );
  await assert.rejects(
    () => verifyActionQuorumCertificate({
      certificate: {
        ...action.audit.quorumCertificate,
        votes: votes.slice(0, 1),
      },
      action,
      players,
    }, webcrypto),
    /expected at least 2/,
  );
});

test("disconnect forfeits verify with peer-signed timeout evidence", async () => {
  const matchId = "m-disconnect-forfeit";
  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
  ]);
  const players = await Promise.all(keys.map(async (keyPair, index) => ({
    index,
    keyPair,
    peerId: `peer-${index}`,
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto),
  })));
  const votes = await Promise.all([0, 2].map((voter) =>
    buildSignedDisconnectForfeitVote({
      keyPair: keys[voter],
      matchId,
      basisSequence: 0,
      forfeitedPlayer: 1,
      forfeitedPeerId: "peer-1",
      disconnectTimeoutMs: 60000,
      disconnectedAtMs: 100000,
      eligibleAtMs: 160000,
      signedAtMs: 160500,
      voter,
    }, webcrypto)
  ));
  const disconnectCertificate = {
    type: "ironsmith-disconnect-forfeit-v1",
    matchId,
    basisSequence: 0,
    forfeitedPlayer: 1,
    forfeitedPeerId: "peer-1",
    disconnectTimeoutMs: 60000,
    threshold: 2,
    voters: [0, 2],
    votes,
  };
  const command = {
    type: "forfeit_player",
    player: 1,
    reason: "peer_claimed_disconnect_timeout",
    disconnected_peer_id: "peer-1",
    disconnect_timeout_ms: 60000,
    disconnected_at_ms: 100000,
    auto_forfeit_at_ms: 160000,
    claimed_at_ms: 161000,
    basis_sequence: 0,
    disconnect_certificate: disconnectCertificate,
  };

  assert.equal(
    (await verifyDisconnectForfeitCertificate({
      certificate: disconnectCertificate,
      command: { ...command, matchId },
      players: [players[0], players[2]],
    }, webcrypto)).valid,
    true,
  );
  await assert.rejects(
    () => verifyDisconnectForfeitCertificate({
      certificate: {
        ...disconnectCertificate,
        voters: [0],
        votes: votes.slice(0, 1),
      },
      command: { ...command, matchId },
      players: [players[0], players[2]],
    }, webcrypto),
    /expected at least 2/,
  );

  const audit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId,
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    publicCheckpointHash: "public-checkpoint-after-disconnect-forfeit",
  }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId,
    players,
    playerCount: 3,
    actions: [{ seq: 1, actorIndex: 0, command, audit }],
  });

  assert.equal((await verifyLiveAuditTranscript(transcript, webcrypto)).valid, true);
});

test("live transcript verifier validates action-fork dispute evidence", async () => {
  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
  ]);
  const players = await Promise.all(keys.map(async (keyPair, index) => ({
    index,
    keyPair,
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto),
  })));
  const firstCommand = { type: "priority_action", action_index: 0 };
  const secondCommand = { type: "priority_action", action_index: 1 };
  const firstAudit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId: "m-action-fork",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command: firstCommand,
    publicCheckpointHash: "public-checkpoint-first-branch",
  }, webcrypto);
  const secondAudit = await buildSignedActionEnvelope({
    keyPair: keys[2],
    matchId: "m-action-fork",
    seq: 1,
    actor: 2,
    prevStateHash: "0".repeat(64),
    command: secondCommand,
    publicCheckpointHash: "public-checkpoint-second-branch",
  }, webcrypto);
  const firstAction = { seq: 1, actorIndex: 0, command: firstCommand, audit: firstAudit };
  const secondAction = { seq: 1, actorIndex: 2, command: secondCommand, audit: secondAudit };
  const firstVotes = await Promise.all([0, 1].map((voter) =>
    buildSignedActionQuorumVote({
      keyPair: keys[voter],
      action: firstAction,
      voter,
    }, webcrypto)
  ));
  const secondVotes = await Promise.all([1, 2].map((voter) =>
    buildSignedActionQuorumVote({
      keyPair: keys[voter],
      action: secondAction,
      voter,
    }, webcrypto)
  ));
  firstAction.audit.quorumCertificate = {
    type: "ironsmith-action-quorum-v1",
    matchId: firstAudit.matchId,
    seq: firstAudit.seq,
    actor: firstAudit.actor,
    prevStateHash: firstAudit.prevStateHash,
    nextStateHash: firstAudit.nextStateHash,
    publicCheckpointHash: firstAudit.publicCheckpointHash,
    actionSignature: firstAudit.signature,
    threshold: 2,
    voters: [0, 1],
    votes: firstVotes,
  };
  secondAction.audit.quorumCertificate = {
    type: "ironsmith-action-quorum-v1",
    matchId: secondAudit.matchId,
    seq: secondAudit.seq,
    actor: secondAudit.actor,
    prevStateHash: secondAudit.prevStateHash,
    nextStateHash: secondAudit.nextStateHash,
    publicCheckpointHash: secondAudit.publicCheckpointHash,
    actionSignature: secondAudit.signature,
    threshold: 2,
    voters: [1, 2],
    votes: secondVotes,
  };

  const dispute = buildActionForkDisputeEvidence({
    sequence: 1,
    existingAction: firstAction,
    conflictingAction: secondAction,
  });
  assert.deepEqual(dispute.accusedPlayers, [1]);
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-action-fork",
    players,
    playerCount: 3,
    actions: [firstAction],
  });
  transcript.disputes = [dispute];
  transcript.outcome = {
    status: "disputed",
    accusedPlayers: [1],
  };
  const report = await verifyLiveAuditTranscript(transcript, webcrypto);
  assert.equal(report.valid, true);
  assert.equal(report.outcome.status, "disputed");
  assert.deepEqual(report.outcome.accusedPlayers, [1]);

  await assert.rejects(
    () => verifyLiveAuditTranscript({
      ...transcript,
      outcome: {
        status: "disputed",
        accusedPlayers: [2],
      },
    }, webcrypto),
    /accused players/,
  );
});

test("live transcript verifier binds exported winner to the final public checkpoint", async () => {
  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
  ]);
  const players = await Promise.all(keys.map(async (keyPair, index) => ({
    index,
    keyPair,
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto),
  })));
  const finalPublicCheckpoint = {
    version: 1,
    players: [
      { id: 0, name: "Alice", hasWon: true, hasLost: false, hasLeftGame: false },
      { id: 1, name: "Bob", hasWon: false, hasLost: true, hasLeftGame: false },
    ],
    hiddenZones: [],
  };
  const finalPublicCheckpointHash = await publicCheckpointHash(finalPublicCheckpoint, webcrypto);
  const command = { type: "priority_action", action_index: 0 };
  const audit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId: "m-exported-winner",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    publicCheckpointHash: finalPublicCheckpointHash,
  }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-exported-winner",
    players,
    playerCount: 2,
    actions: [{ seq: 1, actorIndex: 0, command, audit }],
  });
  transcript.finalPublicCheckpoint = finalPublicCheckpoint;
  transcript.outcome = {
    status: "winner",
    winner: 0,
  };

  const report = await verifyLiveAuditTranscript(transcript, webcrypto);
  assert.equal(report.valid, true);
  assert.equal(report.outcome.status, "winner");
  assert.equal(report.outcome.winner, 0);
  await assert.rejects(
    () => verifyLiveAuditTranscript({
      ...transcript,
      outcome: {
        status: "winner",
        winner: 1,
      },
    }, webcrypto),
    /outcome winner/,
  );
});

test("two-player transcripts remain tamper-evident without action quorum", async () => {
  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
  ]);
  const players = await Promise.all(keys.map(async (keyPair, index) => ({
    index,
    keyPair,
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto),
  })));
  const command = { type: "priority_action", action_index: 0 };
  const audit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId: "m-two-player",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    publicCheckpointHash: "public-checkpoint-after-two-player-action",
  }, webcrypto);
  const action = { seq: 1, actorIndex: 0, command, audit };

  assert.equal(
    (await verifyActionQuorumCertificate({
      certificate: null,
      action,
      players,
    }, webcrypto)).threshold,
    0,
  );

  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-two-player",
    players,
    playerCount: 2,
    actions: [action],
  });
  assert.equal(transcript.actions[0].audit.quorumCertificate, undefined);
  assert.equal((await verifyLiveAuditTranscript(transcript, webcrypto)).valid, true);
  await assert.rejects(
    () => verifyLiveAuditTranscript({
      ...transcript,
      actions: [{
        ...transcript.actions[0],
        command: { type: "draw_cards", player: 0, count: 1 },
      }],
    }, webcrypto),
    /command mismatch/,
  );
});

test("live audit transcript verifier checks signed match clock chain", async () => {
  const actorKey = await createAuditSessionKey(webcrypto);
  const actorPublicKey = await exportAuditPublicKey(actorKey, webcrypto);
  const clock = {
    type: "match_clock_v1",
    version: 1,
    matchId: "m-clock",
    seq: 1,
    actor: 0,
    reason: "action",
    policy: {
      type: "per_player_match_clock_v1",
      initialMs: 900000,
      graceMs: 2000,
    },
    activePlayer: 0,
    elapsedMs: 1250,
    remainingMsByPlayer: [898750, 900000],
    previousClockHash: "0".repeat(64),
    basisSequence: 0,
  };
  clock.clockHash = await sha256Hex(canonicalJson({
    domain: "ironsmith-match-clock-audit-v1",
    clock,
  }), webcrypto);
  const command = { type: "priority_action", action_index: 0 };
  const audit = await buildSignedActionEnvelope({
    keyPair: actorKey,
    matchId: "m-clock",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    clock,
    publicCheckpointHash: "public-checkpoint-after-clock",
  }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-clock",
    players: [{ index: 0, keyPair: actorKey, auditPublicKey: actorPublicKey }],
    actions: [{ seq: 1, actorIndex: 0, command, audit }],
  });

  assert.equal((await verifyLiveAuditTranscript(transcript, webcrypto)).valid, true);
  await assert.rejects(
    () => verifyLiveAuditTranscript({
      ...transcript,
      actions: [{
        ...transcript.actions[0],
        audit: {
          ...audit,
          clock: {
            ...audit.clock,
            elapsedMs: 0,
          },
        },
      }],
    }, webcrypto),
    /Match clock audit hash mismatch/,
  );
});

test("private deck manifests commit slots without exposing card names publicly", async () => {
  const manifest = await buildPrivateDeckManifest({
    matchId: "m",
    owner: 1,
    deck: ["Forest", "Lightning Bolt"],
    sideboard: ["Duress"],
    commanders: [],
    saltForSlot: (slot) => `salt-${slot}`,
  }, webcrypto);
  const publicManifest = publicDeckManifest(manifest);

  assert.equal(publicManifest.deckCount, 2);
  assert.equal(publicManifest.sideboardCount, 1);
  assert.equal(JSON.stringify(publicManifest).includes("Lightning Bolt"), false);
  assert.equal(JSON.stringify(publicManifest).includes("salt-1"), false);
  assert.deepEqual(
    await buildDeckSlotOpening({
      manifest,
      slot: 1,
      card: "Lightning Bolt",
    }, webcrypto),
    {
      owner: 1,
      slot: 1,
      card: "Lightning Bolt",
      salt: "salt-1",
      commitment: publicManifest.slotCommitments[1].commitment,
    },
  );
  assert.equal(
    await verifyCardOpeningAgainstManifest({
      manifest: publicManifest,
      slot: 1,
      card: "Lightning Bolt",
      salt: "salt-1",
    }, webcrypto),
    true,
  );
  assert.equal(
    await verifyCardOpeningAgainstManifest({
      manifest: publicManifest,
      slot: 1,
      card: "Counterspell",
      salt: "salt-1",
    }, webcrypto),
    false,
  );
});

test("decklist hash distinguishes stale manifests for duplicate-card decks", async () => {
  const matchId = "same-lobby-id";
  const staleManifest = await buildPrivateDeckManifest({
    matchId,
    owner: 0,
    deck: Array(60).fill("Island"),
    saltForSlot: (slot) => `old-salt-${slot}`,
  }, webcrypto);
  const currentHash = await decklistHashForCards({
    matchId,
    owner: 0,
    deck: Array(60).fill("Mountain"),
  }, webcrypto);

  assert.notEqual(staleManifest.decklistHash, currentHash);
  await assert.rejects(
    () => buildDeckSlotOpening({
      manifest: staleManifest,
      slot: 56,
      card: "Mountain",
    }, webcrypto),
    /Private deck opening does not match slot 56/,
  );
});

test("private-view encrypted disclosure binds signed proof to committed deck slot", async () => {
  const encryptionKey = await createAuditEncryptionKey(webcrypto);
  const recipientPublicKey = await exportAuditEncryptionPublicKey(encryptionKey, webcrypto);
  const manifest = await buildPrivateDeckManifest({
    matchId: "m-private",
    owner: 0,
    deck: ["Ponder"],
    saltForSlot: () => "private-view-salt",
  }, webcrypto);
  const opening = await buildDeckSlotOpening({ manifest, slot: 0 }, webcrypto);
  const payload = {
    type: "private_view_opening",
    matchId: "m-private",
    requirementId: "private_open:0:library:0:1",
    owner: 0,
    viewer: 0,
    zone: "library",
    objectId: 1,
    opening,
  };
  const encryptedOpening = await encryptPrivateAuditPayload({
    recipientPublicKey,
    payload,
  }, webcrypto);
  const proof = {
    type: "encrypted_private_opening",
    requirementId: payload.requirementId,
    owner: 0,
    viewer: 0,
    zone: "library",
    objectId: 1,
    slot: 0,
    commitment: opening.commitment,
    encryptedOpening,
  };
  const verified = await verifyPrivateViewDisclosure({
    proof,
    disclosure: payload,
    manifest: publicDeckManifest(manifest),
  }, webcrypto);
  assert.equal(verified.valid, true);
  await assert.rejects(
    () => verifyPrivateViewDisclosure({
      proof,
      disclosure: {
        ...payload,
        opening: {
          ...payload.opening,
          card: "Black Lotus",
        },
      },
      manifest: publicDeckManifest(manifest),
    }, webcrypto),
    /hash does not match/,
  );
});

test("live audit transcript verifier accepts encrypted private-view summary proofs", async () => {
  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
  ]);
  const players = await Promise.all(keys.map(async (keyPair, index) => ({
    index,
    keyPair,
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto),
  })));
  const privateViewProof = {
    type: "encrypted_private_view",
    requirementId: "private_view:1:0:library:2",
    owner: 1,
    viewer: 0,
    zone: "library",
    count: 2,
    reason: "look_at_top_two",
    openingHashes: [
      "a".repeat(64),
      "b".repeat(64),
    ],
    disclosurePolicy: "postgame_or_dispute",
  };
  privateViewProof.materialHash = await sha256Hex(canonicalJson(privateViewProof), webcrypto);
  const command = { type: "priority_action", action_index: 0 };
  const audit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId: "m-private-view-summary",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    privateViewProofs: [privateViewProof],
    publicCheckpointHash: "public-checkpoint-after-private-view",
  }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-private-view-summary",
    players,
    playerCount: 2,
    actions: [{
      seq: 1,
      actorIndex: 0,
      command,
      audit,
    }],
  });

  assert.equal((await verifyLiveAuditTranscript(transcript, webcrypto)).valid, true);
  const badProof = {
    ...privateViewProof,
    materialHash: "0".repeat(64),
  };
  const badAudit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId: "m-private-view-summary",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    privateViewProofs: [badProof],
    publicCheckpointHash: "public-checkpoint-after-private-view",
  }, webcrypto);
  const badTranscript = await buildCurrentProtocolTranscript({
    matchId: "m-private-view-summary",
    players,
    playerCount: 2,
    actions: [{
      seq: 1,
      actorIndex: 0,
      command,
      audit: badAudit,
    }],
  });
  await assert.rejects(
    () => verifyLiveAuditTranscript(badTranscript, webcrypto),
    /material hash mismatch/,
  );
});

test("live audit transcript verifier requires actor-signed actions", async () => {
  const actorKey = await createAuditSessionKey(webcrypto);
  const actorPublicKey = await exportAuditPublicKey(actorKey, webcrypto);
  const command = { type: "pass_priority" };
  const audit = await buildSignedActionEnvelope({
    keyPair: actorKey,
    matchId: "m",
    seq: 1,
    actor: 1,
    prevStateHash: "0".repeat(64),
    command,
    publicCheckpointHash: "public-checkpoint-after-seq-1",
  }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m",
    players: [
      { index: 1, keyPair: actorKey, auditPublicKey: actorPublicKey },
    ],
    actions: [
      {
        seq: 1,
        actorIndex: 1,
        command,
        label: "Pass",
        audit,
      },
    ],
  });

  const report = await verifyLiveAuditTranscript(transcript, webcrypto);
  assert.equal(report.valid, true);
  assert.equal(report.verifiedActions, 1);
  await assert.rejects(
    () => verifyLiveAuditTranscript({
      ...transcript,
      actions: [{ ...transcript.actions[0], command: { type: "draw_cards" } }],
    }, webcrypto),
    /command mismatch/,
  );
  await assert.rejects(
    () => verifyLiveAuditTranscript({
      ...transcript,
      actions: [
        {
          ...transcript.actions[0],
          audit: { ...audit, signer: 0 },
        },
      ],
    }, webcrypto),
    /not signed by the acting player/,
  );
});

test("live audit transcript verifier checks committed openings", async () => {
  const actorKey = await createAuditSessionKey(webcrypto);
  const actorPublicKey = await exportAuditPublicKey(actorKey, webcrypto);
  const manifest = await buildPrivateDeckManifest({
    matchId: "m-open",
    owner: 0,
    deck: ["Island"],
    saltForSlot: () => "open-salt",
  }, webcrypto);
  const opening = await buildDeckSlotOpening({ manifest, slot: 0 }, webcrypto);
  const command = { type: "draw_cards", player: 0, count: 1 };
  const audit = await buildSignedActionEnvelope({
    keyPair: actorKey,
    matchId: "m-open",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    openings: [opening],
    publicCheckpointHash: "public-checkpoint-after-opening",
  }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-open",
    players: [{ index: 0, keyPair: actorKey, auditPublicKey: actorPublicKey }],
    deckAuditManifests: [publicDeckManifest(manifest)],
    actions: [{ seq: 1, actorIndex: 0, command, audit }],
  });

  assert.equal((await verifyLiveAuditTranscript(transcript, webcrypto)).valid, true);
  const tamperedOpening = { ...opening, card: "Black Lotus" };
  const tamperedAudit = await buildSignedActionEnvelope({
    keyPair: actorKey,
    matchId: "m-open",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    openings: [tamperedOpening],
    publicCheckpointHash: "public-checkpoint-after-opening",
  }, webcrypto);
  const tamperedTranscript = await buildCurrentProtocolTranscript({
    matchId: "m-open",
    players: [{ index: 0, keyPair: actorKey, auditPublicKey: actorPublicKey }],
    deckAuditManifests: [publicDeckManifest(manifest)],
    actions: [{ seq: 1, actorIndex: 0, command, audit: tamperedAudit }],
  });
  await assert.rejects(
    () => verifyLiveAuditTranscript(tamperedTranscript, webcrypto),
    /Opening does not match committed deck slot/,
  );
});

test("live audit transcript verifier rejects incomplete fair-random reveals", async () => {
  const actorKey = await createAuditSessionKey(webcrypto);
  const otherKey = await createAuditSessionKey(webcrypto);
  const actorPublicKey = await exportAuditPublicKey(actorKey, webcrypto);
  const otherPublicKey = await exportAuditPublicKey(otherKey, webcrypto);
  const command = { type: "priority_action", action_index: 0 };
  const audit = await buildSignedActionEnvelope({
    keyPair: actorKey,
    matchId: "m-rng",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    rngReveals: [{
      type: "commit_reveal_random",
      requirementId: "rng-1",
      commits: [{ player: 0, commitmentHex: "commit-0" }],
      reveals: [{ player: 0, nonceHex: "nonce-0", commitmentHex: "commit-0" }],
      combinedSeedHex: "seed",
    }],
    publicCheckpointHash: "public-checkpoint-after-rng",
  }, webcrypto);

  await assert.rejects(
    async () => verifyLiveAuditTranscript(await buildCurrentProtocolTranscript({
      matchId: "m-rng",
      players: [
        { index: 0, keyPair: actorKey, auditPublicKey: actorPublicKey },
        { index: 1, keyPair: otherKey, auditPublicKey: otherPublicKey },
      ],
      actions: [{ seq: 1, actorIndex: 0, command, audit }],
    }), webcrypto),
    /must include every player exactly once/,
  );
});

test("live audit transcript verifier accepts signed fair-random reveals", async () => {
  const playerKeys = [
    await createAuditSessionKey(webcrypto),
    await createAuditSessionKey(webcrypto),
    await createAuditSessionKey(webcrypto),
  ];
  const playerPublicKeys = await Promise.all(
    playerKeys.map((keyPair) => exportAuditPublicKey(keyPair, webcrypto))
  );
  const matchId = "m-rng-signed";
  const seq = 1;
  const requirementId = "rng-1";
  const commits = [];
  const reveals = [];
  for (let player = 0; player < playerKeys.length; player += 1) {
    const requestId = `commit-${player}`;
    const revealRequestId = `reveal-${player}`;
    const nonceHex = `000000000000000000000000000000000000000000000000000000000000000${player}`;
    const commitmentHex = await sha256Hex(canonicalJson({
      domain: "ironsmith-rng-commit-v1",
      nonceHex,
    }), webcrypto);
    const commitPayload = rngCommitmentPayload({
      matchId,
      seq,
      requirementId,
      requestId,
      requester: 0,
      player,
      commitmentHex,
    });
    commits.push({
      player,
      requester: 0,
      requestId,
      commitmentHex,
      signature: await signAuditPayload(playerKeys[player], commitPayload, webcrypto),
    });
    const revealPayload = rngRevealPayload({
      matchId,
      seq,
      requirementId,
      requestId: revealRequestId,
      commitRequestId: requestId,
      requester: 0,
      player,
      nonceHex,
      commitmentHex,
    });
    reveals.push({
      player,
      requester: 0,
      requestId: revealRequestId,
      commitRequestId: requestId,
      nonceHex,
      commitmentHex,
      signature: await signAuditPayload(playerKeys[player], revealPayload, webcrypto),
    });
  }
  const command = { type: "priority_action", action_index: 0 };
  const rngReveal = {
    type: "commit_reveal_random",
    requirementId,
    commits,
    reveals,
    combinedSeedHex: await sha256Hex(canonicalJson({
      domain: "ironsmith-combined-rng-v1",
      matchId,
      seq,
      requirementId,
      commits,
      reveals,
    }), webcrypto),
  };
  const audit = await buildSignedActionEnvelope({
    keyPair: playerKeys[0],
    matchId,
    seq,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    rngReveals: [rngReveal],
    publicCheckpointHash: "public-checkpoint-after-rng",
  }, webcrypto);

  assert.equal((await verifyLiveAuditTranscript(await buildCurrentProtocolTranscript({
    matchId,
    players: playerPublicKeys.map((auditPublicKey, index) => ({
      index,
      keyPair: playerKeys[index],
      auditPublicKey,
    })),
    actions: [{ seq, actorIndex: 0, command, audit }],
  }), webcrypto)).valid, true);
});

test("live audit transcript verifier requires a shuffle-proof verifier", async () => {
  const actorKey = await createAuditSessionKey(webcrypto);
  const actorPublicKey = await exportAuditPublicKey(actorKey, webcrypto);
  const command = { type: "priority_action", action_index: 0 };
  const shuffleProof = {
    type: "ziffle_shuffle",
    requirementId: "shuffle-1",
    owner: 0,
    zone: "library",
    deckCount: 2,
    context: "ctx",
    keys: [],
    steps: [],
    deckHash: "deck-hash",
  };
  const audit = await buildSignedActionEnvelope({
    keyPair: actorKey,
    matchId: "m-shuffle",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    shuffleProofs: [shuffleProof],
    publicCheckpointHash: "public-checkpoint-after-shuffle",
  }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-shuffle",
    players: [{ index: 0, keyPair: actorKey, auditPublicKey: actorPublicKey }],
    actions: [{ seq: 1, actorIndex: 0, command, audit }],
  });

  await assert.rejects(
    () => verifyLiveAuditTranscript(transcript, webcrypto),
    /no verifier was provided/,
  );
  const report = await verifyLiveAuditTranscript(transcript, webcrypto, {
    verifyShuffleProof: async (proof) => {
      assert.equal(proof.deckHash, "deck-hash");
    },
  });
  assert.equal(report.valid, true);
});

test("match genesis and resync envelopes bind roster and checkpoints", async () => {
  const hostKey = await createAuditSessionKey(webcrypto);
  const playerKeys = [
    hostKey,
    await createAuditSessionKey(webcrypto),
    await createAuditSessionKey(webcrypto),
  ];
  const encryptionKeys = [
    await createAuditEncryptionKey(webcrypto),
    await createAuditEncryptionKey(webcrypto),
    await createAuditEncryptionKey(webcrypto),
  ];
  const publicKeys = await Promise.all(
    playerKeys.map((keyPair) => exportAuditPublicKey(keyPair, webcrypto))
  );
  const encryptionPublicKeys = await Promise.all(
    encryptionKeys.map((keyPair) => exportAuditEncryptionPublicKey(keyPair, webcrypto))
  );
  const hostPublicKey = publicKeys[0];
  const manifests = await Promise.all(playerKeys.map((_, index) =>
    buildPrivateDeckManifest({
      matchId: "m",
      owner: index,
      deck: index === 0 ? ["Island", "Mountain"] : [],
      saltForSlot: (slot) => `genesis-salt-${index}-${slot}`,
    }, webcrypto)
  ));
  const players = await Promise.all(playerKeys.map(async (keyPair, index) => {
    const player = {
      peerId: index === 0 ? "peer-host" : `peer-${index}`,
      name: index === 0 ? "Host" : `Player ${index + 1}`,
      index,
      auditPublicKey: publicKeys[index],
      auditEncryptionPublicKey: encryptionPublicKeys[index],
      deckAuditManifest: publicDeckManifest(manifests[index]),
      ziffleKey: {
        player: index,
        publicKeyHex: `abc-${index}`,
        ownershipProofHex: `proof-${index}`,
      },
      deckCount: manifests[index].deckCount,
      sideboardCount: 0,
      commanderCount: 0,
    };
    player.playerGenesisSignature = await buildSignedPlayerGenesis({
      keyPair,
      matchId: "m",
      protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
      timeoutMs: 300000,
      player,
    }, webcrypto);
    return player;
  }));
  const ziffleKeys = players.map((player) => player.ziffleKey);
  const match = {
    protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
    auditMatchId: "m",
    lobbyId: "m",
    hostPeerId: "peer-host",
    format: "normal",
    startingLife: 20,
    openingHandSize: 7,
    seed: 123,
    timeoutMs: 300000,
    matchClockPolicy: {
      type: "per_player_match_clock_v1",
      initialMs: 300000,
      graceMs: 2000,
    },
    initialPublicCheckpointHash: "initial-public-checkpoint",
    players,
    deckAuditManifests: manifests.map(publicDeckManifest),
    ziffleKeys,
    ziffleCeremonies: players.map((player) => ({
      owner: player.index,
      deckCount: player.deckCount,
      context: "m",
      keyContext: "m",
      keys: ziffleKeys,
      steps: players.map((shuffler) => ({
        shuffler: shuffler.index,
        deckHex: `deck-${player.index}-${shuffler.index}`,
        proofHex: `proof-${player.index}-${shuffler.index}`,
      })),
      deckHash: `deck-hash-${player.index}`,
    })),
  };
  match.genesis = await buildSignedMatchGenesis({
    keyPair: hostKey,
    match,
    hostSeat: 0,
  }, webcrypto);

  assert.equal((await verifySignedMatchGenesis(match, webcrypto)).valid, true);
  await assert.rejects(
    () => verifySignedMatchGenesis({
      ...match,
      players: [{ ...players[0], name: "Impostor" }, ...players.slice(1)],
    }, webcrypto),
    /genesis payload hash mismatch|genesis signature/,
  );

  const checkpoint = { players: [{ id: 0, hand: [] }], objects: [] };
  const actions = [];
  const envelope = await buildSignedResyncEnvelope({
    keyPair: hostKey,
    matchId: "m",
    signer: 0,
    lastSequence: 0,
    finalStateHash: "0".repeat(64),
    checkpoint,
    actions,
  }, webcrypto);
  const hostPublicCryptoKey = await importAuditPublicKey(hostPublicKey, webcrypto);
  assert.equal(
    (await verifySignedResyncEnvelope({
      envelope,
      publicKey: hostPublicCryptoKey,
      checkpoint,
      actions,
    }, webcrypto)).valid,
    true,
  );
  await assert.rejects(
    () => verifySignedResyncEnvelope({
      envelope,
      publicKey: hostPublicCryptoKey,
      checkpoint: { ...checkpoint, forged: true },
      actions,
    }, webcrypto),
    /checkpoint hash mismatch/,
  );
});
