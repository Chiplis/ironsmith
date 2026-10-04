import { buildZiffleInputDeck } from "../src/lib/ziffle-private-epochs.js";
import { signZiffleShuffleStep, ziffleShuffleStepStatement } from "../src/lib/ziffle-step-signatures.js";
import test from "node:test";
import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import {
  auditStateHash,
  actionQuorumThreshold,
  assertResyncActionsExtendLocalTranscript,
  assertCurrentAuditPlayerCount,
  authorizeCryptoMaterialRequestRequirements,
  cryptoMaterialResponsibleSeat,
  buildActionForkDisputeEvidence,
  buildSignedDisconnectForfeitVote,
  buildSignedProtocolResponseTimeoutVote,
  buildSignedMatchGenesis,
  buildSignedActionEnvelope,
  buildSignedActionQuorumVote,
  buildSignedPlayerGenesis,
  buildSignedResyncEnvelope,
  buildDeckSlotOpening,
  buildPrivateDeckManifest,
  buildZiffleOpeningProof,
  canonicalJson,
  CURRENT_AUDIT_PROTOCOL_VERSION,
  decklistHashForCards,
  publicDeckManifest,
  createAuditSessionKey,
  createAuditEncryptionKey,
  encryptPrivateAuditPayload,
  exportAuditEncryptionPublicKey,
  exportAuditPublicKey,
  fairRandomCombinedSeedHex,
  disconnectForfeitVoteThreshold,
  DISCONNECT_FORFEIT_REASON,
  protocolResponseTimeoutVoteThreshold,
  PROTOCOL_RESPONSE_TIMEOUT_REASON,
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
  verifyProtocolResponseTimeoutCertificate,
  verifyLiveAuditTranscript,
  verifyPrivateViewDisclosure,
  verifySignedMatchGenesis,
  verifySignedResyncEnvelope,
  auditMatchInstanceId,
  deriveMatchSeedFromReveals,
  genesisAuditChainHash,
  matchSeedCommitment,
  verifyActionQuorumVote,
} from "../src/lib/multiplayer-audit.js";

function cloneTestPayload(value) {
  return JSON.parse(JSON.stringify(value));
}

const P256_ORDER = BigInt("0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551");
const P256_HALF_ORDER = P256_ORDER >> 1n;

function p256SignatureS(signatureHex) {
  return BigInt(`0x${String(signatureHex || "").slice(64, 128) || "0"}`);
}

function fixedWidthScalarHex(value) {
  return value.toString(16).padStart(64, "0");
}

function malleateP256SignatureHex(signatureHex) {
  const normalized = String(signatureHex || "");
  const rHex = normalized.slice(0, 64);
  const s = p256SignatureS(normalized);
  return `${rHex}${fixedWidthScalarHex(P256_ORDER - s)}`;
}

// Tests sign their actions before the fixture builds the match genesis, so
// they chain from the all-zero placeholder root. Protocol v16 roots the chain
// in the signed genesis instead: re-chain every action that extends the
// placeholder, re-signing (and re-voting) only what was validly signed and
// recomputing only honestly computed state hashes, so any deliberate tamper
// a test planted survives the rebase.
const PLACEHOLDER_CHAIN_ROOT = "0".repeat(64);

function testEnvelopeStateFields(audit) {
  return {
    matchId: audit.matchId,
    seq: audit.seq,
    prevStateHash: audit.prevStateHash,
    command: audit.command,
    clock: audit.clock,
    openings: audit.openings || [],
    rngReveals: audit.rngReveals || [],
    shuffleProofs: audit.shuffleProofs || [],
    privateViewProofs: audit.privateViewProofs || [],
    publicCheckpointHash: audit.publicCheckpointHash,
  };
}

async function testSignatureIsValid(publicKeyHex, payload, signature) {
  if (!publicKeyHex || !signature) return false;
  try {
    return await verifyAuditPayload(await importAuditPublicKey(publicKeyHex, webcrypto), payload, signature, webcrypto);
  } catch {
    return false;
  }
}

async function rebaseTestActionsOntoGenesis(actions, chainRoot, privatePlayers) {
  const bySeat = new Map(privatePlayers.map((player) => [Number(player.index), player]));
  const rebasedHash = new Map([[PLACEHOLDER_CHAIN_ROOT, chainRoot]]);
  const rebased = [];
  for (const entry of actions) {
    const action = cloneTestPayload(entry);
    const audit = action?.audit;
    if (!audit || !rebasedHash.has(audit.prevStateHash)) {
      rebased.push(action);
      continue;
    }
    const original = cloneTestPayload(action);
    const signer = bySeat.get(Number(audit.signer ?? audit.actor));
    const signatureWasValid = await testSignatureIsValid(signer?.auditPublicKey,
      actionEnvelopePayload(original.audit), original.audit.signature);
    const nextWasHonest = audit.nextStateHash === await auditStateHash(testEnvelopeStateFields(original.audit), webcrypto);
    audit.prevStateHash = rebasedHash.get(audit.prevStateHash);
    if (nextWasHonest) {
      audit.nextStateHash = await auditStateHash(testEnvelopeStateFields(audit), webcrypto);
      rebasedHash.set(original.audit.nextStateHash, audit.nextStateHash);
    }
    if (signatureWasValid) {
      audit.signature = await signAuditPayload(signer.keyPair, actionEnvelopePayload(audit), webcrypto);
    }
    const certificate = audit.quorumCertificate || action.quorumCertificate;
    if (certificate) {
      const votes = [];
      for (const vote of certificate.votes || []) {
        const voter = bySeat.get(Number(vote?.voter));
        const voteWasValid = voter && await verifyActionQuorumVote({ vote, action: original,
          players: privatePlayers }, webcrypto).then(() => true, () => false);
        votes.push(voteWasValid
          ? await buildSignedActionQuorumVote({ keyPair: voter.keyPair, action, voter: vote.voter }, webcrypto)
          : vote);
      }
      Object.assign(certificate, {
        prevStateHash: audit.prevStateHash,
        nextStateHash: audit.nextStateHash,
        actionSignature: audit.signature,
        votes,
      });
    }
    rebased.push(action);
  }
  return rebased;
}

function testSeedNonce(seat) {
  return (Number(seat) + 1).toString(16).padStart(64, "0");
}

// Protocol v16 requires every ziffle shuffle step signed by its shuffler's
// genesis audit key; `signers` holds each seat's `index` and `keyPair`.
// ECDSA signatures are randomized, but accepted shuffle histories embed the
// signed steps, so a fixture rebuilt from the same keys must reproduce the
// same signatures: memoize per key pair and step statement.
const testStepSignatures = new WeakMap();

async function signTestZiffleCeremony(ceremony, signers) {
  const keyPairs = new Map(signers.map((signer) => [Number(signer.index), signer.keyPair]));
  const steps = [];
  for (const step of ceremony.steps) {
    const keyPair = keyPairs.get(Number(step.shuffler));
    if (!testStepSignatures.has(keyPair)) testStepSignatures.set(keyPair, new Map());
    const memo = testStepSignatures.get(keyPair);
    const statementKey = canonicalJson(await ziffleShuffleStepStatement(ceremony, steps, step, webcrypto));
    if (!memo.has(statementKey)) {
      memo.set(statementKey, await signZiffleShuffleStep(keyPair, ceremony, steps, step, webcrypto));
    }
    steps.push({ ...step, signature: memo.get(statementKey) });
  }
  return { ...ceremony, steps };
}

function actionEnvelopePayload(audit) {
  return {
    matchId: audit.matchId,
    seq: Number(audit.seq),
    actor: Number(audit.actor),
    signer: Number(audit.signer ?? audit.actor),
    prevStateHash: audit.prevStateHash,
    command: audit.command,
    clock: audit.clock,
    openings: audit.openings || [],
    rngReveals: audit.rngReveals || [],
    shuffleProofs: audit.shuffleProofs || [],
    privateViewProofs: audit.privateViewProofs || [],
    publicCheckpointHash: audit.publicCheckpointHash,
    nextStateHash: audit.nextStateHash,
  };
}

function verifyEnvelopeOnlyTranscript(transcript, options = {}) {
  return verifyLiveAuditTranscript(transcript, webcrypto, {
    requireEngineReplay: false,
    ...options,
  });
}

async function finalDisclosureFixture({ initialPublicCheckpointHash } = {}) {
  const matchId = "m-final-origin-disclosure";
  const keyPair = await createAuditSessionKey(webcrypto);
  const deck = Array(61).fill("Mountain");
  deck[2] = deck[4] = "Barbarian Ring";
  const manifest = await buildPrivateDeckManifest({ matchId, owner: 1, deck }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId, players: [{ index: 1, keyPair, deckAuditManifest: manifest }], actions: [],
    initialPublicCheckpointHash,
  });
  const ceremony = transcript.match.ziffleCeremonies.find(entry => entry.owner === 1);
  const opening = {
    ...await buildDeckSlotOpening({ manifest, slot: 4 }, webcrypto),
    position: 51, positionCommitment: "ziffle:later-fetch:51",
    originPosition: 23, originPositionCommitment: `ziffle:${ceremony.deckHash}:23`,
  };
  const tokens = ceremony.keys.map(key => ({ ...key, tokenHex: "token", proofHex: "valid" }));
  opening.ziffleReveal = buildZiffleOpeningProof({ opening, ceremony, tokens, compact: true });
  const signedEntry = async (openings = [opening], fields = {}) => {
    const payload = { domain: "ironsmith-end-of-match-disclosure-v1", matchId, player: 1, openings, ...fields };
    return { player: 1, matchId, disclosure: { ...payload, signature: await signAuditPayload(keyPair, payload, webcrypto) } };
  };
  transcript.endOfMatchDisclosures = [await signedEntry()];
  const disclosureReports = [{ player: 1, replayVerdict: { status: "verified", reason: "" } }];
  const proofCalls = [];
  const options = {
    verifyZiffleOpening: async input => {
      proofCalls.push(input);
      if (input.proof.tokens.some(token => token.proofHex !== "valid")) throw new Error("Invalid reveal token proof");
      return { originalSlot: 4 };
    },
    replayTranscript: async ({ finalPublicCheckpointHash }) => ({
      finalPublicCheckpointHash,
      endOfMatchDisclosures: disclosureReports,
      endOfMatchDisclosuresVerified: true,
    }),
  };
  return { transcript, manifest, ceremony, tokens, opening, signedEntry, options, proofCalls, disclosureReports };
}

async function buildCurrentProtocolTranscript({
  matchId,
  players,
  actions,
  deckAuditManifests = [],
  privateViewDisclosures = [],
  initialPublicCheckpointHash = "initial-public-checkpoint",
  playerCount = null,
  protocolVersion = CURRENT_AUDIT_PROTOCOL_VERSION,
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
    // Seed commit-reveal: each seat commits H(nonce) in its player genesis.
    entry.seedNonce = testSeedNonce(index);
    entry.playerGenesisSignature = await buildSignedPlayerGenesis({
      keyPair,
      matchId,
      protocolVersion,
      timeoutMs: 300000,
      player: entry,
      seedCommitment: await matchSeedCommitment(entry.seedNonce, webcrypto),
    }, webcrypto);
    privatePlayers.push(entry);
  }
  const normalizedPlayers = privatePlayers.map((player) => {
    const entry = { ...player };
    delete entry.keyPair;
    delete entry.seedNonce;
    return entry;
  });
  const ziffleKeys = normalizedPlayers.map((player) => player.ziffleKey);
  const ziffleCeremonies = await Promise.all(normalizedPlayers.map((player) => signTestZiffleCeremony({
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
  }, privatePlayers)));
  const host = privatePlayers[0];
  const seedReveals = privatePlayers.map((player) => ({ seat: player.index, nonce: player.seedNonce }));
  const match = {
    protocolVersion,
    auditMatchId: matchId,
    lobbyId: matchId,
    hostPeerId: host.peerId,
    format: "normal",
    startingLife: 20,
    openingHandSize: 7,
    seed: await deriveMatchSeedFromReveals({ matchId, reveals: seedReveals }, webcrypto),
    seedReveals,
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
  const chainRoot = await genesisAuditChainHash({ matchId, genesisPayloadHash: match.genesis.payloadHash }, webcrypto);
  // `actions` may be a callback for actions that must bind genesis-derived
  // values (the chain root, or the match instance id forfeit votes sign).
  const suppliedActions = typeof actions === "function"
    ? await actions({ chainRoot, matchInstanceId: auditMatchInstanceId(match), match })
    : actions;
  const rebasedActions = await rebaseTestActionsOntoGenesis(suppliedActions, chainRoot, privatePlayers);
  const actionsWithQuorum = await Promise.all(rebasedActions.map(async (entry) => {
    const action = cloneTestPayload(entry);
    if (!action?.audit || action.audit.quorumCertificate || action.quorumCertificate) {
      return action;
    }
    const threshold = actionQuorumThreshold(privatePlayers.length);
    if (threshold <= 0) return action;
    // The actor's own vote never counts toward the threshold.
    const actor = Number(action.audit.actor ?? action.actorIndex ?? 0);
    const quorumVoters = privatePlayers.filter((player) => Number(player.index) !== actor).slice(0, threshold);
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
    protocolVersion,
    signatureAlgorithm: "ecdsa-p256-sha256",
    genesis: match.genesis,
    initialStateHash: chainRoot,
    initialPublicCheckpointHash,
    privateViewDisclosures,
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

test("crypto material authorization rejects forged private slot requests", () => {
  const previewed = [{
    id: "play-visible-card",
    type: "public_open",
    owner: 1,
    zone: "hand",
    slot: 4,
    objectId: 99,
    commitment: "commitment-4",
  }];

  assert.deepEqual(
    authorizeCryptoMaterialRequestRequirements({
      localSeat: 1,
      requestedRequirements: [{
        id: "play-visible-card",
        type: "public_open",
        owner: 1,
        zone: "hand",
        slot: 4,
        objectId: 99,
        commitment: "commitment-4",
      }],
      previewedRequirements: previewed,
    }),
    previewed,
  );

  assert.deepEqual(
    authorizeCryptoMaterialRequestRequirements({
      localSeat: 1,
      requestedRequirements: [{
        id: "play-visible-card",
        type: "public_open",
        owner: 1,
      }],
      previewedRequirements: previewed,
    }),
    previewed,
  );

  assert.deepEqual(
    authorizeCryptoMaterialRequestRequirements({
      localSeat: null,
      requestedRequirements: previewed,
      previewedRequirements: previewed,
    }),
    [],
  );

  assert.throws(
    () => authorizeCryptoMaterialRequestRequirements({
      localSeat: 1,
      requestedRequirements: [{
        type: "public_open",
        owner: 1,
        slot: 0,
      }],
      previewedRequirements: previewed,
    }),
    /unauthorized hidden-card material/,
  );

  assert.throws(
    () => authorizeCryptoMaterialRequestRequirements({
      localSeat: 1,
      requestedRequirements: [{
        type: "public_open",
        owner: 1,
        slot: 0,
        card: "Black Lotus",
        commitment: "public-slot-0-commitment",
      }],
      previewedRequirements: previewed,
    }),
    /unauthorized hidden-card material/,
  );

  const postApplyRequirement = {
    id: "post-apply-open",
    type: "public_open",
    owner: 1,
    zone: "library",
    slot: 0,
    objectId: 123,
    commitment: "post-apply-commitment",
  };
  assert.throws(
    () => authorizeCryptoMaterialRequestRequirements({
      localSeat: 1,
      requestedRequirements: [postApplyRequirement],
      previewedRequirements: previewed,
    }),
    /unauthorized hidden-card material/,
  );

  assert.deepEqual(
    authorizeCryptoMaterialRequestRequirements({
      localSeat: 1,
      requestedRequirements: [postApplyRequirement],
      previewedRequirements: [...previewed, postApplyRequirement],
    }),
    [postApplyRequirement],
  );
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

test("public checkpoint hashes normalize public object runtime ids to stable ids", async () => {
  const hostCheckpoint = {
    version: 1,
    players: [
      { id: 0, handCount: 6, libraryCount: 53, graveyard: [140], commanders: [] },
      { id: 1, handCount: 8, libraryCount: 52, graveyard: [], commanders: [] },
    ],
    objects: [
      {
        id: 135,
        stableId: 60,
        owner: 0,
        controller: 0,
        zone: "battlefield",
        identity: { name: "Mountain", cardTypes: ["Land"], subtypes: ["Mountain"], oracleText: "" },
        attachments: [140],
        attachedTo: null,
        tapped: false,
      },
      {
        id: 140,
        stableId: 64,
        owner: 0,
        controller: 0,
        zone: "graveyard",
        identity: { name: "Mountain", cardTypes: ["Land"], subtypes: ["Mountain"], oracleText: "" },
        attachments: [],
        attachedTo: { kind: "object", object: 135 },
        tapped: false,
      },
    ],
    battlefield: [135],
    publicExile: [],
    command: [],
    stack: [{ objectId: 140, controller: 0, targets: [{ kind: "object", object: 135 }] }],
    hiddenZones: [{ owner: 0, zone: "hand", count: 6, commitmentRoot: "same-root" }],
  };
  const guestCheckpoint = cloneTestPayload(hostCheckpoint);
  guestCheckpoint.players[0].graveyard = [940];
  guestCheckpoint.objects[0].id = 931;
  guestCheckpoint.objects[1].id = 940;
  guestCheckpoint.objects[0].attachments = [940];
  guestCheckpoint.objects[1].attachedTo.object = 931;
  guestCheckpoint.battlefield = [931];
  guestCheckpoint.stack[0].objectId = 940;
  guestCheckpoint.stack[0].targets[0].object = 931;

  assert.equal(
    await publicCheckpointHash(hostCheckpoint, webcrypto),
    await publicCheckpointHash(guestCheckpoint, webcrypto),
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

test("audit payload signatures are canonical low-S P-256 signatures", async () => {
  const keyPair = await createAuditSessionKey(webcrypto);
  const payload = {
    matchId: "m-low-s",
    seq: 1,
    actor: 0,
    signer: 0,
    prevStateHash: "0".repeat(64),
    command: { type: "priority_action", action_index: 0 },
    openings: [],
    rngReveals: [],
    shuffleProofs: [],
    privateViewProofs: [],
    publicCheckpointHash: "public-checkpoint-low-s",
    nextStateHash: "1".repeat(64),
  };
  const signature = await signAuditPayload(keyPair, payload, webcrypto);
  assert.equal(signature.length, 128);
  assert.ok(p256SignatureS(signature) <= P256_HALF_ORDER);

  const publicKey = await importAuditPublicKey(
    await exportAuditPublicKey(keyPair, webcrypto),
    webcrypto,
  );
  assert.equal(await verifyAuditPayload(publicKey, payload, signature, webcrypto), true);
  assert.equal(
    await verifyAuditPayload(publicKey, payload, malleateP256SignatureHex(signature), webcrypto),
    false,
  );
});

test("action quorum certificates require a strict majority of the non-actor players", async () => {
  assert.equal(actionQuorumThreshold(2), 0);
  assert.equal(actionQuorumThreshold(3), 2);
  assert.equal(actionQuorumThreshold(4), 2);
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
    threshold: 2,
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
    (await verifyEnvelopeOnlyTranscript(await buildCurrentProtocolTranscript({
      matchId: "m-quorum",
      players: players.map((player, index) => ({
        index: player.index,
        keyPair: keys[index],
        auditPublicKey: player.auditPublicKey,
      })),
      actions: [action],
    }))).valid,
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
    /1 non-actor vote\(s\), expected at least 2/,
  );
});

test("three-player live transcripts need both opponents to co-sign each action", async () => {
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
  const votes = await Promise.all([1, 2].map((voter) =>
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
    voters: [1, 2],
    votes,
  };

  assert.equal(
    (await verifyEnvelopeOnlyTranscript(await buildCurrentProtocolTranscript({
      matchId: "m-three-player-two-of-three",
      players,
      playerCount: 3,
      actions: [action],
    }))).valid,
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

test("disconnect timeout policy forfeits require unanimous non-target consent", async () => {
  assert.equal(disconnectForfeitVoteThreshold(0), 0);
  assert.equal(disconnectForfeitVoteThreshold(1), 1);
  assert.equal(disconnectForfeitVoteThreshold(2), 2);
  assert.equal(disconnectForfeitVoteThreshold(3), 3);

  const matchId = "m-disconnect-forfeit";
  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
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
    reason: DISCONNECT_FORFEIT_REASON,
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
  const staggeredObservationVote = await buildSignedDisconnectForfeitVote({
    keyPair: keys[2],
    matchId,
    basisSequence: 0,
    forfeitedPlayer: 1,
    forfeitedPeerId: "peer-1",
    disconnectTimeoutMs: 60000,
    disconnectedAtMs: 102000,
    eligibleAtMs: 162000,
    signedAtMs: 162500,
    voter: 2,
  }, webcrypto);
  assert.equal(
    (await verifyDisconnectForfeitCertificate({
      certificate: {
        ...disconnectCertificate,
        votes: [votes[0], staggeredObservationVote],
      },
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
  await assert.rejects(
    () => verifyDisconnectForfeitCertificate({
      certificate: disconnectCertificate,
      command: { ...command, matchId },
      players: [players[0], players[2], players[3]],
    }, webcrypto),
    /expected at least 3/,
  );
  await assert.rejects(
    () => verifyDisconnectForfeitCertificate({
      certificate: disconnectCertificate,
      command: { ...command, matchId, disconnected_peer_id: "peer-forged" },
      players: [players[0], players[2]],
    }, webcrypto),
    /does not match the command/,
  );
  await assert.rejects(
    () => verifyDisconnectForfeitCertificate({
      certificate: disconnectCertificate,
      command: {
        ...command,
        matchId,
        nowMs: 160000,
        maxFutureSkewMs: 0,
      },
      players: [players[0], players[2]],
    }, webcrypto),
    /signed in the future/,
  );

  const transcript = await buildCurrentProtocolTranscript({
    matchId,
    players: players.slice(0, 3),
    playerCount: 3,
    // Forfeit votes in a live transcript sign the match instance id.
    actions: async ({ matchInstanceId, chainRoot }) => {
      const instanceVotes = await Promise.all([0, 2].map((voter) =>
        buildSignedDisconnectForfeitVote({
          keyPair: keys[voter],
          matchId: matchInstanceId,
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
      const instanceCommand = {
        ...command,
        disconnect_certificate: { ...disconnectCertificate, matchId: matchInstanceId, votes: instanceVotes },
      };
      const audit = await buildSignedActionEnvelope({
        keyPair: keys[0],
        matchId,
        seq: 1,
        actor: 0,
        prevStateHash: chainRoot,
        command: instanceCommand,
        publicCheckpointHash: "public-checkpoint-after-disconnect-forfeit",
      }, webcrypto);
      return [{ seq: 1, actorIndex: 0, command: instanceCommand, audit }];
    },
  });

  assert.equal((await verifyEnvelopeOnlyTranscript(transcript)).valid, true);
});

test("protocol response timeout forfeits require non-target quorum", async () => {
  assert.equal(protocolResponseTimeoutVoteThreshold(0), 0);
  assert.equal(protocolResponseTimeoutVoteThreshold(1), 0);
  assert.equal(protocolResponseTimeoutVoteThreshold(2), 2);
  assert.equal(protocolResponseTimeoutVoteThreshold(3), 3);

  const matchId = "m-protocol-response-timeout";
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
  const baseVote = {
    matchId,
    basisSequence: 0,
    forfeitedPlayer: 1,
    forfeitedPeerId: "peer-1",
    requestType: "crypto_material_request",
    requestId: "crypto-material:test",
    requestPayloadHash: "a".repeat(64),
    responseTimeoutMs: 60000,
    requestedAtMs: 100000,
    eligibleAtMs: 160000,
  };
  const votes = await Promise.all([0, 2].map((voter) =>
    buildSignedProtocolResponseTimeoutVote({
      keyPair: keys[voter],
      ...baseVote,
      signedAtMs: 160500,
      voter,
    }, webcrypto)
  ));
  const certificate = {
    type: "ironsmith-protocol-response-timeout-v1",
    ...baseVote,
    threshold: 2,
    voters: [0, 2],
    votes,
  };
  const command = {
    type: "forfeit_player",
    player: 1,
    reason: PROTOCOL_RESPONSE_TIMEOUT_REASON,
    timed_out_peer_id: "peer-1",
    request_type: baseVote.requestType,
    request_id: baseVote.requestId,
    request_payload_hash: baseVote.requestPayloadHash,
    response_timeout_ms: baseVote.responseTimeoutMs,
    requested_at_ms: baseVote.requestedAtMs,
    eligible_at_ms: baseVote.eligibleAtMs,
    claimed_at_ms: 161000,
    basis_sequence: 0,
    protocol_timeout_certificate: certificate,
  };

  assert.equal(
    (await verifyProtocolResponseTimeoutCertificate({
      certificate,
      command: { ...command, matchId },
      players: [players[0], players[2]],
    }, webcrypto)).valid,
    true,
  );
  await assert.rejects(
    () => verifyProtocolResponseTimeoutCertificate({
      certificate: {
        ...certificate,
        votes: votes.slice(0, 1),
      },
      command: { ...command, matchId },
      players: [players[0], players[2]],
    }, webcrypto),
    /expected at least 2/,
  );
  await assert.rejects(
    () => verifyProtocolResponseTimeoutCertificate({
      certificate,
      command: { ...command, matchId, request_payload_hash: "b".repeat(64) },
      players: [players[0], players[2]],
    }, webcrypto),
    /does not match the command/,
  );
  const earlyVote = await buildSignedProtocolResponseTimeoutVote({
    keyPair: keys[2],
    ...baseVote,
    signedAtMs: 159999,
    voter: 2,
  }, webcrypto);
  await assert.rejects(
    () => verifyProtocolResponseTimeoutCertificate({
      certificate: {
        ...certificate,
        votes: [votes[0], earlyVote],
      },
      command: { ...command, matchId },
      players: [players[0], players[2]],
    }, webcrypto),
    /before the response timeout elapsed/,
  );

  const transcript = await buildCurrentProtocolTranscript({
    matchId,
    players,
    playerCount: 3,
    // Forfeit votes in a live transcript sign the match instance id.
    actions: async ({ matchInstanceId, chainRoot }) => {
      const instanceVotes = await Promise.all([0, 2].map((voter) =>
        buildSignedProtocolResponseTimeoutVote({
          keyPair: keys[voter],
          ...baseVote,
          matchId: matchInstanceId,
          signedAtMs: 160500,
          voter,
        }, webcrypto)
      ));
      const instanceCommand = {
        ...command,
        protocol_timeout_certificate: { ...certificate, matchId: matchInstanceId, votes: instanceVotes },
      };
      const audit = await buildSignedActionEnvelope({
        keyPair: keys[0],
        matchId,
        seq: 1,
        actor: 0,
        prevStateHash: chainRoot,
        command: instanceCommand,
        publicCheckpointHash: "public-checkpoint-after-protocol-timeout",
      }, webcrypto);
      return [{ seq: 1, actorIndex: 0, command: instanceCommand, audit }];
    },
  });

  assert.equal((await verifyEnvelopeOnlyTranscript(transcript)).valid, true);
});

test("live transcript verifier rejects a fabricated forfeit of another player", async () => {
  const matchId = "m-forged-forfeit";
  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
  ]);
  const players = await Promise.all(keys.map(async (keyPair, index) => ({
    index,
    keyPair,
    peerId: `peer-${index}`,
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto),
  })));

  const transcriptForCommand = async (command, publicCheckpointHash) => {
    const audit = await buildSignedActionEnvelope({
      keyPair: keys[0],
      matchId,
      seq: 1,
      actor: 0,
      prevStateHash: "0".repeat(64),
      command,
      publicCheckpointHash,
    }, webcrypto);
    return buildCurrentProtocolTranscript({
      matchId,
      players,
      playerCount: 2,
      actions: [{ seq: 1, actorIndex: 0, command, audit }],
    });
  };

  // A generic forfeit aimed at the opponent is rejected even though a two-player
  // game requires no quorum certificate: the live gate forbids forfeiting another
  // player, and the transcript verifier must agree or a player could fabricate a
  // winning transcript from a forfeit that never happened.
  const genericForfeit = await transcriptForCommand(
    { type: "forfeit_player", player: 1 },
    "public-checkpoint-forged-generic-forfeit",
  );
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(genericForfeit),
    /without a valid involuntary-forfeit reason/,
  );

  // Dressing the same forfeit up as a match-clock timeout does not help: with only
  // two players there is no independent (non-actor, non-target) witness, so the
  // transcript alone cannot prove the opponent timed out.
  const timeoutForfeit = await transcriptForCommand(
    { type: "forfeit_player", player: 1, reason: "match_clock_timeout" },
    "public-checkpoint-forged-timeout-forfeit",
  );
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(timeoutForfeit),
    /without independent attestation/,
  );

  // A self-forfeit (concede) must still verify — the fix must not block legitimate
  // surrenders.
  const selfForfeit = await transcriptForCommand(
    { type: "forfeit_player", player: 0 },
    "public-checkpoint-self-forfeit",
  );
  assert.equal((await verifyEnvelopeOnlyTranscript(selfForfeit)).valid, true);
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
  // Two conflicting seq-1 actions, each certified by both of its actor's
  // opponents: player 1 signed both, so it is the one accused.
  const buildFork = async (prevStateHash) => {
    const firstAudit = await buildSignedActionEnvelope({
      keyPair: keys[0],
      matchId: "m-action-fork",
      seq: 1,
      actor: 0,
      prevStateHash,
      command: firstCommand,
      publicCheckpointHash: "public-checkpoint-first-branch",
    }, webcrypto);
    const secondAudit = await buildSignedActionEnvelope({
      keyPair: keys[2],
      matchId: "m-action-fork",
      seq: 1,
      actor: 2,
      prevStateHash,
      command: secondCommand,
      publicCheckpointHash: "public-checkpoint-second-branch",
    }, webcrypto);
    const firstAction = { seq: 1, actorIndex: 0, command: firstCommand, audit: firstAudit };
    const secondAction = { seq: 1, actorIndex: 2, command: secondCommand, audit: secondAudit };
    const certify = async (action, voters) => {
      const votes = await Promise.all(voters.map((voter) =>
        buildSignedActionQuorumVote({ keyPair: keys[voter], action, voter }, webcrypto)
      ));
      action.audit.quorumCertificate = {
        type: "ironsmith-action-quorum-v1",
        matchId: action.audit.matchId,
        seq: action.audit.seq,
        actor: action.audit.actor,
        prevStateHash: action.audit.prevStateHash,
        nextStateHash: action.audit.nextStateHash,
        publicCheckpointHash: action.audit.publicCheckpointHash,
        actionSignature: action.audit.signature,
        threshold: 2,
        voters,
        votes,
      };
    };
    await certify(firstAction, [1, 2]);
    await certify(secondAction, [0, 1]);
    return {
      firstAction,
      dispute: buildActionForkDisputeEvidence({
        sequence: 1,
        existingAction: firstAction,
        conflictingAction: secondAction,
      }),
    };
  };
  let dispute = null;
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-action-fork",
    players,
    playerCount: 3,
    actions: async ({ chainRoot }) => {
      const fork = await buildFork(chainRoot);
      dispute = fork.dispute;
      return [fork.firstAction];
    },
  });
  assert.deepEqual(dispute.accusedPlayers, [1]);
  transcript.disputes = [dispute];
  transcript.outcome = {
    status: "disputed",
    accusedPlayers: [1],
  };
  const report = await verifyEnvelopeOnlyTranscript(transcript);
  assert.equal(report.valid, true);
  assert.equal(report.outcome.status, "disputed");
  assert.deepEqual(report.outcome.accusedPlayers, [1]);

  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript({
      ...transcript,
      outcome: {
        status: "disputed",
        accusedPlayers: [2],
      },
    }),
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

  const report = await verifyEnvelopeOnlyTranscript(transcript);
  assert.equal(report.valid, true);
  assert.equal(report.outcome.status, "winner");
  assert.equal(report.outcome.winner, 0);
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript({
      ...transcript,
      outcome: {
        status: "winner",
        winner: 1,
      },
    }),
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
  assert.equal((await verifyEnvelopeOnlyTranscript(transcript)).valid, true);
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript({
      ...transcript,
      actions: [{
        ...transcript.actions[0],
        command: { type: "draw_cards", player: 0, count: 1 },
      }],
    }),
    /command mismatch/,
  );
});

test("action-fork disputes ignore alternate signatures for the same signed payload", async () => {
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
  let dispute = null;
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-action-same-payload",
    players,
    playerCount: 2,
    actions: async ({ chainRoot }) => {
      const audit = await buildSignedActionEnvelope({
        keyPair: keys[0],
        matchId: "m-action-same-payload",
        seq: 1,
        actor: 0,
        prevStateHash: chainRoot,
        command,
        publicCheckpointHash: "public-checkpoint-same-payload",
      }, webcrypto);
      const action = { seq: 1, actorIndex: 0, command, audit };
      const payload = actionEnvelopePayload(audit);
      let alternateSignature = audit.signature;
      for (let attempt = 0; attempt < 8 && alternateSignature === audit.signature; attempt += 1) {
        alternateSignature = await signAuditPayload(keys[0], payload, webcrypto);
      }
      assert.notEqual(alternateSignature, audit.signature);
      const duplicateAction = cloneTestPayload(action);
      duplicateAction.audit.signature = alternateSignature;
      dispute = buildActionForkDisputeEvidence({
        sequence: 1,
        existingAction: action,
        conflictingAction: duplicateAction,
      });
      return [action];
    },
  });
  assert.deepEqual(dispute.accusedPlayers, []);
  transcript.disputes = [dispute];

  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(transcript),
    /does not contain conflicting actions/,
  );
});

test("live audit transcript verifier can require engine replayed checkpoint hashes", async () => {
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
    matchId: "m-engine-replay",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    publicCheckpointHash: "public-checkpoint-after-engine-replay",
  }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-engine-replay",
    players,
    playerCount: 2,
    actions: [{ seq: 1, actorIndex: 0, command, audit }],
  });

  await assert.rejects(
    () => verifyLiveAuditTranscript(transcript, webcrypto, { requireEngineReplay: true }),
    /requires engine replay/,
  );

  await assert.rejects(
    () => verifyLiveAuditTranscript(transcript, webcrypto, {
      requireEngineReplay: true,
      replayTranscript: async ({ finalPublicCheckpointHash }) => ({
        actionReports: [],
        finalPublicCheckpointHash,
      }),
    }),
    /Engine replay must report every action in the transcript/,
  );

  const report = await verifyLiveAuditTranscript(transcript, webcrypto, {
    requireEngineReplay: true,
    replayTranscript: async ({ actions, finalPublicCheckpointHash }) => ({
      actionReports: actions.map((action) => ({
        seq: Number(action.seq),
        publicCheckpointHash: String(action.audit.publicCheckpointHash || ""),
      })),
      finalPublicCheckpointHash,
    }),
  });
  assert.equal(report.valid, true);
  assert.equal(report.engineReplay.verified, true);
  assert.equal(report.engineReplay.replayedActions, 1);

  await assert.rejects(
    () => verifyLiveAuditTranscript(transcript, webcrypto, {
      requireEngineReplay: true,
      replayTranscript: async ({ actions }) => ({
        actionReports: actions.map((action) => ({
          seq: Number(action.seq),
          publicCheckpointHash: "forged-public-checkpoint",
        })),
      }),
    }),
    /Engine replay public checkpoint hash mismatch at sequence 1/,
  );
});

test("final disclosures verify genesis proofs separately from current positions and retain engine verdicts", async () => {
  const h = await finalDisclosureFixture();
  const report = await verifyLiveAuditTranscript(h.transcript, webcrypto, h.options);
  assert.equal(report.valid, true);
  assert.equal(h.proofCalls.length, 1);
  assert.equal(h.proofCalls[0].proof.position, 23);
  assert.equal(h.proofCalls[0].opening.position, 51);
  assert.equal(h.proofCalls[0].ceremony.deckCount, 61);
  assert.deepEqual(report.engineReplay.endOfMatchDisclosures, h.disclosureReports);
  assert.equal(report.engineReplay.endOfMatchDisclosuresVerified, true);
});

test("final disclosures reject wrong signatures, match, player, domain and ownership", async () => {
  const h = await finalDisclosureFixture();
  for (const [entry, message] of [
    [null, /missing its signed payload/],
    [await h.signedEntry([h.opening], { domain: "other-domain" }), /wrong domain/],
    [await h.signedEntry([h.opening], { matchId: "other-match" }), /different match/],
    [await h.signedEntry([h.opening], { player: 7 }), /unknown player/],
    [{ ...await h.signedEntry(), player: 0 }, /player does not match/],
    [await h.signedEntry([{ ...h.opening, owner: 0 }]), /different player/],
    [await h.signedEntry([{ ...h.opening, owner: undefined }]), /different player/],
    [await h.signedEntry(null), /missing its openings/],
  ]) {
    await assert.rejects(verifyLiveAuditTranscript({ ...h.transcript, endOfMatchDisclosures: [entry] }, webcrypto, h.options), message);
  }
  const tampered = structuredClone(h.transcript);
  tampered.endOfMatchDisclosures[0].disclosure.openings[0].position = 50;
  await assert.rejects(verifyLiveAuditTranscript(tampered, webcrypto, h.options), /signature is invalid/);
  await assert.rejects(verifyLiveAuditTranscript({ ...h.transcript,
    endOfMatchDisclosures: [...h.transcript.endOfMatchDisclosures, ...h.transcript.endOfMatchDisclosures],
  }, webcrypto, h.options), /duplicate player/);
});

test("signed final disclosures cannot swap duplicate cards, alter origins or omit genesis proofs", async () => {
  const h = await finalDisclosureFixture();
  const wrongCopy = { ...h.opening, ...await buildDeckSlotOpening({ manifest: h.manifest, slot: 2 }, webcrypto) };
  wrongCopy.ziffleReveal = buildZiffleOpeningProof({ opening: wrongCopy, ceremony: h.ceremony,
    shuffleOriginalSlot: 4, tokens: h.tokens, compact: true });
  const missingProof = structuredClone(h.opening); delete missingProof.ziffleReveal;
  const wrongOrigin = { ...h.opening, originPosition: 22, originPositionCommitment: `ziffle:${h.ceremony.deckHash}:22` };
  const badToken = structuredClone(h.opening); badToken.ziffleReveal.tokens[0].proofHex = "forged";
  for (const [opening, message] of [
    [{ ...h.opening, salt: "forged" }, /does not match committed deck slot/],
    [wrongCopy, /different committed slot/],
    [missingProof, /missing its position reveal proof/],
    [wrongOrigin, /proof position mismatch/],
    [badToken, /Invalid reveal token proof/],
  ]) {
    await assert.rejects(verifyLiveAuditTranscript({ ...h.transcript,
      endOfMatchDisclosures: [await h.signedEntry([opening])],
    }, webcrypto, h.options), message);
  }
});

test("final disclosure engine failures and incomplete report coverage cannot become verified", async () => {
  const h = await finalDisclosureFixture();
  for (const [fields, message] of [
    [{ endOfMatchDisclosuresVerified: false }, /did not verify end-of-match/],
    [{ endOfMatchDisclosures: [{ player: 1, replayVerdict: { status: "cheat_detected", reason: "wrong trusted origin" } }], endOfMatchDisclosuresVerified: true }, /wrong trusted origin/],
    [{ endOfMatchDisclosures: [{ player: 1, replayVerdict: { status: "missing", reason: "required disclosure missing" } }], endOfMatchDisclosuresVerified: true }, /required disclosure missing/],
    [{}, /must report verification/],
    [{ endOfMatchDisclosures: [], endOfMatchDisclosuresVerified: true }, /coverage does not match/],
    [{ endOfMatchDisclosures: h.disclosureReports }, /must report verification/],
    [{ endOfMatchDisclosures: {}, endOfMatchDisclosuresVerified: true }, /invalid end-of-match disclosure reports/],
    [{ endOfMatchDisclosures: [...h.disclosureReports, ...h.disclosureReports], endOfMatchDisclosuresVerified: true }, /player coverage/],
    [{ endOfMatchDisclosures: [{ player: 99, replayVerdict: { status: "verified" } }], endOfMatchDisclosuresVerified: true }, /player coverage/],
  ]) {
    await assert.rejects(verifyLiveAuditTranscript(h.transcript, webcrypto, {
      ...h.options, replayTranscript: async ({ finalPublicCheckpointHash }) => ({ finalPublicCheckpointHash, ...fields }),
    }), message);
  }
  // Missing required entries discovered by the engine remain fatal even when
  // the exporter omits the entire disclosure collection.
  const noDisclosures = { ...h.transcript, endOfMatchDisclosures: [] };
  await assert.rejects(verifyLiveAuditTranscript(noDisclosures, webcrypto, {
    ...h.options, replayTranscript: async () => ({ endOfMatchDisclosuresVerified: false,
      endOfMatchDisclosures: [{ player: 1, replayVerdict: { status: "missing" } }] }),
  }), /did not verify end-of-match/);
});

test("a terminal checkpoint requires engine disclosure checks even without exported records", async () => {
  const checkpoint = { players: [{ id: 0, hasWon: true }, { id: 1, hasLost: true }], hiddenZones: [] };
  const hash = await publicCheckpointHash(checkpoint, webcrypto);
  const h = await finalDisclosureFixture({ initialPublicCheckpointHash: hash });
  const transcript = { ...h.transcript, endOfMatchDisclosures: [],
    finalPublicCheckpoint: checkpoint };
  await assert.rejects(verifyLiveAuditTranscript(transcript, webcrypto, {
    ...h.options, replayTranscript: async () => ({ finalPublicCheckpointHash: hash }),
  }), /must report verification of end-of-match disclosures/);
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

  assert.equal((await verifyEnvelopeOnlyTranscript(transcript)).valid, true);
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript({
      ...transcript,
      actions: [{
        ...transcript.actions[0],
        audit: {
          ...transcript.actions[0].audit,
          clock: {
            ...transcript.actions[0].audit.clock,
            elapsedMs: 0,
          },
        },
      }],
    }),
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
  assert.deepEqual(publicManifest.slotCommitments.map(({ slot }) => slot), [0, 1, 2]);
  assert.equal(manifest.slotSecrets[2].card, "Duress");
  assert.equal(publicManifest.decklistHash, undefined);
  assert.ok(publicManifest.decklistCommitment);
  assert.notEqual(publicManifest.decklistCommitment, manifest.decklistHash);
  assert.equal(JSON.stringify(publicManifest).includes("Lightning Bolt"), false);
  assert.equal(JSON.stringify(publicManifest).includes("Duress"), false);
  assert.equal(JSON.stringify(publicManifest).includes("salt-1"), false);
  assert.equal(JSON.stringify(publicManifest).includes(manifest.decklistHash), false);
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
  const sideboardOpening = await buildDeckSlotOpening({ manifest, slot: 2, card: "Duress" }, webcrypto);
  assert.equal(sideboardOpening.salt, "salt-2");
  assert.equal(await verifyCardOpeningAgainstManifest({ manifest: publicManifest, ...sideboardOpening }, webcrypto), true);
  assert.equal(await verifyCardOpeningAgainstManifest({
    manifest: publicManifest, ...sideboardOpening, card: "Counterspell",
  }, webcrypto), false);
});

test("public deck manifests use salted decklist commitments", async () => {
  const base = {
    matchId: "m-private-decklist",
    owner: 0,
    deck: ["Island", "Island"],
    saltForSlot: (slot) => `slot-salt-${slot}`,
  };
  const first = await buildPrivateDeckManifest({
    ...base,
    decklistSalt: "decklist-salt-one",
  }, webcrypto);
  const second = await buildPrivateDeckManifest({
    ...base,
    decklistSalt: "decklist-salt-two",
  }, webcrypto);
  assert.equal(first.decklistHash, second.decklistHash);
  assert.notEqual(first.decklistCommitment, second.decklistCommitment);
  assert.notEqual(first.commitmentRoot, second.commitmentRoot);
  assert.equal(publicDeckManifest(first).decklistHash, undefined);
  assert.equal(
    JSON.stringify(publicDeckManifest(first)).includes(first.decklistHash),
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

test("ziffle public openings must prove shuffled position to committed slot", async () => {
  const matchId = "m-ziffle-opening-proof";
  const playerKeys = [
    await createAuditSessionKey(webcrypto),
    await createAuditSessionKey(webcrypto),
  ];
  const playerPublicKeys = await Promise.all(
    playerKeys.map((keyPair) => exportAuditPublicKey(keyPair, webcrypto))
  );
  const ziffleKeys = playerPublicKeys.map((auditPublicKey, index) => ({
    player: index,
    publicKeyHex: `ziffle-key-${index}`,
    ownershipProofHex: `ziffle-owner-proof-${index}-${auditPublicKey.slice(0, 8)}`,
  }));
  const manifest = await buildPrivateDeckManifest({
    matchId,
    owner: 0,
    deck: ["Forest", "Lightning Bolt"],
    saltForSlot: (slot) => `slot-salt-${slot}`,
    decklistSalt: "decklist-salt",
  }, webcrypto);
  const command = { type: "priority_action", action_index: 0 };
  const baseOpening = {
    ...await buildDeckSlotOpening({
      manifest,
      slot: 1,
      card: "Lightning Bolt",
    }, webcrypto),
    position: 0,
    positionCommitment: "ziffle:test-ziffle-deck-0:0",
  };
  const ceremony = {
    owner: 0,
    deckCount: 2,
    context: matchId,
    keyContext: matchId,
    keys: ziffleKeys,
    steps: [0, 1].map((shuffler) => ({
      shuffler,
      deckHex: `test-deck-0-${shuffler}`,
      proofHex: `test-proof-0-${shuffler}`,
    })),
    deckHash: "test-ziffle-deck-0",
  };
  const tokens = ziffleKeys.map((key) => ({
    player: key.player,
    publicKeyHex: key.publicKeyHex,
    tokenHex: `token-${key.player}`,
    proofHex: `proof-${key.player}`,
  }));
  const proofOpening = {
    ...baseOpening,
    ziffleReveal: buildZiffleOpeningProof({
      opening: baseOpening,
      ceremony,
      position: 0,
      originalSlot: 1,
      positionCommitment: baseOpening.positionCommitment,
      tokens,
    }),
  };
  const buildTranscript = async (openings, shuffleProofs = []) => {
    const audit = await buildSignedActionEnvelope({
      keyPair: playerKeys[0],
      matchId,
      seq: 1,
      actor: 0,
      prevStateHash: "0".repeat(64),
      command,
      openings,
      shuffleProofs,
      publicCheckpointHash: "public-checkpoint-after-ziffle-opening",
    }, webcrypto);
    return buildCurrentProtocolTranscript({
      protocolVersion: 14,
      matchId,
      players: playerPublicKeys.map((auditPublicKey, index) => ({
        index,
        keyPair: playerKeys[index],
        auditPublicKey,
        ziffleKey: ziffleKeys[index],
        deckCount: index === 0 ? 2 : 0,
      })),
      deckAuditManifests: [publicDeckManifest(manifest)],
      actions: [{ seq: 1, actorIndex: 0, command, audit }],
    });
  };

  await assert.rejects(
    async () => verifyEnvelopeOnlyTranscript(await buildTranscript([baseOpening]), {
      verifyZiffleOpening: async () => ({ originalSlot: 1 }),
    }),
    /missing its position reveal proof/,
  );

  const transcript = await buildTranscript([proofOpening]);
  const report = await verifyEnvelopeOnlyTranscript(transcript, {
    verifyZiffleOpening: async ({ proof }) => ({ originalSlot: Number(proof.originalSlot) }),
  });
  assert.equal(report.valid, true);
  const compactProofOpening = {
    ...baseOpening,
    ziffleReveal: buildZiffleOpeningProof({
      opening: baseOpening,
      ceremony,
      position: 0,
      originalSlot: 1,
      positionCommitment: baseOpening.positionCommitment,
      tokens,
      compact: true,
    }),
  };
  assert.equal(compactProofOpening.ziffleReveal.keys, undefined);
  assert.equal(compactProofOpening.ziffleReveal.steps, undefined);
  assert.equal((await verifyEnvelopeOnlyTranscript(await buildTranscript([compactProofOpening]), {
    verifyZiffleOpening: async ({ proof }) => ({ originalSlot: Number(proof.originalSlot) }),
  })).valid, true);
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(transcript, {
      verifyZiffleOpening: async () => ({ originalSlot: 0 }),
    }),
    /reveals a different shuffle slot/,
  );

  const remapShuffleProof = await signTestZiffleCeremony({
    type: "ziffle_shuffle",
    requirementId: "shuffle-remap-1",
    owner: 0,
    zone: "library",
    epoch: 1,
    deckCount: 2,
    context: `${matchId}:action:1:shuffle:shuffle-remap-1:0:library`,
    keyContext: matchId,
    keys: ziffleKeys,
    steps: ceremony.steps,
    deckHash: "remapped-ziffle-deck-0",
    beforeOrder: [42, 77],
    afterOrder: [42, 77],
  }, playerKeys.map((keyPair, index) => ({ index, keyPair })));
  const remappedOpeningBase = {
    ...baseOpening,
    objectId: 42,
    position: 0,
    positionCommitment: "ziffle:remapped-ziffle-deck-0:0",
  };
  const remappedOpening = {
    ...remappedOpeningBase,
    ziffleReveal: buildZiffleOpeningProof({
      opening: remappedOpeningBase,
      ceremony: remapShuffleProof,
      position: 0,
      originalSlot: 1,
      shuffleOriginalSlot: 0,
      positionCommitment: remappedOpeningBase.positionCommitment,
      tokens,
    }),
  };
  const remappedTranscript = await buildTranscript([remappedOpening], [remapShuffleProof]);
  await assert.rejects(verifyEnvelopeOnlyTranscript(remappedTranscript, {
    verifyShuffleProof: async () => {},
    verifyZiffleOpening: async () => ({ originalSlot: 0 }),
  }), /immutable genesis origin/);

  const anchoredOpening = { ...remappedOpeningBase,
    originPosition: 0, originPositionCommitment: baseOpening.positionCommitment };
  anchoredOpening.ziffleReveal = buildZiffleOpeningProof({
    opening: anchoredOpening, ceremony, tokens, compact: true,
  });
  assert.equal((await verifyEnvelopeOnlyTranscript(
    await buildTranscript([anchoredOpening], [remapShuffleProof]), {
      verifyShuffleProof: async () => {},
      verifyZiffleOpening: async () => ({ originalSlot: 1 }),
    }
  )).valid, true);

  await assert.rejects(
    async () => verifyEnvelopeOnlyTranscript(
      await buildTranscript([remappedOpening], [{
        ...remapShuffleProof,
        afterOrder: [77, 42],
      }]),
      {
        verifyShuffleProof: async () => {},
        verifyZiffleOpening: async ({ proof }) => ({
          originalSlot: Number(proof.shuffleOriginalSlot ?? proof.originalSlot),
        }),
      },
    ),
    /immutable genesis origin/,
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

test("live audit transcript verifier requires postgame disclosures for encrypted private views", async () => {
  const matchId = "m-private-view-summary";
  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
  ]);
  const encryptionKeys = await Promise.all([
    createAuditEncryptionKey(webcrypto),
    createAuditEncryptionKey(webcrypto),
  ]);
  const players = await Promise.all(keys.map(async (keyPair, index) => ({
    index,
    keyPair,
    encryptionKeyPair: encryptionKeys[index],
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto),
    auditEncryptionPublicKey: await exportAuditEncryptionPublicKey(encryptionKeys[index], webcrypto),
  })));
  const ownerManifest = await buildPrivateDeckManifest({
    matchId,
    owner: 1,
    deck: ["Ponder"],
    saltForSlot: () => "private-view-summary-salt",
  }, webcrypto);
  const opening = await buildDeckSlotOpening({ manifest: ownerManifest, slot: 0 }, webcrypto);
  const requirementId = "private_open:1:library:0:7";
  const disclosurePayload = {
    type: "private_view_opening",
    matchId,
    requirementId,
    owner: 1,
    viewer: 0,
    zone: "library",
    objectId: 7,
    opening,
  };
  const encryptedOpening = await encryptPrivateAuditPayload({
    recipientPublicKey: players[0].auditEncryptionPublicKey,
    payload: disclosurePayload,
  }, webcrypto);
  const openingProof = {
    type: "encrypted_private_opening",
    requirementId,
    owner: 1,
    viewer: 0,
    zone: "library",
    objectId: 7,
    slot: 0,
    commitment: opening.commitment,
    encryptedOpening,
    disclosurePolicy: "postgame_or_dispute",
  };
  const summaryProof = {
    type: "encrypted_private_view",
    requirementId: "private_view:1:0:library:2",
    owner: 1,
    viewer: 0,
    zone: "library",
    count: 1,
    reason: "look_at_top_two",
    openingHashes: [encryptedOpening.plaintextHash],
    disclosurePolicy: "postgame_or_dispute",
  };
  summaryProof.materialHash = await sha256Hex(canonicalJson(summaryProof), webcrypto);
  const command = { type: "priority_action", action_index: 0 };
  const audit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId,
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    privateViewProofs: [openingProof, summaryProof],
    publicCheckpointHash: "public-checkpoint-after-private-view",
  }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId,
    players,
    deckAuditManifests: [null, ownerManifest],
    playerCount: 2,
    actions: [{
      seq: 1,
      actorIndex: 0,
      command,
      audit,
    }],
  });

  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(transcript),
    /missing its postgame disclosure/,
  );
  assert.equal(
    (await verifyEnvelopeOnlyTranscript(transcript, {
      requirePrivateViewDisclosures: false,
    })).valid,
    true,
  );

  const disclosedTranscript = {
    ...transcript,
    privateViewDisclosures: [{
      type: "private_view_opening_disclosure",
      matchId,
      seq: 1,
      requirementId,
      owner: 1,
      viewer: 0,
      zone: "library",
      objectId: 7,
      plaintextHash: encryptedOpening.plaintextHash,
      payload: disclosurePayload,
    }],
  };
  assert.equal((await verifyEnvelopeOnlyTranscript(disclosedTranscript)).valid, true);

  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript({
      ...disclosedTranscript,
      privateViewDisclosures: [{
        ...disclosedTranscript.privateViewDisclosures[0],
        payload: {
          ...disclosurePayload,
          opening: {
            ...disclosurePayload.opening,
            card: "Black Lotus",
          },
        },
      }],
    }),
    /missing its postgame disclosure/,
  );

  const badProof = {
    ...summaryProof,
    materialHash: "0".repeat(64),
  };
  const badAudit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId,
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    privateViewProofs: [openingProof, badProof],
    publicCheckpointHash: "public-checkpoint-after-private-view",
  }, webcrypto);
  const badTranscript = await buildCurrentProtocolTranscript({
    matchId,
    players,
    deckAuditManifests: [null, ownerManifest],
    privateViewDisclosures: disclosedTranscript.privateViewDisclosures,
    playerCount: 2,
    actions: [{
      seq: 1,
      actorIndex: 0,
      command,
      audit: badAudit,
    }],
  });
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(badTranscript),
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

  const report = await verifyEnvelopeOnlyTranscript(transcript);
  assert.equal(report.valid, true);
  assert.equal(report.verifiedActions, 1);
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript({
      ...transcript,
      actions: [{ ...transcript.actions[0], command: { type: "draw_cards" } }],
    }),
    /command mismatch/,
  );
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript({
      ...transcript,
      actions: [
        {
          ...transcript.actions[0],
          audit: { ...transcript.actions[0].audit, signer: 0 },
        },
      ],
    }),
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

  assert.equal((await verifyEnvelopeOnlyTranscript(transcript)).valid, true);
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
    () => verifyEnvelopeOnlyTranscript(tamperedTranscript),
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
    async () => verifyEnvelopeOnlyTranscript(await buildCurrentProtocolTranscript({
      matchId: "m-rng",
      players: [
        { index: 0, keyPair: actorKey, auditPublicKey: actorPublicKey },
        { index: 1, keyPair: otherKey, auditPublicKey: otherPublicKey },
      ],
      actions: [{ seq: 1, actorIndex: 0, command, audit }],
    })),
    /must include every player exactly once/,
  );
});

test("fair-random combined seed ignores transport request metadata", async () => {
  const commits = [
    { player: 0, requester: 0, requestId: "commit-a", commitmentHex: "commitment-0", signature: "sig-a" },
    { player: 1, requester: 0, requestId: "commit-b", commitmentHex: "commitment-1", signature: "sig-b" },
  ];
  const reveals = [
    {
      player: 0,
      requester: 0,
      requestId: "reveal-a",
      commitRequestId: "commit-a",
      nonceHex: "nonce-0",
      commitmentHex: "commitment-0",
      signature: "reveal-sig-a",
    },
    {
      player: 1,
      requester: 0,
      requestId: "reveal-b",
      commitRequestId: "commit-b",
      nonceHex: "nonce-1",
      commitmentHex: "commitment-1",
      signature: "reveal-sig-b",
    },
  ];
  const base = await fairRandomCombinedSeedHex({
    matchId: "m-rng-stable",
    seq: 3,
    requirementId: "rng-1",
    commits,
    reveals,
  }, webcrypto);
  const transportMutated = await fairRandomCombinedSeedHex({
    matchId: "m-rng-stable",
    seq: 3,
    requirementId: "rng-1",
    commits: commits.map((entry, index) => ({
      ...entry,
      requestId: `other-commit-${index}`,
      signature: `other-sig-${index}`,
    })),
    reveals: reveals.map((entry, index) => ({
      ...entry,
      requestId: `other-reveal-${index}`,
      commitRequestId: `other-commit-${index}`,
      signature: `other-reveal-sig-${index}`,
    })),
  }, webcrypto);
  assert.equal(base, transportMutated);

  const changedContribution = await fairRandomCombinedSeedHex({
    matchId: "m-rng-stable",
    seq: 3,
    requirementId: "rng-1",
    commits,
    reveals: reveals.map((entry, index) =>
      index === 0 ? { ...entry, nonceHex: "different-nonce" } : entry
    ),
  }, webcrypto);
  assert.notEqual(base, changedContribution);
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
    combinedSeedHex: await fairRandomCombinedSeedHex({
      matchId,
      seq,
      requirementId,
      commits,
      reveals,
    }, webcrypto),
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

  assert.equal((await verifyEnvelopeOnlyTranscript(await buildCurrentProtocolTranscript({
    matchId,
    players: playerPublicKeys.map((auditPublicKey, index) => ({
      index,
      keyPair: playerKeys[index],
      auditPublicKey,
    })),
    actions: [{ seq, actorIndex: 0, command, audit }],
  }))).valid, true);
});

test("live audit transcript verifier requires a shuffle-proof verifier", async () => {
  const playerKeys = [
    await createAuditSessionKey(webcrypto),
    await createAuditSessionKey(webcrypto),
  ];
  const playerPublicKeys = await Promise.all(
    playerKeys.map((keyPair) => exportAuditPublicKey(keyPair, webcrypto))
  );
  const ziffleKeys = playerKeys.map((_, index) => ({
    player: index,
    publicKeyHex: `shuffle-ziffle-key-${index}`,
    ownershipProofHex: `shuffle-ziffle-proof-${index}`,
  }));
  const command = { type: "priority_action", action_index: 0 };
  const shuffleProof = await signTestZiffleCeremony({
    type: "ziffle_shuffle",
    requirementId: "shuffle-1",
    owner: 0,
    zone: "library",
    epoch: 1,
    deckCount: 2,
    context: "m-shuffle:action:1:shuffle:shuffle-1:0:library",
    keyContext: "m-shuffle",
    keys: ziffleKeys,
    steps: [0, 1].map((shuffler) => ({
      shuffler,
      deckHex: `shuffle-deck-${shuffler}`,
      proofHex: `shuffle-proof-${shuffler}`,
    })),
    deckHash: "deck-hash",
    beforeOrder: [1001, 1002],
    afterOrder: [1002, 1001],
  }, playerKeys.map((keyPair, index) => ({ index, keyPair })));
  const buildTranscriptForProof = async (proof) => {
    const audit = await buildSignedActionEnvelope({
      keyPair: playerKeys[0],
      matchId: "m-shuffle",
      seq: 1,
      actor: 0,
      prevStateHash: "0".repeat(64),
      command,
      shuffleProofs: [proof],
      publicCheckpointHash: "public-checkpoint-after-shuffle",
    }, webcrypto);
    return buildCurrentProtocolTranscript({
      protocolVersion: 14,
      matchId: "m-shuffle",
      players: playerPublicKeys.map((auditPublicKey, index) => ({
        index,
        keyPair: playerKeys[index],
        auditPublicKey,
        ziffleKey: ziffleKeys[index],
      })),
      actions: [{ seq: 1, actorIndex: 0, command, audit }],
    });
  };
  const transcript = await buildTranscriptForProof(shuffleProof);

  await assert.rejects(
    () => verifyLiveAuditTranscript(transcript, webcrypto),
    /no verifier was provided/,
  );
  const report = await verifyEnvelopeOnlyTranscript(transcript, {
    verifyShuffleProof: async (proof) => {
      assert.equal(proof.deckHash, "deck-hash");
    },
  });
  assert.equal(report.valid, true);

  const rematerializedOrderTranscript = await buildTranscriptForProof({
    ...shuffleProof,
    afterOrder: [2002, 2001],
  });
  const rematerializedReport = await verifyEnvelopeOnlyTranscript(rematerializedOrderTranscript, {
    verifyShuffleProof: async (proof) => {
      assert.equal(proof.deckHash, "deck-hash");
    },
  });
  assert.equal(rematerializedReport.valid, true);

  const missingOrderTranscript = await buildTranscriptForProof({
    ...shuffleProof,
    beforeOrder: undefined,
    afterOrder: undefined,
  });
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(missingOrderTranscript, {
      verifyShuffleProof: async () => {},
    }),
    /missing its object order/,
  );

  const wrongEpochTranscript = await buildTranscriptForProof({
    ...shuffleProof,
    epoch: 2,
  });
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(wrongEpochTranscript, {
      verifyShuffleProof: async () => {},
    }),
    /different action/,
  );

  const badRosterTranscript = await buildTranscriptForProof({
    ...shuffleProof,
    keys: ziffleKeys.map((key, index) => ({
      ...key,
      publicKeyHex: index === 0 ? "attacker-key" : key.publicKeyHex,
    })),
  });
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(badRosterTranscript, {
      verifyShuffleProof: async () => {},
    }),
    /signed ziffle key roster/,
  );

  const badContextTranscript = await buildTranscriptForProof({
    ...shuffleProof,
    context: "attacker-match:action:1:shuffle:shuffle-1:0:library",
    keyContext: "attacker-match",
  });
  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(badContextTranscript, {
      verifyShuffleProof: async () => {},
    }),
    /different match/,
  );
});

test("match genesis and resync envelopes bind roster, actions and final state", async () => {
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
      seedCommitment: await matchSeedCommitment(testSeedNonce(index), webcrypto),
    }, webcrypto);
    return player;
  }));
  const ziffleKeys = players.map((player) => player.ziffleKey);
  const seedReveals = players.map((player) => ({ seat: player.index, nonce: testSeedNonce(player.index) }));
  const signers = playerKeys.map((keyPair, index) => ({ index, keyPair }));
  const match = {
    protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
    auditMatchId: "m",
    lobbyId: "m",
    hostPeerId: "peer-host",
    format: "normal",
    startingLife: 20,
    openingHandSize: 7,
    seed: await deriveMatchSeedFromReveals({ matchId: "m", reveals: seedReveals }, webcrypto),
    seedReveals,
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
    ziffleCeremonies: await Promise.all(players.map((player) => signTestZiffleCeremony({
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
    }, signers))),
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
  await assert.rejects(
    () => verifySignedMatchGenesis({
      ...match,
      hostPeerId: "peer-1",
    }, webcrypto),
    /genesis payload hash mismatch|genesis signature|host peer id/,
  );
  const peerIdTamperedMatch = cloneTestPayload(match);
  peerIdTamperedMatch.players[1].peerId = "peer-attacker";
  peerIdTamperedMatch.genesis = await buildSignedMatchGenesis({
    keyPair: hostKey,
    match: peerIdTamperedMatch,
    hostSeat: 0,
  }, webcrypto);
  await assert.rejects(
    () => verifySignedMatchGenesis(peerIdTamperedMatch, webcrypto),
    /genesis payload hash mismatch/,
  );

  const actions = [];
  const envelope = await buildSignedResyncEnvelope({
    keyPair: hostKey,
    matchId: "m",
    signer: 0,
    lastSequence: 0,
    finalStateHash: "0".repeat(64),
    actions,
  }, webcrypto);
  const hostPublicCryptoKey = await importAuditPublicKey(hostPublicKey, webcrypto);
  assert.equal(
    (await verifySignedResyncEnvelope({
      envelope,
      publicKey: hostPublicCryptoKey,
        actions,
    }, webcrypto)).valid,
    true,
  );
  await assert.rejects(
    () => verifySignedResyncEnvelope({
      envelope: { ...envelope, finalStateHash: "f".repeat(64) },
      publicKey: hostPublicCryptoKey,
      actions,
    }, webcrypto),
    /signature is invalid/,
  );

  const signedAction = {
    seq: 1,
    actorIndex: 0,
    command: { type: "priority_action", action_ref: { kind: "test_priority_action" } },
    label: "action 1",
    audit: { signature: "sig-1", nextStateHash: "1".repeat(64) },
  };
  const actionEnvelope = await buildSignedResyncEnvelope({
    keyPair: hostKey,
    matchId: "m",
    signer: 0,
    lastSequence: 1,
    finalStateHash: "1".repeat(64),
    actions: [signedAction],
  }, webcrypto);
  assert.equal(
    (await verifySignedResyncEnvelope({
      envelope: actionEnvelope,
      publicKey: hostPublicCryptoKey,
        actions: [signedAction],
    }, webcrypto)).valid,
    true,
  );
  await assert.rejects(
    () => buildSignedResyncEnvelope({
      keyPair: hostKey,
      matchId: "m",
      signer: 0,
      lastSequence: 2,
      finalStateHash: "1".repeat(64),
        actions: [signedAction],
    }, webcrypto),
    /last sequence/,
  );
  await assert.rejects(
    () => verifySignedResyncEnvelope({
      envelope: actionEnvelope,
      publicKey: hostPublicCryptoKey,
        actions: [],
    }, webcrypto),
    /last sequence/,
  );
});

test("open decklist match genesis verifies main-deck and sideboard slot openings", async () => {
  const matchId = "open-decklists";
  const playerKeys = [
    await createAuditSessionKey(webcrypto),
    await createAuditSessionKey(webcrypto),
  ];
  const encryptionKeys = [
    await createAuditEncryptionKey(webcrypto),
    await createAuditEncryptionKey(webcrypto),
  ];
  const publicKeys = await Promise.all(
    playerKeys.map((keyPair) => exportAuditPublicKey(keyPair, webcrypto))
  );
  const encryptionPublicKeys = await Promise.all(
    encryptionKeys.map((keyPair) => exportAuditEncryptionPublicKey(keyPair, webcrypto))
  );
  const decks = [
    ["Island", "Mountain"],
    ["Forest", "Forest"],
  ];
  const sideboards = [["Duress"], ["Swamp", "Plains"]];
  const manifests = await Promise.all(decks.map((deck, index) =>
    buildPrivateDeckManifest({
      matchId,
      owner: index,
      deck,
      sideboard: sideboards[index],
      saltForSlot: (slot) => `open-decklist-salt-${index}-${slot}`,
    }, webcrypto)
  ));
  const buildPlayers = async (slotOpeningOverrides = {}) => Promise.all(
    playerKeys.map(async (keyPair, index) => {
      const openings = await Promise.all([...decks[index], ...sideboards[index]].map((card, slot) =>
        buildDeckSlotOpening({ manifest: manifests[index], slot, card }, webcrypto)
      ));
      const player = {
        peerId: `peer-${index}`,
        name: `Player ${index + 1}`,
        index,
        auditPublicKey: publicKeys[index],
        auditEncryptionPublicKey: encryptionPublicKeys[index],
        deckAuditManifest: publicDeckManifest(manifests[index]),
        ziffleKey: {
          player: index,
          publicKeyHex: `open-ziffle-key-${index}`,
          ownershipProofHex: `open-ziffle-proof-${index}`,
        },
        deck: decks[index],
        sideboard: sideboards[index],
        commanders: [],
        deckSlotOpenings: slotOpeningOverrides[index] || openings,
        deckCount: decks[index].length,
        sideboardCount: sideboards[index].length,
        commanderCount: 0,
      };
      player.playerGenesisSignature = await buildSignedPlayerGenesis({
        keyPair,
        matchId,
        protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
        timeoutMs: 300000,
        player,
        seedCommitment: await matchSeedCommitment(testSeedNonce(index), webcrypto),
      }, webcrypto);
      return player;
    })
  );
  const buildMatch = async (players) => {
    const ziffleKeys = players.map((player) => player.ziffleKey);
    const seedReveals = players.map((player) => ({ seat: player.index, nonce: testSeedNonce(player.index) }));
    const signers = playerKeys.map((keyPair, index) => ({ index, keyPair }));
    const match = {
      protocolVersion: CURRENT_AUDIT_PROTOCOL_VERSION,
      auditMatchId: matchId,
      lobbyId: matchId,
      hostPeerId: "peer-0",
      format: "normal",
      openDecklists: true,
      startingLife: 20,
      openingHandSize: 7,
      seed: await deriveMatchSeedFromReveals({ matchId, reveals: seedReveals }, webcrypto),
      seedReveals,
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
      ziffleCeremonies: await Promise.all(players.map((player) => signTestZiffleCeremony({
        owner: player.index,
        deckCount: player.deckCount,
        context: matchId,
        keyContext: matchId,
        keys: ziffleKeys,
        steps: players.map((shuffler) => ({
          shuffler: shuffler.index,
          deckHex: `open-deck-${player.index}-${shuffler.index}`,
          proofHex: `open-proof-${player.index}-${shuffler.index}`,
        })),
        deckHash: `open-deck-hash-${player.index}`,
      }, signers))),
    };
    match.genesis = await buildSignedMatchGenesis({
      keyPair: playerKeys[0],
      match,
      hostSeat: 0,
    }, webcrypto);
    return match;
  };

  const match = await buildMatch(await buildPlayers());
  assert.equal((await verifySignedMatchGenesis(match, webcrypto)).valid, true);

  const badOpening = {
    ...(await buildDeckSlotOpening({ manifest: manifests[0], slot: 0, card: "Island" }, webcrypto)),
    card: "Swamp",
  };
  const tamperedMatch = await buildMatch(await buildPlayers({ 0: [
    badOpening,
    await buildDeckSlotOpening({ manifest: manifests[0], slot: 1, card: "Mountain" }, webcrypto),
    await buildDeckSlotOpening({ manifest: manifests[0], slot: 2, card: "Duress" }, webcrypto),
  ] }));
  await assert.rejects(
    () => verifySignedMatchGenesis(tamperedMatch, webcrypto),
    /does not match the declared card|does not match its commitment/,
  );

  const validOpenings = (await buildPlayers())[0].deckSlotOpenings;
  for (const [openings, error] of [
    [validOpenings.slice(0, 2), /missing committed slot openings/],
    [[...validOpenings.slice(0, 2), { ...validOpenings[2], card: "Counterspell" }], /does not match the declared card/],
    [[...validOpenings.slice(0, 2), { ...validOpenings[2], salt: "wrong-sideboard-salt" }], /does not match its commitment/],
    [[...validOpenings.slice(0, 2), validOpenings[1]], /duplicate slot/],
  ]) {
    const invalid = await buildMatch(await buildPlayers({ 0: openings }));
    await assert.rejects(() => verifySignedMatchGenesis(invalid, webcrypto), error);
  }
});

test("live audit transcript verifier rejects cross-match replayed actions", async () => {
  const keys = await Promise.all([
    createAuditSessionKey(webcrypto),
    createAuditSessionKey(webcrypto),
  ]);
  const players = await Promise.all(keys.map(async (keyPair, index) => ({
    index,
    keyPair,
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto),
  })));
  const command = {
    type: "priority_action",
    action_ref: { kind: "test_priority_action", actor: 0 },
  };
  const audit = await buildSignedActionEnvelope({
    keyPair: keys[0],
    matchId: "m-other-match",
    seq: 1,
    actor: 0,
    prevStateHash: "0".repeat(64),
    command,
    publicCheckpointHash: "public-checkpoint-cross-match",
  }, webcrypto);
  const transcript = await buildCurrentProtocolTranscript({
    matchId: "m-cross-match",
    players,
    actions: [{
      seq: 1,
      actorIndex: 0,
      command,
      audit,
    }],
  });

  await assert.rejects(
    () => verifyEnvelopeOnlyTranscript(transcript),
    /belongs to a different match/,
  );
});

test("resync continuity rejects rollback and divergent local history", () => {
  const action1 = {
    seq: 1,
    actorIndex: 0,
    command: { type: "priority_action", action_ref: { kind: "test_priority_action", sequence: 0 } },
    label: "action 1",
    audit: { signature: "sig-1", nextStateHash: "1".repeat(64) },
  };
  const action2 = {
    seq: 2,
    actorIndex: 1,
    command: { type: "priority_action", action_ref: { kind: "test_priority_action", sequence: 1 } },
    label: "action 2",
    audit: { signature: "sig-2", nextStateHash: "2".repeat(64) },
  };
  const action3 = {
    seq: 3,
    actorIndex: 0,
    command: { type: "priority_action", action_ref: { kind: "test_priority_action", sequence: 2 } },
    label: "action 3",
    audit: { signature: "sig-3", nextStateHash: "3".repeat(64) },
  };

  assert.deepEqual(
    assertResyncActionsExtendLocalTranscript({
      actionEntries: [action1, action2, action3],
      localActions: [action1, action2],
      localLastSequence: 2,
    }),
    {
      localSequence: 2,
      finalSequence: 3,
      checkedActions: 2,
    },
  );
  assert.throws(
    () => assertResyncActionsExtendLocalTranscript({
      actionEntries: [action1],
      localActions: [action1, action2],
      localLastSequence: 2,
    }),
    /older than the local transcript/,
  );
  assert.throws(
    () => assertResyncActionsExtendLocalTranscript({
      actionEntries: [action1, { ...action2, label: "tampered label" }],
      localActions: [action1, action2],
      localLastSequence: 2,
    }),
    /does not match local transcript/,
  );
  assert.throws(
    () => assertResyncActionsExtendLocalTranscript({
      actionEntries: [action1, action2],
      localActions: [action2],
      localLastSequence: 2,
    }),
    /incomplete/,
  );
});

test('v15 signed audit accepts private subset chains and current-epoch openings for the original manifest', async () => {
  const matchId = 'private-epoch-offline-audit';
  const encryptionKeys = await Promise.all([0, 1].map(() => createAuditEncryptionKey(webcrypto)));
  const keyPairs = await Promise.all([0, 1].map(() => createAuditSessionKey(webcrypto)));
  const players = await Promise.all(keyPairs.map(async (keyPair, index) => ({ index, keyPair,
    auditPublicKey: await exportAuditPublicKey(keyPair, webcrypto), deckCount: index === 0 ? 8 : 0,
    auditEncryptionPublicKey: await exportAuditEncryptionPublicKey(encryptionKeys[index], webcrypto),
    ziffleKey: { player: index, publicKeyHex: `private-key-${index}`, ownershipProofHex: `private-owner-${index}` } })));
  const manifest = await buildPrivateDeckManifest({ matchId, owner: 0,
    deck: ['Forest', 'Island', 'Swamp', 'Mountain', 'Plains', 'Grizzly Bears', 'Lightning Bolt', 'Nexus of Fate'] }, webcrypto);
  const base = await buildCurrentProtocolTranscript({ matchId, players, actions: [],
    deckAuditManifests: [publicDeckManifest(manifest)] });
  const genesis = base.match.ziffleCeremonies[0];
  const keys = base.match.ziffleKeys;
  // Every tampered proof below is re-signed so each case reaches the check it
  // targets instead of stopping at the shuffle-step signatures.
  const signers = keyPairs.map((keyPair, index) => ({ index, keyPair }));
  const unsignedSteps = [0, 1].map(shuffler => ({ shuffler, deckHex: `private-deck-${shuffler}`, proofHex: `private-proof-${shuffler}` }));
  const sign = proof => signTestZiffleCeremony({ ...proof, steps: proof.steps.map(({ signature, ...step }) => step) }, signers);
  const first = await sign({ type: 'ziffle_shuffle', requirementId: 'first', owner: 0, zone: 'library', epoch: 1,
    deckCount: 4, context: `${matchId}:action:1:shuffle:first:0:library`, keyContext: matchId,
    deckHash: 'private-first', keys, steps: unsignedSteps,
    inputDeck: buildZiffleInputDeck([genesis], [0, 2, 4, 7].map(position => `ziffle:${genesis.deckHash}:${position}`)) });
  const second = await sign({ type: 'ziffle_shuffle', requirementId: 'second', owner: 0, zone: 'library', epoch: 2,
    deckCount: 3, context: `${matchId}:action:2:shuffle:second:0:library`, keyContext: matchId,
    deckHash: 'private-second', keys, steps: unsignedSteps,
    inputDeck: buildZiffleInputDeck([genesis, first], ['ziffle:private-first:0', 'ziffle:private-first:2', `ziffle:${genesis.deckHash}:1`]) });
  const plainOpening = { ...await buildDeckSlotOpening({ manifest, slot: 7 }, webcrypto),
    position: 1, positionCommitment: 'ziffle:private-second:1',
    originPosition: 1, originPositionCommitment: 'ziffle:private-second:1', timing: 'post' };
  const tokens = keys.map(key => ({ player: key.player, publicKeyHex: key.publicKeyHex,
    tokenHex: `token-${key.player}`, proofHex: `token-proof-${key.player}` }));
  const command = { type: 'priority_action', action_ref: { kind: 'pass_priority' } };
  const build = async (proofs, opening, privateMaterial = null) => {
    const actions = [];
    let previous = '0'.repeat(64);
    for (const [index, proof] of proofs.entries()) {
      const audit = await buildSignedActionEnvelope({ keyPair: keyPairs[0], matchId, seq: index + 1,
        actor: 0, prevStateHash: previous, command, shuffleProofs: [proof],
        openings: index === 1 && opening ? [opening] : [],
        privateViewProofs: index === 1 && privateMaterial ? [privateMaterial.proof] : [],
        publicCheckpointHash: `checkpoint-${index + 1}` }, webcrypto);
      actions.push({ seq: index + 1, actorIndex: 0, command, audit });
      previous = audit.nextStateHash;
    }
    return buildCurrentProtocolTranscript({ matchId, players, actions, deckAuditManifests: [publicDeckManifest(manifest)],
      privateViewDisclosures: privateMaterial ? [privateMaterial.disclosure] : [] });
  };
  const verifiedRoot = proof => ({ deckCount: proof.deckCount, deckHash: proof.deckHash,
    rootDeckHash: genesis.deckHash, rootContext: matchId, universeCount: 8 });
  for (const compact of [false, true]) {
    const opening = { ...plainOpening,
      ziffleReveal: buildZiffleOpeningProof({ opening: plainOpening, ceremony: second, tokens, compact }) };
    assert.deepEqual(opening.ziffleReveal.inputDeck, compact ? undefined : second.inputDeck,
      'compact openings reuse the accepted graph without retransmitting it for every card');
    const calls = [];
    const options = {
      verifyShuffleProof: async proof => { calls.push(['shuffle', proof.deckHash]); return verifiedRoot(proof); },
      verifyZiffleOpening: async ({ ceremony, proof }) => {
        assert.equal(ceremony.deckCount, 3);
        assert.equal(ceremony.inputDeck.universeCount, 8);
        assert.deepEqual(ceremony.inputDeck, second.inputDeck);
        assert.equal(proof.originalSlot, 7, 'manifest label may exceed the current collection size');
        calls.push(['opening', ceremony.deckHash]);
        return { originalSlot: 7 };
      },
    };
    assert.equal((await verifyEnvelopeOnlyTranscript(await build([first, second], opening), options)).valid, true);
    assert.deepEqual(calls, [['shuffle', 'private-first'], ['shuffle', 'private-second'], ['opening', 'private-second']]);
    const payload = { type: 'private_view_opening', matchId, requirementId: 'private_open:0:library:7:99',
      owner: 0, viewer: 1, zone: 'library', objectId: 99, opening: { ...opening, objectId: 99, timing: 'private' } };
    const privateMaterial = {
      proof: { type: 'encrypted_private_opening', owner: 0, viewer: 1, zone: 'library', objectId: 99,
        position: 1, positionCommitment: opening.positionCommitment,
        encryptedOpening: await encryptPrivateAuditPayload({ recipientPublicKey: players[1].auditEncryptionPublicKey, payload }, webcrypto) },
      disclosure: { seq: 2, matchId, payload },
    };
    calls.length = 0;
    assert.equal((await verifyEnvelopeOnlyTranscript(await build([first, second], null, privateMaterial), options)).valid, true);
    assert.deepEqual(calls, [['shuffle', 'private-first'], ['shuffle', 'private-second'], ['opening', 'private-second']],
      'postgame private disclosures receive the same ciphertext/manifest proof checks as public openings');
    const wrongPosition = structuredClone(privateMaterial);
    wrongPosition.proof.position = 0;
    await assert.rejects(verifyEnvelopeOnlyTranscript(await build([first, second], null, wrongPosition), options), /position does not match/);
    for (const [field, value] of [['slot', 7], ['commitment', opening.commitment], ['requirementId', payload.requirementId], ['card', opening.card]]) {
      const leakingHeader = structuredClone(privateMaterial);
      leakingHeader.proof[field] = value;
      const leakingTranscript = await build([first, second], null, leakingHeader);
      await assert.rejects(verifyEnvelopeOnlyTranscript(leakingTranscript, options), /exposes private/);
      await assert.rejects(verifyEnvelopeOnlyTranscript(leakingTranscript, { ...options, requirePrivateViewDisclosures: false }), /exposes private/);
    }
    await assert.rejects(verifyEnvelopeOnlyTranscript(await build([first, second], null, privateMaterial), {
      ...options, verifyZiffleOpening: async () => { throw new Error('invalid private disclosure token proof'); },
    }), /invalid private disclosure token proof/);

    const staleAnchor = { ...plainOpening, originPosition: 0,
      originPositionCommitment: `ziffle:${genesis.deckHash}:0` };
    staleAnchor.ziffleReveal = buildZiffleOpeningProof({ opening: staleAnchor, ceremony: genesis, tokens, compact });
    await assert.rejects(verifyEnvelopeOnlyTranscript(await build([first, second], staleAnchor), options), /origin across a private shuffle/);
    const wrongHistory = structuredClone(second);
    wrongHistory.inputDeck.epochs[0].steps[0].deckHex = 'different-signed-owner-root';
    await assert.rejects(verifyEnvelopeOnlyTranscript(await build([first, await sign(wrongHistory)], opening), options), /accepted signed transcript/);
    const consumedSource = structuredClone(second);
    consumedSource.inputDeck.sources[0] = { epoch: 0, position: 0 };
    await assert.rejects(verifyEnvelopeOnlyTranscript(await build([first, await sign(consumedSource)], opening), options), /already consumed/);
    const alteredOpening = structuredClone(opening);
    alteredOpening.ziffleReveal.inputDeck = structuredClone(second.inputDeck);
    alteredOpening.ziffleReveal.inputDeck.epochs[0].steps[0].proofHex = 'another-history';
    await assert.rejects(verifyEnvelopeOnlyTranscript(await build([first, second], alteredOpening), options), /accepted encrypted input history/);
    await assert.rejects(verifyEnvelopeOnlyTranscript(await build([first, second], opening), {
      ...options, verifyShuffleProof: async proof => ({ ...verifiedRoot(proof), rootDeckHash: 'other-owner' }),
    }), /signed initial deck/);
  }
  const legacy = { ...first, beforeOrder: [1, 2, 3, 4], afterOrder: [4, 3, 2, 1] };
  delete legacy.inputDeck;
  await assert.rejects(verifyEnvelopeOnlyTranscript(await build([await sign(legacy)]), {
    verifyShuffleProof: async proof => verifiedRoot(proof),
  }), /linked legacy shuffle/);
});

test("private views of a face-down exiled card its owner may not see are produced by the viewer", () => {
  // Bob (1) exiled Alice's (0) card face down with Gonti: Alice does not know it.
  const blind = { id: "private_open:0:face_down_exile:5:212", type: "private_open", owner: 0, viewer: 1,
    zone: "face_down_exile", slot: 5, objectId: 212, commitment: "commitment-5", ownerBlind: true };
  const window = { id: "private_view:1:0:face_down_exile:1", type: "private_view_window", owner: 0, viewer: 1,
    zone: "face_down_exile", count: 1, ownerBlind: true };
  assert.equal(cryptoMaterialResponsibleSeat(blind), 1);
  assert.equal(cryptoMaterialResponsibleSeat(window), 1);
  // Alice exiled it herself (Kheru Mind-Eater from her hand): she knows it.
  const known = { ...blind, ownerBlind: undefined };
  assert.equal(cryptoMaterialResponsibleSeat(known), 0);
  assert.equal(cryptoMaterialResponsibleSeat({ ...blind, type: "public_open", viewer: undefined }), 0);

  // Alice's engine never asks herself for it, and refuses to answer a request for it.
  assert.deepEqual(authorizeCryptoMaterialRequestRequirements({
    localSeat: 0, requestedRequirements: [], previewedRequirements: [blind, window] }), []);
  assert.deepEqual(authorizeCryptoMaterialRequestRequirements({
    localSeat: 1, requestedRequirements: [blind], previewedRequirements: [blind, window] }), [blind]);
});
