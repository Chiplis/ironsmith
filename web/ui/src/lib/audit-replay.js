import {
  importAuditPublicKey,
  publicCheckpointHash,
  publicDeckManifest,
  verifyAuditPayload,
  verifyCardOpeningAgainstManifest,
} from "./multiplayer-audit.js";
import { resolveSyncedCommand } from "./sync-commands.js";
import { captureEngineRestorePoint, restoreEngineRestorePoint } from "./engine-restore-point.js";

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

// Mirrors publicDecklistsForMatchPayload: the live match derived the Miracle
// draw reveal seats from the open decklists, so the replay must too.
function replayPublicDecklists(match) {
  if (!match?.openDecklists) return undefined;
  const players = transcriptPlayers(match);
  if (players.length === 0) return undefined;
  const cards = (list) => (Array.isArray(list) ? list : [])
    .map((card) => String(card || "").trim())
    .filter(Boolean);
  return players.map((player) => [
    ...cards(player?.deck),
    ...cards(player?.sideboard),
    ...cards(player?.commanders),
  ]);
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
    publicDecklists: replayPublicDecklists(match),
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
  // Match by requirement id only and apply in shuffle order (engine random
  // counter), mirroring the live peer path.
  const aligned = [];
  const usedProofs = new Set();
  const seenRequirementIds = new Set();
  for (const [index, requirement] of shuffleRequirements.entries()) {
    const id = String(requirementId(requirement) || "");
    if (id) {
      if (seenRequirementIds.has(id)) continue;
      seenRequirementIds.add(id);
    }
    const proof = (shuffleProofs || []).find((candidate) =>
      !usedProofs.has(candidate) && shuffleProofMatchesRequirement(candidate, requirement)
    );
    if (!proof) continue;
    usedProofs.add(proof);
    const randomCountBefore = Number(
      requirement?.randomCountBefore ?? requirement?.random_count_before
    );
    aligned.push({
      index,
      randomCountBefore: Number.isSafeInteger(randomCountBefore) && randomCountBefore >= 0
        ? randomCountBefore
        : Number.MAX_SAFE_INTEGER,
      proof: proofWithRequirementOrder(proof, requirement),
    });
  }
  if (aligned.length === 0) return shuffleProofs || [];
  aligned.sort((left, right) =>
    left.randomCountBefore - right.randomCountBefore || left.index - right.index
  );
  return aligned.map((entry) => entry.proof);
}

async function applyVerifiedShuffleProofs(game, shuffleProofs = [], requirements = []) {
  const proofs = alignShuffleProofsWithRequirements(shuffleProofs, requirements)
    .filter((proof) => String(proof?.zone || "library") === "library");
  if (proofs.length === 0) return;
  const applyShuffle = optionalGameMethod(game, "applyVerifiedHiddenLibraryShuffle");
  if (!applyShuffle) {
    throw new Error("Game engine cannot replay transcript: missing applyVerifiedHiddenLibraryShuffle");
  }
  const lastProofIndexByOwner = new Map();
  proofs.forEach((proof, index) => lastProofIndexByOwner.set(Number(proof.owner), index));
  const ownersWithOrderUpdates = new Set(
    (requirements || [])
      .filter((requirement) =>
        requirementType(requirement) === "hidden_order_update"
        && String(requirement?.zone || "library") === "library"
      )
      .map((requirement) => Number(requirement.owner))
  );
  for (const [index, proof] of proofs.entries()) {
    const owner = Number(proof.owner);
    await applyShuffle({
      owner,
      deckHash: String(proof.deckHash || ""),
      afterOrder: normalizeShuffleOrder(proof.afterOrder ?? proof.after_order),
      enforceLibraryOrder:
        lastProofIndexByOwner.get(owner) === index && !ownersWithOrderUpdates.has(owner),
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
  // Same order as the live actor and peers: reseal verified shuffles first, then
  // reveal post openings against the post-shuffle ceremony.
  await applyVerifiedShuffleProofs(game, audit.shuffleProofs || [], requirements);
  await revealAuditOpenings(game, audit.openings || [], "post");
  const checkpointHash = await currentPublicCheckpointHash(game, cryptoImpl);
  const uiState = optionalGameMethod(game, "uiState");
  return {
    seq,
    publicCheckpointHash: checkpointHash,
    state: uiState ? await uiState() : null,
  };
}

// Mirrors END_OF_MATCH_DISCLOSURE_DOMAIN in hooks/peer-lobby/end-of-match-disclosure.js.
const END_OF_MATCH_DISCLOSURE_DOMAIN = "ironsmith-end-of-match-disclosure-v1";

function transcriptPlayerForSeat(match, seat) {
  return transcriptPlayers(match).find((player, index) =>
    Number(player?.index ?? index) === Number(seat)
  ) || null;
}

function transcriptDeckManifestForSeat(match, seat) {
  const manifests = Array.isArray(match?.deckAuditManifests) ? match.deckAuditManifests : [];
  const fromList = manifests.find((manifest) => Number(manifest?.owner) === Number(seat))
    || manifests[Number(seat)]
    || null;
  return publicDeckManifest(fromList || transcriptPlayerForSeat(match, seat)?.deckAuditManifest);
}

async function replayVerdictForDisclosure(game, match, entry, cryptoImpl, transcriptMatchId = "") {
  const disclosure = entry?.disclosure || null;
  if (!disclosure) {
    return { status: "missing", reason: "no signed end-of-match disclosure in the transcript" };
  }
  const player = Number(disclosure.player ?? entry.player);
  const payload = {
    domain: END_OF_MATCH_DISCLOSURE_DOMAIN,
    matchId: String(disclosure.matchId || ""),
    player,
    openings: clonePayload(Array.isArray(disclosure.openings) ? disclosure.openings : []),
  };
  const expectedMatchId = String(transcriptMatchId || match?.auditMatchId || "");
  if (String(disclosure.domain || "") !== END_OF_MATCH_DISCLOSURE_DOMAIN) {
    return { status: "cheat_detected", reason: "End-of-match disclosure has the wrong domain" };
  }
  if (expectedMatchId && payload.matchId !== expectedMatchId) {
    return { status: "cheat_detected", reason: "End-of-match disclosure belongs to a different match" };
  }
  const publicKeyHex = String(transcriptPlayerForSeat(match, player)?.auditPublicKey || "");
  if (!publicKeyHex) {
    return { status: "unverifiable", reason: `no audit public key for player ${player + 1}` };
  }
  const publicKey = await importAuditPublicKey(publicKeyHex, cryptoImpl);
  const validSignature = await verifyAuditPayload(
    publicKey,
    payload,
    String(disclosure.signature || ""),
    cryptoImpl,
  );
  if (!validSignature) {
    return { status: "cheat_detected", reason: "End-of-match disclosure signature is invalid" };
  }
  // Deck-manifest commitments. Ziffle position proofs are not re-verified
  // here (the replay reveals every opening by its position, as it does for
  // the openings of replayed actions).
  const manifest = transcriptDeckManifestForSeat(match, player);
  if (manifest) {
    for (const opening of payload.openings) {
      if (!opening || opening.slot == null) continue;
      const valid = await verifyCardOpeningAgainstManifest({
        manifest,
        slot: opening.slot,
        card: opening.card,
        salt: opening.salt,
      }, cryptoImpl);
      if (!valid) {
        return {
          status: "cheat_detected",
          reason: `Card opening for player ${player + 1}, slot ${Number(opening.slot)} `
            + "does not match its deck commitment",
        };
      }
    }
  }
  const verify = optionalGameMethod(game, "verifyEndOfMatchDisclosure");
  if (!verify) {
    return { status: "unverifiable", reason: "engine cannot verify end-of-match disclosures" };
  }
  const result = await verify(player, payload.openings);
  const violations = Array.isArray(result?.violations) ? result.violations : [];
  const missing = Array.isArray(result?.missing) ? result.missing : [];
  if (violations.length > 0) {
    return { status: "cheat_detected", reason: violations.join("; ") };
  }
  if (missing.length > 0) {
    return {
      status: "cheat_detected",
      reason: `End-of-match disclosure omits ${missing.length} hidden card`
        + `${missing.length === 1 ? "" : "s"} (objects ${missing.join(", ")})`,
    };
  }
  return { status: "verified", reason: manifest ? "" : "deck manifest unavailable" };
}

// Re-verify the transcript's end-of-match disclosures against the engine's
// final replayed state (commitments, obligation ledger, library anchors).
// Returns one report per disclosure entry: the verdict recorded live and the
// verdict reached by the replay.
export async function verifyEndOfMatchDisclosuresWithGame({
  game,
  transcript,
  cryptoImpl = globalThis.crypto,
} = {}) {
  const entries = Array.isArray(transcript?.endOfMatchDisclosures)
    ? transcript.endOfMatchDisclosures
    : [];
  const match = transcript?.match || {};
  const reports = [];
  for (const entry of entries) {
    let replayVerdict;
    try {
      replayVerdict = await replayVerdictForDisclosure(
        game,
        match,
        entry,
        cryptoImpl,
        String(transcript?.matchId || "")
      );
    } catch (err) {
      replayVerdict = {
        status: "unverifiable",
        reason: String(err?.message || err || "disclosure verification failed"),
      };
    }
    reports.push({
      player: Number(entry?.disclosure?.player ?? entry?.player),
      recordedVerdict: entry?.verdict ? clonePayload(entry.verdict) : null,
      replayVerdict,
    });
  }
  return reports;
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
  requiredGameMethod(game, "exportSyncCheckpoint");
  requiredGameMethod(game, "importSyncCheckpoint");
  const restorePerspective = normalizedPerspective(perspectiveIndex, match);
  // Restore the caller's game losslessly: a sync checkpoint alone would drop
  // continuous effects, delayed triggers and the rest of the rules state.
  const restorePoint = await captureEngineRestorePoint(game);
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
    // End-of-match disclosures are checked against the final replayed state,
    // before the caller's game is restored.
    const endOfMatchDisclosures = await verifyEndOfMatchDisclosuresWithGame({
      game,
      transcript,
      cryptoImpl,
    });
    report = {
      verified: true,
      replayedActions: actionReports.length,
      actions: actionReports,
      actionReports,
      initialPublicCheckpointHash,
      finalPublicCheckpointHash,
      endOfMatchDisclosures,
      endOfMatchDisclosuresVerified: endOfMatchDisclosures.every(
        (entry) => entry.replayVerdict?.status === "verified"
      ),
    };
  } catch (err) {
    replayError = err;
  } finally {
    try {
      await restoreEngineRestorePoint(game, restorePoint, restorePerspective);
    } catch (restoreErr) {
      restoreError = restoreErr;
    }
  }

  if (replayError) throw replayError;
  if (restoreError) throw restoreError;
  return report;
}
