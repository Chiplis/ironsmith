import { acceptedZiffleEpochs, assertZiffleEpochInputs, ziffleEpochMaterial } from "./ziffle-private-epochs.js";
import {
  importAuditPublicKey,
  publicCheckpointHash,
  publicDeckManifest,
  verifyAuditPayload,
  verifyCardOpeningAgainstManifest,
  ziffleOriginAnchorFromOpening,
  ziffleOriginAnchorFromMetadata,
  assertZiffleOpeningOriginMatchesMetadata,
} from "./multiplayer-audit.js";
import { resolveSyncedCommand } from "./sync-commands.js";
import { actionRefObjectId, actionRefWithObjectId, hiddenObjectIdForHiddenRefFromCheckpoint } from "./sync-object-identity.js";
import { captureEngineRestorePoint, restoreEngineRestorePoint } from "./engine-restore-point.js";
import { findZiffleDisclosureOrigin, ziffleDisclosureDueForPlayer } from "./ziffle-disclosure-origin.js";

const DEFAULT_OPENING_HAND_SIZE = 7;
const privateReplayHistories = new WeakMap();

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
      if (proof?.deckHash && !proof.inputDeck) {
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
    if (proof?.deckHash && !proof.inputDeck) seeds.push(String(proof.deckHash));
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
  let positionObjectId = null;
  const exportCheckpoint = optionalGameMethod(game, "getHiddenCardState");
  const checkpoint = exportCheckpoint ? await exportCheckpoint() : null;
  const ownerMetadata = (checkpoint?.objects || [])
    .map(object => object.hiddenCard || object.hidden_card)
    .filter(hidden => hidden && Number(hidden.owner) === owner);
  if (ownerMetadata.some(hidden => ziffleOriginAnchorFromMetadata(hidden))
    && !positionCommitment?.startsWith("ziffle:")) {
    // A sender cannot opt out of identity verification by omitting its
    // position fields. Only an exact unshuffled commitment (e.g. sideboard)
    // may use the ordinary slot-opening path in a Ziffle deck.
    const unshuffled = ownerMetadata.filter(hidden => !ziffleOriginAnchorFromMetadata(hidden)
      && commitment && hidden.commitment === commitment && Number(hidden.slot) === slot);
    if (unshuffled.length !== 1) throw new Error("Replay Ziffle opening is missing its current position and origin");
  }

  // The proof establishes the original committed identity. Independently bind
  // it to the replay engine's current position before revealing any card.
  if (positionCommitment?.startsWith("ziffle:") || ziffleOriginAnchorFromOpening(opening)) {
    if (!checkpoint) throw new Error("Game engine cannot bind replay opening identity");
    const matches = (checkpoint?.objects || []).filter(object => {
      const hidden = object.hiddenCard || object.hidden_card;
      return hidden && Number(hidden.owner) === owner
        && Number(hidden.publicSlot ?? hidden.public_slot ?? hidden.slot) === Number(position)
        && String(hidden.publicCommitment || hidden.public_commitment || hidden.commitment || "") === String(positionCommitment);
    });
    if (matches.length !== 1) throw new Error("Replay opening does not identify one current committed card");
    const hidden = matches[0].hiddenCard || matches[0].hidden_card;
    if (ziffleOriginAnchorFromMetadata(hidden) || ziffleOriginAnchorFromOpening(opening)) {
      assertZiffleOpeningOriginMatchesMetadata(opening, hidden);
    }
    // A card may already be revealed and have a new object id after changing
    // zones. Keep the identity authenticated above instead of relying on the
    // engine's placeholder-only positional lookup or the sender's retired id.
    positionObjectId = Number(matches[0].id);
    if (!Number.isSafeInteger(positionObjectId) || positionObjectId <= 0) {
      throw new Error("Replay opening has no valid current committed card identity");
    }
  }

  const revealPosition = optionalGameMethod(game, "revealHiddenPosition");
  if (position != null && revealPosition) {
    try {
      await revealPosition({
        owner,
        ...(positionObjectId != null ? { objectId: positionObjectId } : {}),
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
  if (!proof || !requirement || proof.inputDeck) return proof;
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
    .filter((proof) => !proof.inputDeck && String(proof?.zone || "library") === "library");
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

async function localReplayCommand(game, command) {
  const hasPriorityIdentity = command?.type === "priority_action"
    && actionRefObjectId(command.action_ref) != null
    && (command.object_stable_id != null || command.object_hidden_ref);
  const hasSelectionIdentity = command?.type === "select_objects"
    && (command.object_stable_ids?.some(value => value != null) || command.object_hidden_refs?.some(Boolean));
  if (!hasPriorityIdentity && !hasSelectionIdentity) return command;
  const checkpoint = await requiredGameMethod(game, "getHiddenCardState")();
  const resolve = (originalId, stableId, hiddenRef) => {
    if (stableId != null) {
      const matches = (checkpoint?.objects || []).filter(object =>
        Number(object.stableId ?? object.stable_id) === Number(stableId));
      if (matches.length !== 1) throw new Error("Replay command has no unique current stable card identity");
      return Number(matches[0].id);
    }
    if (hiddenRef) {
      const id = hiddenObjectIdForHiddenRefFromCheckpoint(checkpoint, hiddenRef);
      if (id == null) throw new Error("Replay command has no unique current hidden card identity");
      return id;
    }
    return originalId;
  };
  if (hasPriorityIdentity) {
    const id = resolve(actionRefObjectId(command.action_ref), command.object_stable_id, command.object_hidden_ref);
    return { ...command, object_id: id, action_ref: actionRefWithObjectId(command.action_ref, id) };
  }
  return { ...command, object_ids: command.object_ids.map((id, index) =>
    resolve(id, command.object_stable_ids?.[index], command.object_hidden_refs?.[index])) };
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
  privateReplayHistories.set(game, { match, actions: [] });
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

function futurePrivateOpeningProof(opening, proofs = []) {
  const positionCommitment = String(opening?.positionCommitment || opening?.position_commitment || "");
  return proofs.find(proof => proof.inputDeck && Number(proof.owner) === Number(opening.owner)
    && positionCommitment === `ziffle:${proof.deckHash}:${Number(opening.position)}`);
}

async function queuePrivateReplayEpochs(game, command, initialRequirements, audit) {
  const privateProofs = (audit.shuffleProofs || []).filter(proof => proof.inputDeck);
  const history = privateReplayHistories.get(game);
  const requiresPrivateProofs = Number(history?.match?.protocolVersion) >= 15;
  if (privateProofs.length === 0) {
    if (requiresPrivateProofs && initialRequirements.some(entry => requirementType(entry) === "verifiable_shuffle")) {
      throw new Error("Private replay is missing a required shuffle proof");
    }
    return initialRequirements;
  }
  if (!history) throw new Error("Private replay is missing its signed initial deck history");
  const queueEpoch = requiredGameMethod(game, "queueVerifiedHiddenLibraryEpoch");
  const preceding = [];
  let requirements = initialRequirements;
  for (const proof of privateProofs) {
    const requirement = requirements.find(entry => requirementType(entry) === "verifiable_shuffle"
      && shuffleProofMatchesRequirement(proof, entry));
    if (!requirement || Number(requirement.owner) !== Number(proof.owner)) {
      throw new Error("Private replay shuffle is not required by the local engine");
    }
    const accepted = acceptedZiffleEpochs(history.match, history.actions, proof.owner, preceding);
    const inputs = assertZiffleEpochInputs(proof, requirement, accepted);
    await queueEpoch(ziffleEpochMaterial(proof, requirement, inputs));
    requirements = await previewCryptoRequirements(game, command);
    let queuedOpening = false;
    for (const opening of audit.openings || []) {
      if (String(opening?.timing || "pre") !== "pre" || futurePrivateOpeningProof(opening, [proof]) !== proof) continue;
      const authorized = requirements.some(entry => requirementType(entry) === "public_open"
        && Number(entry.owner) === Number(opening.owner)
        && Number(entry.publicSlot ?? entry.public_slot ?? entry.slot) === Number(opening.position)
        && String(entry.publicCommitment || entry.public_commitment || entry.commitment || "") === String(opening.positionCommitment));
      if (!authorized) throw new Error("Future replay opening is not required for public disclosure");
      const origin = ziffleOriginAnchorFromOpening(opening);
      if (!origin || origin.originPosition !== Number(opening.position)
        || origin.originPositionCommitment !== String(opening.positionCommitment)) {
        throw new Error("Future replay opening is not bound to its new ciphertext position");
      }
      await requiredGameMethod(game, "queueVerifiedHiddenLibraryOpening")({
        owner: Number(opening.owner), deckHash: String(proof.deckHash),
        position: Number(opening.position), cardName: String(opening.card),
        originalSlot: Number(opening.slot), commitment: String(opening.commitment || ""),
      });
      queuedOpening = true;
    }
    preceding.push(proof);
    if (queuedOpening) requirements = await previewCryptoRequirements(game, command);
  }
  if (requirements.some(entry => requirementType(entry) === "verifiable_shuffle"
    && !preceding.some(proof => Number(proof.owner) === Number(entry.owner)
      && shuffleProofMatchesRequirement(proof, entry)))) {
    throw new Error("Private replay is missing a shuffle proof discovered after opening its source cards");
  }
  return requirements;
}

export async function applyAuditReplayActionWithGame({
  game,
  action,
  actionIndex = 0,
  cryptoImpl = globalThis.crypto,
} = {}) {
  const seq = actionSeq(action, Number(actionIndex) + 1);
  const audit = action?.audit || {};
  let command = resolveSyncedCommand(action?.command || audit.command);
  // Public replay starts with concealed hands. Reveal authenticated pre-action
  // cards before asking the engine whether a land or spell can be played.
  await revealAuditOpenings(game, (audit.openings || []).filter(opening =>
    !futurePrivateOpeningProof(opening, audit.shuffleProofs || [])), "pre");
  command = await localReplayCommand(game, command);
  let requirements = await previewCryptoRequirements(game, command);
  await injectTranscriptSeeds(game, requirements, audit);
  if ((audit.shuffleProofs || []).some(proof => !proof.inputDeck) || audit.rngReveals?.length) {
    requirements = await previewCryptoRequirements(game, command);
  }
  requirements = await queuePrivateReplayEpochs(game, command, requirements, audit);
  await dispatchReplayCommand(game, command);
  // Same order as the live actor and peers: reseal verified shuffles first, then
  // reveal post openings against the post-shuffle ceremony.
  await applyVerifiedShuffleProofs(game, audit.shuffleProofs || [], requirements);
  await revealAuditOpenings(game, audit.openings || [], "post");
  const checkpointHash = await currentPublicCheckpointHash(game, cryptoImpl);
  privateReplayHistories.get(game)?.actions.push(action);
  const uiState = optionalGameMethod(game, "uiState");
  return {
    seq,
    publicCheckpointHash: checkpointHash,
    state: uiState ? await uiState() : null,
  };
}

// Mirrors END_OF_MATCH_DISCLOSURE_DOMAIN in hooks/peer-lobby/end-of-match-disclosure.js.
const END_OF_MATCH_DISCLOSURE_DOMAIN = "ironsmith-end-of-match-disclosure-v1";

// Mirrors pendingDisclosureClaimCount in hooks/peer-lobby/end-of-match-disclosure.js:
// deferred claims about `player`'s hidden cards (public facts only) that only
// its end-of-match disclosure can settle. Null when the engine cannot tell.
async function pendingDisclosureClaimCount(game, player) {
  const obligations = optionalGameMethod(game, "endOfMatchDisclosureObligations");
  if (!obligations) return null;
  const result = await obligations(Number(player));
  const subjects = Number(result?.claimSubjects ?? result?.claim_subjects ?? 0);
  const anchors = Number(result?.libraryAnchors ?? result?.library_anchors ?? 0);
  return (Number.isFinite(subjects) ? subjects : 0) + (Number.isFinite(anchors) ? anchors : 0);
}

// The verdict for a player that delivered no disclosure: withheld (a verdict
// against it) when claims about its hidden cards are pending, not required
// when none are, missing when the engine cannot tell.
async function absentDisclosureVerdict(game, player, missingReason) {
  const pendingClaims = await pendingDisclosureClaimCount(game, player);
  if (pendingClaims != null && pendingClaims > 0) {
    return {
      status: "withheld",
      reason: `withheld its end-of-match disclosure with ${pendingClaims} pending`
        + ` claim${pendingClaims === 1 ? "" : "s"} about hidden cards`,
      accusedPlayers: [Number(player)],
    };
  }
  if (pendingClaims === 0) {
    return { status: "not_required", reason: "no pending claims about hidden cards" };
  }
  return { status: "missing", reason: missingReason };
}

// Disclosure verdicts that complete verification (withheld is a verdict
// against the player, not a failure to verify).
const SETTLED_DISCLOSURE_STATUSES = new Set(["verified", "not_required", "withheld"]);

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
    return absentDisclosureVerdict(
      game,
      Number(entry?.player),
      "no signed end-of-match disclosure in the transcript"
    );
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
  // The transcript verifier checks cryptographic opening proofs. Replay also
  // binds their claimed genesis origins to its independently derived final
  // obligations, including cards removed when a player leaves the game.
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
  const requirementsForPlayer = optionalGameMethod(game, "endOfMatchDisclosureRequirements");
  const state = await optionalGameMethod(game, "uiState")?.();
  const requirements = requirementsForPlayer ? await requirementsForPlayer(player) : [];
  const hasZiffleRequirements = requirements.some(requirement =>
    ziffleOriginAnchorFromMetadata(requirement));
  for (const opening of payload.openings) {
    if (Number(opening?.owner) !== player) {
      return { status: "cheat_detected", reason: "Disclosure opening belongs to another player" };
    }
    const hasZifflePosition = String(opening.positionCommitment || "").startsWith("ziffle:");
    if (hasZifflePosition || ziffleOriginAnchorFromOpening(opening)) {
      const origin = findZiffleDisclosureOrigin({ opening, state, requirements });
      assertZiffleOpeningOriginMatchesMetadata(opening, origin?.metadata);
    } else if (hasZiffleRequirements) {
      const unshuffled = requirements.filter(requirement => !ziffleOriginAnchorFromMetadata(requirement)
        && Number(requirement.owner) === player && Number(requirement.slot) === Number(opening.slot)
        && opening.commitment && requirement.commitment === opening.commitment);
      if (unshuffled.length !== 1) {
        return { status: "cheat_detected", reason: "Disclosure opening omits its Ziffle origin" };
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
  const requirementsForPlayer = optionalGameMethod(game, "endOfMatchDisclosureRequirements");
  const state = await optionalGameMethod(game, "uiState")?.();
  if (requirementsForPlayer && state) {
    for (let player = 0; player < replayPlayerNames(match).length; player++) {
      if (!ziffleDisclosureDueForPlayer(state, player)) continue;
      const requirements = await requirementsForPlayer(player);
      if (requirements.length === 0 || reports.some(report => report.player === player)) continue;
      reports.push({ player, recordedVerdict: null, replayVerdict: await absentDisclosureVerdict(
        game,
        player,
        "Transcript omits a required end-of-match disclosure"
      ) });
    }
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
  requiredGameMethod(game, "getHiddenCardState");
  // Restore the caller's game losslessly: a sync checkpoint alone would drop
  // continuous effects, delayed triggers and the rest of the rules state.
  const restorePoint = await captureEngineRestorePoint(game);
  const previousPrivateHistory = privateReplayHistories.get(game);
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
        (entry) => SETTLED_DISCLOSURE_STATUSES.has(entry.replayVerdict?.status)
      ),
      endOfMatchDisclosureWithheldPlayers: endOfMatchDisclosures
        .filter((entry) => entry.replayVerdict?.status === "withheld")
        .map((entry) => Number(entry.player))
        .sort((left, right) => left - right),
    };
  } catch (err) {
    replayError = err;
  } finally {
    try {
      await restoreEngineRestorePoint(game, restorePoint);
    } catch (restoreErr) {
      restoreError = restoreErr;
    }
    if (previousPrivateHistory) privateReplayHistories.set(game, previousPrivateHistory);
    else privateReplayHistories.delete(game);
  }

  if (replayError) throw replayError;
  if (restoreError) throw restoreError;
  return report;
}
