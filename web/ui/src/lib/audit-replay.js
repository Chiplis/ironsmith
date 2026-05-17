import { publicCheckpointHash } from "./multiplayer-audit.js";
import { resolveSyncedCommand } from "./sync-commands.js";

const DEFAULT_OPENING_HAND_SIZE = 7;

function clonePayload(value) {
  if (value == null) return value;
  return JSON.parse(JSON.stringify(value));
}

function requiredGameMethod(game, name) {
  const method = game?.[name];
  if (typeof method !== "function") {
    throw new Error(`Game engine cannot replay transcript: missing ${name}`);
  }
  return method.bind(game);
}

function optionalGameMethod(game, name) {
  const method = game?.[name];
  return typeof method === "function" ? method.bind(game) : null;
}

function transcriptPlayers(match) {
  return Array.isArray(match?.players) ? match.players : [];
}

function replayPlayerNames(match) {
  const players = transcriptPlayers(match);
  if (players.length === 0 && Array.isArray(match?.decks)) {
    return match.decks.map((_, index) => `Player ${index + 1}`);
  }
  return players.map((player, index) =>
    String(player?.name || player?.displayName || `Player ${index + 1}`)
  );
}

function replayDecks(match) {
  if (Array.isArray(match?.decks) && match.decks.length > 0) {
    return clonePayload(match.decks);
  }
  return replayPlayerNames(match).map(() => []);
}

function replayMatchConfig(match = {}) {
  return {
    playerNames: replayPlayerNames(match),
    startingLife: Number(match.startingLife || 20),
    seed: match.seed ?? "",
    format: String(match.format || "normal"),
    decks: replayDecks(match),
    sideboards: clonePayload(match.sideboards),
    commanders: clonePayload(match.commanders),
    hiddenDeckManifests: clonePayload(
      match.runtimeHiddenDeckManifests
        || match.hiddenDeckManifests
        || []
    ),
    openingHandSize: match.openingHandSize == null
      ? DEFAULT_OPENING_HAND_SIZE
      : Number(match.openingHandSize),
  };
}

function normalizedPerspective(perspectiveIndex, match) {
  const playerCount = replayPlayerNames(match).length;
  const perspective = Number(perspectiveIndex);
  if (
    Number.isInteger(perspective)
    && perspective >= 0
    && (playerCount === 0 || perspective < playerCount)
  ) {
    return perspective;
  }
  return 0;
}

function actionSeq(entry, fallback) {
  const seq = Number(entry?.audit?.seq ?? entry?.seq ?? fallback);
  return Number.isSafeInteger(seq) ? seq : fallback;
}

function normalizeShuffleOrder(value) {
  return (Array.isArray(value) ? value : [])
    .map((entry) => Number(entry))
    .filter((entry) => Number.isSafeInteger(entry) && entry >= 0);
}

function requirementType(requirement) {
  return String(requirement?.type || requirement?.requirement_type || "");
}

function requirementId(requirement) {
  return String(
    requirement?.id
      || requirement?.requirementId
      || requirement?.requirement_id
      || ""
  );
}

function shuffleProofMatchesRequirement(proof, requirement) {
  if (!proof || !requirement) return false;
  const proofRequirementId = String(proof?.requirementId || proof?.requirement_id || "");
  const reqId = requirementId(requirement);
  if (proofRequirementId && reqId) return proofRequirementId === reqId;
  return (
    Number(proof?.owner) === Number(requirement.owner)
    && String(proof?.zone || "library") === String(requirement.zone || "library")
  );
}

function rngRevealMatchesRequirement(reveal, requirement) {
  if (!reveal || !requirement) return false;
  const revealRequirementId = String(reveal?.requirementId || reveal?.requirement_id || "");
  const reqId = requirementId(requirement);
  return Boolean(revealRequirementId && reqId && revealRequirementId === reqId);
}

function seedEntriesForRequirements(requirements = [], audit = {}) {
  const seeds = [];
  const usedProofs = new Set();
  const usedReveals = new Set();
  const shuffleProofs = Array.isArray(audit.shuffleProofs) ? audit.shuffleProofs : [];
  const rngReveals = Array.isArray(audit.rngReveals) ? audit.rngReveals : [];
  for (const requirement of requirements || []) {
    const type = requirementType(requirement);
    if (type === "verifiable_shuffle") {
      const proof = shuffleProofs.find((entry) =>
        !usedProofs.has(entry) && shuffleProofMatchesRequirement(entry, requirement)
      );
      if (proof?.deckHash) {
        usedProofs.add(proof);
        seeds.push(String(proof.deckHash));
      }
    } else if (type === "fair_random") {
      const reveal = rngReveals.find((entry) =>
        !usedReveals.has(entry) && rngRevealMatchesRequirement(entry, requirement)
      );
      if (reveal?.combinedSeedHex) {
        usedReveals.add(reveal);
        seeds.push(String(reveal.combinedSeedHex));
      }
    }
  }
  return seeds;
}

function fallbackSeedEntries(audit = {}) {
  const seeds = [];
  for (const proof of audit.shuffleProofs || []) {
    if (proof?.deckHash) seeds.push(String(proof.deckHash));
  }
  for (const reveal of audit.rngReveals || []) {
    if (reveal?.combinedSeedHex) seeds.push(String(reveal.combinedSeedHex));
  }
  return seeds;
}

async function previewCryptoRequirements(game, command) {
  const preview = optionalGameMethod(game, "previewCryptoRequirements");
  if (!preview) return [];
  const requirements = await preview(command);
  return Array.isArray(requirements) ? requirements : [];
}

async function injectTranscriptSeeds(game, requirements, audit) {
  const seeds = seedEntriesForRequirements(requirements, audit);
  const resolvedSeeds = seeds.length > 0 ? seeds : fallbackSeedEntries(audit);
  if (resolvedSeeds.length === 0) return;
  const inject = optionalGameMethod(game, "injectTranscriptRandomSeeds");
  if (!inject) {
    throw new Error("Game engine cannot replay transcript: missing injectTranscriptRandomSeeds");
  }
  await inject({ seeds: resolvedSeeds });
}

async function revealOpeningWithGame(game, opening) {
  if (!opening || opening.owner == null || opening.slot == null || !opening.card) return;
  const owner = Number(opening.owner);
  const slot = Number(opening.slot);
  const cardName = String(opening.card);
  const commitment = opening.commitment ? String(opening.commitment) : undefined;
  const position = opening.position ?? opening.publicPosition;
  const positionCommitment = opening.positionCommitment || opening.position_commitment;

  const revealPosition = optionalGameMethod(game, "revealHiddenPosition");
  if (position != null && revealPosition) {
    try {
      await revealPosition({
        owner,
        position: Number(position),
        originalSlot: slot,
        cardName,
        positionCommitment: positionCommitment ? String(positionCommitment) : undefined,
        commitment,
        recomputeDecision: true,
      });
      return;
    } catch (err) {
      const message = String(err?.message || err || "");
      if (!message.includes("not present") && !message.includes("not a hidden")) {
        throw err;
      }
    }
  }

  const revealObject = optionalGameMethod(game, "revealHiddenObject");
  const objectId = opening.objectId ?? opening.object_id;
  if (objectId != null && revealObject) {
    try {
      await revealObject({
        objectId: Number(objectId),
        slot,
        cardName,
        commitment,
        recomputeDecision: true,
      });
      return;
    } catch (err) {
      const message = String(err?.message || err || "");
      if (!message.includes("not present") && !message.includes("not a hidden")) {
        throw err;
      }
    }
  }

  const revealSlot = optionalGameMethod(game, "revealHiddenSlot");
  if (revealSlot) {
    try {
      await revealSlot({
        owner,
        slot,
        cardName,
        commitment,
        recomputeDecision: true,
      });
    } catch (err) {
      const message = String(err?.message || err || "");
      if (!message.includes("not present") && !message.includes("not a hidden")) {
        throw err;
      }
    }
  }
}

async function revealAuditOpenings(game, openings = [], timing = null) {
  for (const opening of openings || []) {
    if (timing && String(opening?.timing || "pre") !== timing) continue;
    await revealOpeningWithGame(game, opening);
  }
}

function proofWithRequirementOrder(proof, requirement) {
  if (!proof || !requirement) return proof;
  const beforeOrder = normalizeShuffleOrder(requirement.beforeOrder ?? requirement.before_order);
  const afterOrder = normalizeShuffleOrder(requirement.afterOrder ?? requirement.after_order);
  if (beforeOrder.length === 0 && afterOrder.length === 0) return proof;
  return {
    ...proof,
    requirementId: String(requirementId(requirement) || proof.requirementId || ""),
    owner: Number(proof.owner ?? requirement.owner),
    zone: String(proof.zone || requirement.zone || "library"),
    beforeOrder,
    before_order: beforeOrder,
    afterOrder,
    after_order: afterOrder,
  };
}

function alignShuffleProofsWithRequirements(shuffleProofs = [], requirements = []) {
  const shuffleRequirements = (requirements || []).filter((requirement) =>
    requirementType(requirement) === "verifiable_shuffle"
  );
  if (shuffleRequirements.length === 0) return shuffleProofs || [];
  const aligned = [];
  const usedProofs = new Set();
  for (const requirement of shuffleRequirements) {
    const proof = (shuffleProofs || []).find((candidate) =>
      !usedProofs.has(candidate) && shuffleProofMatchesRequirement(candidate, requirement)
    ) || (shuffleProofs || []).find((candidate) =>
      !usedProofs.has(candidate)
      && Number(candidate?.owner) === Number(requirement.owner)
      && String(candidate?.zone || "library") === String(requirement.zone || "library")
    );
    if (!proof) continue;
    usedProofs.add(proof);
    aligned.push(proofWithRequirementOrder(proof, requirement));
  }
  return aligned.length > 0 ? aligned : (shuffleProofs || []);
}

async function applyVerifiedShuffleProofs(game, shuffleProofs = [], requirements = []) {
  const proofs = alignShuffleProofsWithRequirements(shuffleProofs, requirements)
    .filter((proof) => String(proof?.zone || "library") === "library");
  if (proofs.length === 0) return;
  const applyShuffle = optionalGameMethod(game, "applyVerifiedHiddenLibraryShuffle");
  if (!applyShuffle) {
    throw new Error("Game engine cannot replay transcript: missing applyVerifiedHiddenLibraryShuffle");
  }
  for (const proof of proofs) {
    await applyShuffle({
      owner: Number(proof.owner),
      deckHash: String(proof.deckHash || ""),
      afterOrder: normalizeShuffleOrder(proof.afterOrder ?? proof.after_order),
    });
  }
}

async function dispatchReplayCommand(game, command) {
  if (command?.type === "cancel_decision") {
    const cancelDecision = requiredGameMethod(game, "cancelDecision");
    return cancelDecision();
  }
  if (command?.type === "forfeit_player") {
    const forfeitPlayer = requiredGameMethod(game, "forfeitPlayer");
    return forfeitPlayer(Number(command.player));
  }
  const dispatch = requiredGameMethod(game, "dispatch");
  return dispatch(command);
}

async function currentPublicCheckpointHash(game, cryptoImpl) {
  const exportPublicAuditCheckpoint = requiredGameMethod(game, "exportPublicAuditCheckpoint");
  return publicCheckpointHash(await exportPublicAuditCheckpoint(), cryptoImpl);
}

export async function startAuditTranscriptReplayWithGame({
  game,
  transcript,
  perspectiveIndex = 0,
  cryptoImpl = globalThis.crypto,
} = {}) {
  if (!transcript || typeof transcript !== "object") {
    throw new Error("Missing audit transcript for engine replay");
  }
  const match = transcript.match || {};
  const startMatch = requiredGameMethod(game, "startMatch");
  await startMatch(replayMatchConfig(match));
  const setPerspective = optionalGameMethod(game, "setPerspective");
  if (setPerspective) {
    await setPerspective(normalizedPerspective(perspectiveIndex, match));
  }

  const expectedInitialPublicCheckpointHash = String(
    transcript.initialPublicCheckpointHash
      || match.initialPublicCheckpointHash
      || ""
  );
  const initialPublicCheckpointHash = await currentPublicCheckpointHash(game, cryptoImpl);
  if (
    expectedInitialPublicCheckpointHash
    && initialPublicCheckpointHash !== expectedInitialPublicCheckpointHash
  ) {
    throw new Error("Engine replay initial public checkpoint hash does not match transcript");
  }
  const uiState = optionalGameMethod(game, "uiState");
  return {
    initialPublicCheckpointHash,
    state: uiState ? await uiState() : null,
  };
}

export async function applyAuditReplayActionWithGame({
  game,
  action,
  actionIndex = 0,
  cryptoImpl = globalThis.crypto,
} = {}) {
  const seq = actionSeq(action, Number(actionIndex) + 1);
  const audit = action?.audit || {};
  const command = resolveSyncedCommand(action?.command || audit.command);
  const requirements = await previewCryptoRequirements(game, command);
  await injectTranscriptSeeds(game, requirements, audit);
  await revealAuditOpenings(game, audit.openings || [], "pre");
  await dispatchReplayCommand(game, command);
  await revealAuditOpenings(game, audit.openings || [], "post");
  await applyVerifiedShuffleProofs(game, audit.shuffleProofs || [], requirements);
  const checkpointHash = await currentPublicCheckpointHash(game, cryptoImpl);
  const uiState = optionalGameMethod(game, "uiState");
  return {
    seq,
    publicCheckpointHash: checkpointHash,
    state: uiState ? await uiState() : null,
  };
}

export async function replayAuditTranscriptWithGame({
  game,
  transcript,
  perspectiveIndex = 0,
  cryptoImpl = globalThis.crypto,
} = {}) {
  if (!transcript || typeof transcript !== "object") {
    throw new Error("Missing audit transcript for engine replay");
  }
  const match = transcript.match || {};
  const actions = Array.isArray(transcript.actions) ? transcript.actions : [];
  const exportSyncCheckpoint = requiredGameMethod(game, "exportSyncCheckpoint");
  const importSyncCheckpoint = requiredGameMethod(game, "importSyncCheckpoint");
  const restorePerspective = normalizedPerspective(perspectiveIndex, match);
  const restoreCheckpoint = await exportSyncCheckpoint();
  let replayError = null;
  let restoreError = null;
  let report = null;

  try {
    const { initialPublicCheckpointHash } = await startAuditTranscriptReplayWithGame({
      game,
      transcript,
      perspectiveIndex,
      cryptoImpl,
    });

    const actionReports = [];
    let index = 0;
    for (const entry of actions) {
      const actionReport = await applyAuditReplayActionWithGame({
        game,
        action: entry,
        actionIndex: index,
        cryptoImpl,
      });
      index += 1;
      actionReports.push({
        seq: actionReport.seq,
        publicCheckpointHash: actionReport.publicCheckpointHash,
      });
    }

    const finalPublicCheckpointHash = actions.length > 0
      ? String(actionReports.at(-1)?.publicCheckpointHash || "")
      : await currentPublicCheckpointHash(game, cryptoImpl);
    report = {
      verified: true,
      replayedActions: actionReports.length,
      actions: actionReports,
      actionReports,
      initialPublicCheckpointHash,
      finalPublicCheckpointHash,
    };
  } catch (err) {
    replayError = err;
  } finally {
    try {
      await importSyncCheckpoint(restoreCheckpoint, restorePerspective);
    } catch (restoreErr) {
      restoreError = restoreErr;
    }
  }

  if (replayError) throw replayError;
  if (restoreError) throw restoreError;
  return report;
}
