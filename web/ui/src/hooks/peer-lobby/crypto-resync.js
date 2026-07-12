import {
  DISCONNECT_AUTO_FORFEIT_MS,
  DISCONNECT_FORFEIT_REASON,
  INITIAL_AUDIT_STATE_HASH,
  INITIAL_MATCH_CLOCK_HASH,
  MATCH_CLOCK_AUDIT_TYPE,
  MATCH_CLOCK_CLAIM_SKEW_MS,
  MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS,
  MULTIPLAYER_SECURITY_TRUSTED,
  MULTIPLAYER_SECURITY_VERIFIED,
  PROTOCOL_RESPONSE_TIMEOUT_MS,
  PROTOCOL_RESPONSE_TIMEOUT_REASON,
  PROTOCOL_RESPONSE_TIMEOUT_VOTE_WAIT_MS,
  PROTOCOL_VERSION,
  TIMEOUT_VOTE_DOMAIN,
  actionQuorumThreshold,
  actionRefObjectId,
  actionRefWithObjectId,
  actionTimerSnapshotFromMatchClock,
  authorizeCryptoMaterialRequestRequirements,
  buildActionForkDisputeEvidence,
  buildDeckSlotOpening,
  buildSignedActionQuorumVote,
  buildSignedDisconnectForfeitVote,
  buildSignedProtocolResponseTimeoutVote,
  buildSignedResyncEnvelope,
  buildZiffleOpeningProof,
  canonicalJson,
  canonicalMultiplayerPayload,
  clearStoredPlayerIndex,
  cloneMultiplayerPayload,
  createEmptyState,
  createMatchClockSnapshot,
  cryptoMaterialResponsibleSeat,
  cryptoRequirementsFromState,
  debitMatchClockRemaining,
  decryptPrivateAuditPayload,
  disconnectCertificateFromCommand,
  disconnectForfeitVoteThreshold,
  emitSyncFailureNotice,
  encryptPrivateAuditPayload,
  expectedTimeoutVoters,
  filterCryptoRequirementsForCommand,
  hiddenObjectIdForHiddenRefFromCheckpoint,
  hiddenOpeningMatchesExport,
  isActionTimeoutForfeitCommand,
  isCurrentAuditPlayerCount,
  isDecisionCommandCompatible,
  isDisadvantageousActivePlayerClockAdvance,
  isDisconnectTimeoutForfeitCommand,
  isForfeitCommand,
  isOwnerPrivateViewRequirement,
  isProtocolResponseTimeoutForfeitCommand,
  isRejectedActionCheatReason,
  isSelfForfeitCommand,
  isSorcerySpeedForfeitState,
  isTrustedMultiplayerSecurityMode,
  isUnauthorizedAddCardCommand,
  isVerifiedMultiplayerSecurityMode,
  matchClockActivePlayerFromState,
  matchClockAuditHash,
  matchClockPolicyFromPayload,
  matchClockPolicyPayload,
  matchPayloadSecurityMode,
  mergeAuditOpenings,
  mergePrivateViewProofs,
  normalizeMatchClockPolicy,
  normalizeMatchClockRemaining,
  normalizeMatchFormat,
  normalizeMultiplayerSecurityMode,
  normalizePlayerIndex,
  normalizeSelectObjectHiddenRef,
  normalizeShuffleOrder,
  nowMonotonicMs,
  payloadSizeBytes,
  playerNameForIndex,
  protocolResponseTimeoutClaimFromError,
  protocolResponseTimeoutVoteThreshold,
  readStoredActionQuorumVote,
  recordPeerSyncPerf,
  redactedMatchPayloadForPeer,
  reindexPlayers,
  resolveLocalPlayerIndex,
  safeSend,
  sameShuffleOrder,
  sessionSecurityMode,
  sha256Hex,
  shuffleProofMatchesRequirement,
  signAuditPayload,
  summarizeCryptoMaterialForPerf,
  summarizeCryptoRequirementsForPerf,
  summarizePeerCommand,
  summarizeSequencedActionForPerf,
  timePeerSyncPhase,
  timeoutCertificateFromCommand,
  timeoutVotePayload,
  toErrorMessage,
  toPublicPlayers,
  useCallback,
  verifyActionQuorumCertificate,
  verifyActionQuorumVote,
  verifyAuditPayload,
  verifyDisconnectForfeitCertificate,
  verifyDisconnectForfeitVote,
  verifyProtocolResponseTimeoutCertificate,
  verifyProtocolResponseTimeoutVote,
  wasmObjectIdArg,
  writeStoredActionQuorumVote,
  ziffleContextFromCeremony,
  ziffleDeckHashFromCommitment,
  ziffleKeyContextForCeremony,
  zifflePositionFromCommitment,
} from "./shared.js";

export function usePeerLobbyCryptoResync(base, servicesRef) {
  const { actionCryptoRequirementsRef, actionHistoryRef, applySyncedCommand, applyingSequencedActionsRef, auditKeyPairRef, auditStateHashRef, awaitingStateResyncRef, clientConnectionsRef, drainingPendingSequencedActionsRef, gameRef, hostConnectionRef, ignoredActionIntentKeysRef, initialPublicCheckpointHashRef, liveAuditTranscriptRef, liveZiffleCeremoniesRef, localDisconnectObservationsRef, localRevealedOpeningsRef, localZiffleCeremonyLookupRef, localZiffleRevealInFlightRef, matchClockConfigRef, matchClockObservationExemptSequenceRef, matchClockRef, matchStartPayloadRef, multiplayerRef, outboundCryptoMaterialRequestsRef, peerConnectionsRef, peerRef, pendingSequencedActionsRef, privateViewDisclosuresRef, reconnectChallengesRef, relayedActionIdsRef, resyncWaitersRef, resyncingPeerIdsRef, setState, setStatus, signedActionQuorumVotesRef, stateRef, timeoutClaimInFlightRef, verifiedAuditOpeningsRef, verifiedShuffleProofsRef, ziffleHandRevealKeyRef, ziffleHandRevealQuickKeyRef, ziffleOpeningPositionsRef, ziffleRevealTokenCacheRef } = base;
  const applySequencedActionMessage = useCallback((...args) => servicesRef.current.applySequencedActionMessage(...args), [servicesRef]);
  const auditEncryptionPublicKeyForPlayer = useCallback((...args) => servicesRef.current.auditEncryptionPublicKeyForPlayer(...args), [servicesRef]);
  const buildLocalOpeningFromRequirement = useCallback((...args) => servicesRef.current.buildLocalOpeningFromRequirement(...args), [servicesRef]);
  const buildLocalOpeningsForCommand = useCallback((...args) => servicesRef.current.buildLocalOpeningsForCommand(...args), [servicesRef]);
  const buildLocalRequirementOpeningsForRequirements = useCallback((...args) => servicesRef.current.buildLocalRequirementOpeningsForRequirements(...args), [servicesRef]);
  const buildOpeningFromResolvedCommittedSlot = useCallback((...args) => servicesRef.current.buildOpeningFromResolvedCommittedSlot(...args), [servicesRef]);
  const clearAllConnectionHeartbeats = useCallback((...args) => servicesRef.current.clearAllConnectionHeartbeats(...args), [servicesRef]);
  const clearAllPendingActionIntents = useCallback((...args) => servicesRef.current.clearAllPendingActionIntents(...args), [servicesRef]);
  const clearPeerWait = useCallback((...args) => servicesRef.current.clearPeerWait(...args), [servicesRef]);
  const clearPendingActionIntent = useCallback((...args) => servicesRef.current.clearPendingActionIntent(...args), [servicesRef]);
  const collectZiffleRevealTokensBatch = useCallback((...args) => servicesRef.current.collectZiffleRevealTokensBatch(...args), [servicesRef]);
  const currentAuditMatchId = useCallback((...args) => servicesRef.current.currentAuditMatchId(...args), [servicesRef]);
  const currentHiddenCardMetadataForObject = useCallback((...args) => servicesRef.current.currentHiddenCardMetadataForObject(...args), [servicesRef]);
  const currentHiddenObjectIdForOpening = useCallback((...args) => servicesRef.current.currentHiddenObjectIdForOpening(...args), [servicesRef]);
  const currentPublicAuditCheckpointHash = useCallback((...args) => servicesRef.current.currentPublicAuditCheckpointHash(...args), [servicesRef]);
  const ensureAuditIdentity = useCallback((...args) => servicesRef.current.ensureAuditIdentity(...args), [servicesRef]);
  const ensureDirectPeerConnections = useCallback((...args) => servicesRef.current.ensureDirectPeerConnections(...args), [servicesRef]);
  const ensureZiffleOpeningProof = useCallback((...args) => servicesRef.current.ensureZiffleOpeningProof(...args), [servicesRef]);
  const ignoreAndClearAllPendingActionIntents = useCallback((...args) => servicesRef.current.ignoreAndClearAllPendingActionIntents(...args), [servicesRef]);
  const importCachedAuditPublicKey = useCallback((...args) => servicesRef.current.importCachedAuditPublicKey(...args), [servicesRef]);
  const localRevealedOpeningForRequirement = useCallback((...args) => servicesRef.current.localRevealedOpeningForRequirement(...args), [servicesRef]);
  const makeZiffleRequestId = useCallback((...args) => servicesRef.current.makeZiffleRequestId(...args), [servicesRef]);
  const playerIndexForPeerId = useCallback((...args) => servicesRef.current.playerIndexForPeerId(...args), [servicesRef]);
  const preserveViewedCardsFromHint = useCallback((...args) => servicesRef.current.preserveViewedCardsFromHint(...args), [servicesRef]);
  const previewRequirementsForCommand = useCallback((...args) => servicesRef.current.previewRequirementsForCommand(...args), [servicesRef]);
  const privateDeckManifestForOwner = useCallback((...args) => servicesRef.current.privateDeckManifestForOwner(...args), [servicesRef]);
  const publicKeyForAuditSigner = useCallback((...args) => servicesRef.current.publicKeyForAuditSigner(...args), [servicesRef]);
  const refreshPendingActionIntentEvidenceForAction = useCallback((...args) => servicesRef.current.refreshPendingActionIntentEvidenceForAction(...args), [servicesRef]);
  const rememberLocalRevealedOpening = useCallback((...args) => servicesRef.current.rememberLocalRevealedOpening(...args), [servicesRef]);
  const rememberPendingActionIntent = useCallback((...args) => servicesRef.current.rememberPendingActionIntent(...args), [servicesRef]);
  const rememberPrivateViewDisclosure = useCallback((...args) => servicesRef.current.rememberPrivateViewDisclosure(...args), [servicesRef]);
  const rememberZiffleOpeningPosition = useCallback((...args) => servicesRef.current.rememberZiffleOpeningPosition(...args), [servicesRef]);
  const resolveCommittedSlotForZifflePosition = useCallback((...args) => servicesRef.current.resolveCommittedSlotForZifflePosition(...args), [servicesRef]);
  const resolveCommittedZiffleRevealSlot = useCallback((...args) => servicesRef.current.resolveCommittedZiffleRevealSlot(...args), [servicesRef]);
  const resolveLocalCryptoPlayerIndex = useCallback((...args) => servicesRef.current.resolveLocalCryptoPlayerIndex(...args), [servicesRef]);
  const revealAuditOpenings = useCallback((...args) => servicesRef.current.revealAuditOpenings(...args), [servicesRef]);
  const routePeerIdForPlayer = useCallback((...args) => servicesRef.current.routePeerIdForPlayer(...args), [servicesRef]);
  const sanitizeObjectBoundOpening = useCallback((...args) => servicesRef.current.sanitizeObjectBoundOpening(...args), [servicesRef]);
  const sendDirectPeerMessage = useCallback((...args) => servicesRef.current.sendDirectPeerMessage(...args), [servicesRef]);
  const sendDirectProtocolMessage = useCallback((...args) => servicesRef.current.sendDirectProtocolMessage(...args), [servicesRef]);
  const signActionIntentForCommand = useCallback((...args) => servicesRef.current.signActionIntentForCommand(...args), [servicesRef]);
  const updateMultiplayer = useCallback((...args) => servicesRef.current.updateMultiplayer(...args), [servicesRef]);
  const verifyCurrentPublicCheckpointHash = useCallback((...args) => servicesRef.current.verifyCurrentPublicCheckpointHash(...args), [servicesRef]);
  const verifySequencedActionAudit = useCallback((...args) => servicesRef.current.verifySequencedActionAudit(...args), [servicesRef]);
  const verifySignedActionIntent = useCallback((...args) => servicesRef.current.verifySignedActionIntent(...args), [servicesRef]);
  const waitForActionQuorumVote = useCallback((...args) => servicesRef.current.waitForActionQuorumVote(...args), [servicesRef]);
  const waitForCryptoMaterial = useCallback((...args) => servicesRef.current.waitForCryptoMaterial(...args), [servicesRef]);
  const waitForProtocolResponse = useCallback((...args) => servicesRef.current.waitForProtocolResponse(...args), [servicesRef]);
  const waitForTimeoutVote = useCallback((...args) => servicesRef.current.waitForTimeoutVote(...args), [servicesRef]);
  const waitForZiffleRoute = useCallback((...args) => servicesRef.current.waitForZiffleRoute(...args), [servicesRef]);
  const ziffleCeremonyForOwner = useCallback((...args) => servicesRef.current.ziffleCeremonyForOwner(...args), [servicesRef]);
  const ziffleCeremonyHasObjectOrder = useCallback((...args) => servicesRef.current.ziffleCeremonyHasObjectOrder(...args), [servicesRef]);
  const zifflePositionForObjectId = useCallback((...args) => servicesRef.current.zifflePositionForObjectId(...args), [servicesRef]);
  const ziffleTokensForPosition = useCallback((...args) => servicesRef.current.ziffleTokensForPosition(...args), [servicesRef]);
  function commandObjectStableIds(command) {
    const stableIds = Array.isArray(command?.object_stable_ids)
      ? command.object_stable_ids
      : Array.isArray(command?.objectStableIds)
        ? command.objectStableIds
        : [];
    return stableIds.map((stableId) => {
      const normalized = Number(stableId);
      return Number.isSafeInteger(normalized) && normalized > 0 ? normalized : null;
    });
  }

  function commandObjectHiddenRefs(command) {
    const hiddenRefs = Array.isArray(command?.object_hidden_refs)
      ? command.object_hidden_refs
      : Array.isArray(command?.objectHiddenRefs)
        ? command.objectHiddenRefs
        : [];
    return hiddenRefs.map(normalizeSelectObjectHiddenRef);
  }

  function openingMatchesCommandHiddenRef(opening, hiddenRef) {
    const ref = normalizeSelectObjectHiddenRef(hiddenRef);
    if (!opening || !ref) return false;
    if (ref.owner != null && Number(opening.owner) !== Number(ref.owner)) {
      return false;
    }
    const openingCommitments = [
      opening.publicCommitment,
      opening.public_commitment,
      opening.positionCommitment,
      opening.position_commitment,
      opening.commitment,
    ].map((entry) => String(entry || "")).filter(Boolean);
    const refCommitments = [
      ref.public_commitment,
      ref.commitment,
    ].map((entry) => String(entry || "")).filter(Boolean);
    if (refCommitments.length > 0) {
      const openingPositionCommitment = String(
        opening.positionCommitment || opening.position_commitment || opening.publicCommitment || opening.public_commitment || ""
      );
      const openingDeckHash = ziffleDeckHashFromCommitment(openingPositionCommitment);
      const openingPosition =
        zifflePositionFromCommitment(openingPositionCommitment)
        ?? (opening.position == null ? null : Number(opening.position))
        ?? (opening.publicSlot == null ? null : Number(opening.publicSlot));
      for (const refCommitment of refCommitments) {
        if (openingCommitments.includes(refCommitment)) return true;
        const refDeckHash = ziffleDeckHashFromCommitment(refCommitment);
        const refPosition =
          zifflePositionFromCommitment(refCommitment)
          ?? (ref.public_slot == null ? null : Number(ref.public_slot));
        if (
          refDeckHash
          && openingDeckHash === refDeckHash
          && openingPosition != null
          && refPosition != null
          && Number(openingPosition) === Number(refPosition)
        ) {
          return true;
        }
      }
      return false;
    }
    if (ref.public_slot != null) {
      const openingPosition = opening.position ?? opening.publicSlot ?? opening.public_slot;
      return Number(openingPosition) === Number(ref.public_slot);
    }
    if (ref.slot != null) {
      return Number(opening.slot) === Number(ref.slot);
    }
    return false;
  }

  function filterOpeningsForCommandHiddenRefs(openings = [], command = null) {
    if (command?.type !== "select_objects") return openings;
    const hiddenRefs = commandObjectHiddenRefs(command).filter(Boolean);
    if (hiddenRefs.length === 0) return openings;
    return (openings || []).filter((opening) =>
      hiddenRefs.some((hiddenRef) => openingMatchesCommandHiddenRef(opening, hiddenRef))
    );
  }

  async function currentObjectIdForStableId(stableId) {
    const normalized = Number(stableId);
    if (!Number.isSafeInteger(normalized) || normalized <= 0) return null;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return null;
    }
    let checkpoint = null;
    try {
      checkpoint = await currentGame.exportSyncCheckpoint();
    } catch {
      return null;
    }
    const object = (checkpoint?.objects || []).find((entry) =>
      Number(entry?.stableId ?? entry?.stable_id) === normalized
    );
    const objectId = Number(object?.id);
    return Number.isSafeInteger(objectId) && objectId > 0 ? objectId : null;
  }

  async function currentStableIdForObjectId(objectId) {
    const normalized = Number(objectId);
    if (!Number.isSafeInteger(normalized) || normalized <= 0) return null;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return null;
    }
    let checkpoint = null;
    try {
      checkpoint = await currentGame.exportSyncCheckpoint();
    } catch {
      return null;
    }
    const object = (checkpoint?.objects || []).find((entry) =>
      Number(entry?.id) === normalized
    );
    const stableId = Number(object?.stableId ?? object?.stable_id);
    return Number.isSafeInteger(stableId) && stableId > 0 ? stableId : null;
  }

  async function currentHiddenRefForObjectId(objectId) {
    const hidden = await currentHiddenCardMetadataForObject(objectId);
    let exported = null;
    const currentGame = gameRef.current;
    if (currentGame && typeof currentGame.exportHiddenCardOpening === "function") {
      try {
        exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(objectId));
      } catch {
        exported = null;
      }
    }
    return normalizeSelectObjectHiddenRef({
      owner: hidden?.owner ?? exported?.owner,
      zone: hidden?.zone,
      slot: hidden?.slot ?? exported?.slot,
      commitment: hidden?.commitment ?? exported?.commitment,
      public_slot:
        hidden?.publicSlot
        ?? hidden?.public_slot
        ?? exported?.publicSlot
        ?? exported?.public_slot,
      public_commitment:
        hidden?.publicCommitment
        ?? hidden?.public_commitment
        ?? exported?.publicCommitment
        ?? exported?.public_commitment,
    });
  }

  async function currentObjectIdForHiddenRef(hiddenRef) {
    if (!normalizeSelectObjectHiddenRef(hiddenRef)) return null;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return null;
    }
    let checkpoint = null;
    try {
      checkpoint = await currentGame.exportSyncCheckpoint();
    } catch {
      return null;
    }
    return hiddenObjectIdForHiddenRefFromCheckpoint(checkpoint, hiddenRef);
  }

  async function remapPriorityCommandForLocalHiddenOpening(command, openings = []) {
    if (!command || command.type !== "priority_action" || !command.action_ref) {
      return command;
    }
    const sourceObjectId = actionRefObjectId(command.action_ref);
    if (sourceObjectId == null) return command;

    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.uiState !== "function") return command;

    const liveState = await currentGame.uiState();
    const actions = Array.isArray(liveState?.decision?.actions)
      ? liveState.decision.actions
      : [];
    const sameKindActions = actions.filter(
      (action) => String(action?.action_ref?.kind || "") === String(command.action_ref.kind || "")
    );
    if (sameKindActions.some((action) =>
      Number(actionRefObjectId(action?.action_ref)) === Number(sourceObjectId)
    )) {
      return command;
    }

    const stableId = Number(command.object_stable_id ?? command.objectStableId);
    if (Number.isSafeInteger(stableId) && stableId > 0) {
      const stableObjectId = await currentObjectIdForStableId(stableId);
      const stableAction = sameKindActions.find((action) =>
        Number(actionRefObjectId(action?.action_ref)) === Number(stableObjectId)
      );
      if (stableAction) {
        const localObjectId = actionRefObjectId(stableAction.action_ref);
        const remapped = cloneMultiplayerPayload(command);
        remapped.action_ref = actionRefWithObjectId(command.action_ref, localObjectId);
        remapped.object_id = Number(localObjectId);
        return remapped;
      }
    }

    const hiddenRef = normalizeSelectObjectHiddenRef(
      command.object_hidden_ref ?? command.objectHiddenRef
    );
    if (hiddenRef) {
      const hiddenObjectId = await currentObjectIdForHiddenRef(hiddenRef);
      const hiddenAction = sameKindActions.find((action) =>
        Number(actionRefObjectId(action?.action_ref)) === Number(hiddenObjectId)
      );
      if (hiddenAction) {
        const localObjectId = actionRefObjectId(hiddenAction.action_ref);
        const remapped = cloneMultiplayerPayload(command);
        remapped.action_ref = actionRefWithObjectId(command.action_ref, localObjectId);
        remapped.object_id = Number(localObjectId);
        return remapped;
      }
    }

    const candidateOpenings = (openings || []).filter(
      (opening) => opening?.owner != null && opening?.slot != null && opening?.card
    );
    if (candidateOpenings.length === 0) return command;

    for (const action of sameKindActions) {
      const localObjectId = actionRefObjectId(action?.action_ref);
      if (localObjectId == null || typeof currentGame.exportHiddenCardOpening !== "function") {
        continue;
      }
      let exported = null;
      try {
        exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(localObjectId));
      } catch {
        continue;
      }
      const opening = candidateOpenings.find((entry) =>
        hiddenOpeningMatchesExport(entry, exported)
      );
      if (!opening) continue;
      const remapped = cloneMultiplayerPayload(command);
      remapped.action_ref = actionRefWithObjectId(command.action_ref, localObjectId);
      remapped.object_id = Number(localObjectId);
      return remapped;
    }

    return command;
  }

  async function remapSelectObjectsCommandForLocalHiddenOpening(command, openings = [], actorIndex = null) {
    if (!command || command.type !== "select_objects" || !Array.isArray(command.object_ids)) {
      return command;
    }

    const selectedIds = command.object_ids.map((objectId) => Number(objectId));
    if (selectedIds.length === 0) return command;

    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.uiState !== "function") return command;

    const liveState = await currentGame.uiState();
    if (String(liveState?.decision?.kind || "") !== "select_objects") return command;
    const visibleCandidateIds = new Set(
      (liveState.decision.candidates || [])
        .map((candidate) => Number(candidate?.id))
        .filter((objectId) => Number.isSafeInteger(objectId) && objectId > 0)
    );
    const selectedStableIds = commandObjectStableIds(command);
    const hasStableIds = selectedStableIds.some((stableId) => stableId != null);
    const selectedHiddenRefs = commandObjectHiddenRefs(command);
    const hasHiddenRefs = selectedHiddenRefs.some((hiddenRef) => hiddenRef != null);
    if (
      !hasStableIds
      && !hasHiddenRefs
      && selectedIds.every((objectId) => visibleCandidateIds.has(objectId))
    ) {
      return command;
    }

    let candidateOpenings = (openings || []).filter(
      (opening) => opening?.owner != null && opening?.slot != null && opening?.card
    );
    const actorOwnedOpenings = Number.isInteger(Number(actorIndex))
      ? candidateOpenings.filter((opening) => Number(opening.owner) === Number(actorIndex))
      : [];
    if (actorOwnedOpenings.length > 0) {
      candidateOpenings = actorOwnedOpenings;
    }

    const usedOpenings = new Set();
    let changed = false;
    const objectIds = [];
    for (const [index, selectedId] of selectedIds.entries()) {
      const stableId = selectedStableIds[index];
      const localStableObjectId = stableId == null
        ? null
        : await currentObjectIdForStableId(stableId);
      if (localStableObjectId != null && visibleCandidateIds.has(localStableObjectId)) {
        objectIds.push(localStableObjectId);
        if (Number(localStableObjectId) !== Number(selectedId)) changed = true;
        continue;
      }

      const hiddenRef = selectedHiddenRefs[index];
      const localHiddenRefObjectId = hiddenRef == null
        ? null
        : await currentObjectIdForHiddenRef(hiddenRef);
      if (localHiddenRefObjectId != null && visibleCandidateIds.has(localHiddenRefObjectId)) {
        objectIds.push(localHiddenRefObjectId);
        if (Number(localHiddenRefObjectId) !== Number(selectedId)) changed = true;
        continue;
      }

      if (visibleCandidateIds.has(selectedId)) {
        objectIds.push(selectedId);
        continue;
      }

      if (candidateOpenings.length === 0) {
        objectIds.push(selectedId);
        continue;
      }

      const orderedOpenings = [
        ...candidateOpenings.filter((opening) => Number(opening.objectId ?? opening.object_id) === selectedId),
        ...candidateOpenings.filter((opening) => Number(opening.objectId ?? opening.object_id) !== selectedId),
      ];
      let localObjectId = null;
      for (const opening of orderedOpenings) {
        if (usedOpenings.has(opening)) continue;
        localObjectId = await currentHiddenObjectIdForOpening(opening);
        if (localObjectId == null) continue;
        usedOpenings.add(opening);
        break;
      }
      if (localObjectId != null) {
        objectIds.push(localObjectId);
        changed = true;
      } else {
        objectIds.push(selectedId);
      }
    }

    if (!changed) return command;
    return {
      ...cloneMultiplayerPayload(command),
      object_ids: objectIds,
    };
  }

  async function remapCommandForLocalHiddenOpening(command, openings = [], actorIndex = null) {
    const priorityCommand = await remapPriorityCommandForLocalHiddenOpening(command, openings);
    return remapSelectObjectsCommandForLocalHiddenOpening(priorityCommand, openings, actorIndex);
  }

  async function privateOpeningFromEncryptedProof(proof, requirement = {}, options = {}) {
    const localSeat = resolveLocalCryptoPlayerIndex();
    const viewer = Number(requirement?.viewer ?? proof?.viewer);
    if (!Number.isInteger(viewer) || viewer !== Number(localSeat)) return null;
    if (!proof?.encryptedOpening?.ciphertextHex) return null;
    const { encryptionKeyPair } = await ensureAuditIdentity();
    const payload = await decryptPrivateAuditPayload({
      keyPair: encryptionKeyPair,
      encrypted: proof.encryptedOpening,
    });
    if (
      payload?.matchId
      && String(payload.matchId || "") !== String(currentAuditMatchId())
    ) {
      throw new Error("Private-view opening belongs to a different match");
    }
    const opening = payload?.opening || null;
    if (!opening || opening.owner == null || opening.slot == null || !opening.card) {
      throw new Error("Private-view opening payload is incomplete");
    }
    if (options.persistDisclosure !== false) {
      rememberPrivateViewDisclosure({
        type: "private_view_opening_disclosure",
        matchId: currentAuditMatchId(),
        seq: Number(options.seq ?? requirement?.seq ?? proof?.seq ?? 0),
        requirementId: String(proof.requirementId || payload.requirementId || ""),
        owner: Number(proof.owner ?? payload.owner ?? opening.owner),
        viewer,
        zone: String(proof.zone || payload.zone || ""),
        objectId: Number(proof.objectId ?? payload.objectId ?? requirement.objectId ?? -1),
        plaintextHash: String(proof.encryptedOpening.plaintextHash || ""),
        payload,
      });
    }
	    return sanitizeObjectBoundOpening({
	      ...opening,
	      owner: Number(opening.owner),
	      slot: Number(opening.slot),
	      card: String(opening.card),
      objectId: Number(requirement.objectId ?? opening.objectId ?? proof.objectId),
      timing: "private",
      ...(proof.position != null && opening.position == null
        ? { position: Number(proof.position) }
        : {}),
	      ...(proof.positionCommitment && !opening.positionCommitment
	        ? { positionCommitment: String(proof.positionCommitment) }
	        : {}),
	    });
	  }

  async function privateOpeningFromProof(requirement, audit = {}, options = {}) {
    const proof = (audit.privateViewProofs || []).find((entry) =>
      String(entry?.type || "") === "encrypted_private_opening"
      && (
        String(entry?.requirementId || "") === String(requirement.id || "")
        || (
          Number(entry?.owner) === Number(requirement.owner)
          && Number(entry?.viewer) === Number(requirement.viewer)
          && Number(entry?.objectId) === Number(requirement.objectId)
        )
      )
    );
    return privateOpeningFromEncryptedProof(proof, requirement, {
      ...options,
      seq: options.seq ?? audit.seq,
    });
  }

  async function revealPrivateAuditProofsForLocalViewer(audit = {}, options = {}) {
    const localSeat = resolveLocalCryptoPlayerIndex();
    const openings = [];
    const seen = new Set();
    for (const proof of audit.privateViewProofs || []) {
      if (String(proof?.type || "") !== "encrypted_private_opening") continue;
      if (Number(proof.viewer) !== Number(localSeat)) continue;
      let opening = await privateOpeningFromEncryptedProof(proof, {
        owner: proof.owner,
        viewer: proof.viewer,
        objectId: proof.objectId,
        seq: audit.seq,
      }, {
        seq: options.seq ?? audit.seq,
        persistDisclosure: options.persistDisclosure,
      });
      if (!opening) continue;
      opening = await sanitizeObjectBoundOpening(opening);
      opening = await ensureZiffleOpeningProof(opening, options);
      opening = await sanitizeObjectBoundOpening(opening);
      const key = [
        Number(opening.owner),
        Number(opening.slot),
        Number(opening.objectId ?? proof.objectId ?? -1),
      ].join(":");
      if (seen.has(key)) continue;
      seen.add(key);
      openings.push(opening);
    }
    if (openings.length > 0) {
      await revealAuditOpenings(openings, options);
    }
  }

  async function batchedOwnerPrivateZiffleOpeningsForLocalViewer(requirements = [], options = {}) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleRevealCards !== "function") {
      return { openings: [], handledRequirements: new Set() };
    }
    const localSeat = resolveLocalCryptoPlayerIndex();
    const manifest = privateDeckManifestForOwner(localSeat);
    if (!manifest) {
      return { openings: [], handledRequirements: new Set() };
    }

    const groups = new Map();
    const handledRequirements = new Set();
    for (const requirement of requirements || []) {
      if (String(requirement?.type || "") !== "private_open") continue;
      if (Number(requirement.viewer) !== Number(localSeat)) continue;
      if (Number(requirement.owner) !== Number(localSeat)) continue;
      const positionCommitment = String(requirement.commitment || "");
      const deckHash = ziffleDeckHashFromCommitment(positionCommitment);
      if (!deckHash) {
        continue;
      }
	      const position = zifflePositionFromCommitment(positionCommitment) ?? Number(requirement.slot);
	      if (!Number.isSafeInteger(position) || position < 0) continue;
	      const objectOrderedPosition = zifflePositionForObjectId(
	        localSeat,
	        requirement?.objectId ?? requirement?.object_id,
	        { commitment: positionCommitment }
	      );
	      const ziffleContext = objectOrderedPosition?.ziffleContext || "";
	      const ceremony = ziffleCeremonyForOwner(localSeat, {
	        commitment: positionCommitment,
	        context: ziffleContext,
	      });
      if (!ceremony) {
        continue;
      }
      const key = [
        Number(localSeat),
        String(ceremony.context || ""),
        String(ceremony.deckHash || deckHash),
      ].join(":");
      if (!groups.has(key)) {
        groups.set(key, {
          ceremony,
          entries: [],
        });
      }
      groups.get(key).entries.push({
        requirement,
        position,
        positionCommitment,
      });
      handledRequirements.add(requirement);
    }

	    const openings = [];
	    const seen = new Set();
	    for (const { ceremony, entries } of groups.values()) {
	      if (ziffleCeremonyHasObjectOrder(ceremony)) {
	        for (const entry of entries) {
	          const objectId = Number(entry.requirement.objectId ?? entry.requirement.object_id);
	          const { resolvedRevealSlot } = await resolveCommittedSlotForZifflePosition({
	            owner: localSeat,
	            ceremony,
	            position: entry.position,
	            card: entry.requirement?.card || "",
	            objectId,
	            manifest,
	            options,
	          });
	          if (!resolvedRevealSlot) {
              handledRequirements.delete(entry.requirement);
              continue;
	          }
	          const builtOpening = await buildOpeningFromResolvedCommittedSlot({
	            manifest,
	            resolvedRevealSlot,
	            fallbackObjectId: Number.isSafeInteger(objectId) && objectId >= 0 ? objectId : null,
	            position: entry.position,
	            positionCommitment: entry.positionCommitment,
	            ceremony,
	            timing: "post",
	          });
	          let openingWithPosition = await sanitizeObjectBoundOpening(builtOpening.openingWithPosition);
	          openingWithPosition = await ensureZiffleOpeningProof(openingWithPosition, options);
	          openingWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
	          const originalSlot = builtOpening.originalSlot;
	          const key = [
	            Number(openingWithPosition.owner),
	            Number(openingWithPosition.slot),
	            Number.isSafeInteger(objectId) ? objectId : -1,
	          ].join(":");
	          if (seen.has(key)) continue;
	          rememberLocalRevealedOpening(openingWithPosition, {
		            objectId: openingWithPosition.objectId,
		            position: openingWithPosition.position,
		            positionCommitment: openingWithPosition.positionCommitment,
		            ziffleContext: openingWithPosition.ziffleContext,
		          });
	          rememberZiffleOpeningPosition(localSeat, originalSlot, entry.position);
	          openings.push(openingWithPosition);
	          seen.add(key);
	        }
	        continue;
      }
	      const positions = [...new Set(entries.map((entry) => entry.position))];
	      const tokens = await collectZiffleRevealTokensBatch(ceremony, positions, {
        ...options,
        requirements,
      });
      const reveals = await currentGame.ziffleRevealCards({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(ceremony),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        cardPositions: positions,
        tokens,
      });
      const revealByPosition = new Map(
        (Array.isArray(reveals) ? reveals : []).map((reveal) => [
          Number(reveal.cardPosition),
          Number(reveal.originalSlot),
        ])
      );
      for (const entry of entries) {
	        const shuffleOriginalSlot = revealByPosition.get(Number(entry.position));
	        if (!Number.isSafeInteger(shuffleOriginalSlot) || shuffleOriginalSlot < 0) {
	          throw new Error(`Missing ziffle reveal for position ${Number(entry.position)}`);
	        }
	        let resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	          owner: localSeat,
	          ceremony,
	          shuffleOriginalSlot,
	          shuffleOriginalSlotIsVerified: true,
	          position: entry.position,
		          card: entry.requirement?.card || "",
		          objectId: entry.requirement?.objectId,
		          manifest,
            options,
		        });
	        if (!resolvedRevealSlot && entry.requirement?.card) {
	          resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	            owner: localSeat,
	            ceremony,
	            shuffleOriginalSlot,
	            shuffleOriginalSlotIsVerified: true,
	            position: entry.position,
		            card: "",
		            objectId: entry.requirement?.objectId,
		            manifest,
              options,
		          });
	        }
	        if (!resolvedRevealSlot) {
	          const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
	          const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
	          const interestingIds = [
	            entry.requirement?.objectId ?? entry.requirement?.object_id,
	            beforeOrder[Number(shuffleOriginalSlot)],
	            afterOrder[Number(entry.position)],
	          ].map((id) => Number(id)).filter((id, index, list) =>
	            Number.isSafeInteger(id) && id >= 0 && list.indexOf(id) === index
	          );
	          let candidateDebug = [];
	          try {
	            const checkpoint = await currentGame.exportSyncCheckpoint?.();
	            const objectsById = new Map((checkpoint?.objects || []).map((object) => [
	              Number(object.id),
	              object,
	            ]));
	            candidateDebug = interestingIds.map((id) => {
	              const object = objectsById.get(id) || {};
	              const hidden = object.hiddenCard || object.hidden_card || {};
	              return {
	                id,
	                name: object.name || object.identity?.name || null,
	                zone: object.zone || null,
	                stableId: object.stableId ?? object.stable_id ?? null,
	                hiddenOwner: hidden.owner ?? null,
	                hiddenSlot: hidden.slot ?? null,
	                hiddenCommitment: String(hidden.commitment || "").slice(0, 32),
	                publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
	                publicCommitment: String(hidden.publicCommitment || hidden.public_commitment || "").slice(0, 32),
	              };
	            });
	          } catch {
	            candidateDebug = [];
	          }
	          throw new Error(
	            `Ziffle opening could not resolve committed slot `
	            + `(owner ${Number(localSeat) + 1}, position ${Number(entry.position)}, `
	            + `shuffle slot ${shuffleOriginalSlot}, card ${String(entry.requirement?.card || "")}, `
	            + `ids ${JSON.stringify(interestingIds)}, candidates ${JSON.stringify(candidateDebug)})`
	          );
	        }
	        const originalSlot = Number(resolvedRevealSlot.slot);
	        const secret = (manifest.slotSecrets || []).find(
	          (candidate) => Number(candidate.slot) === Number(originalSlot)
	        );
        if (!secret) {
          throw new Error(`Missing private deck opening for ziffle slot ${Number(originalSlot)}`);
        }
        const opening = await buildDeckSlotOpening({
          manifest,
          slot: originalSlot,
          card: secret.card,
        });
        const objectId = Number(entry.requirement.objectId);
        const key = [
          Number(opening.owner),
          Number(opening.slot),
          Number.isSafeInteger(objectId) ? objectId : -1,
        ].join(":");
        if (seen.has(key)) continue;
        let openingWithPosition = {
	          ...opening,
	          ...(Number.isSafeInteger(objectId) ? { objectId } : {}),
	          ...(resolvedRevealSlot?.shuffleObjectId != null || resolvedRevealSlot?.objectId != null
	            ? { shuffleObjectId: Number(resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId) }
	            : {}),
		          timing: "post",
		          position: Number(entry.position),
		          positionCommitment: entry.positionCommitment,
		          ziffleContext: ziffleContextFromCeremony(ceremony),
	        };
        if (!ziffleCeremonyHasObjectOrder(ceremony)) {
          openingWithPosition.ziffleReveal = buildZiffleOpeningProof({
            opening: openingWithPosition,
		          ceremony,
		          position: Number(entry.position),
		          originalSlot,
		          shuffleOriginalSlot,
		          positionCommitment: entry.positionCommitment,
		          tokens: ziffleTokensForPosition(tokens, entry.position),
		          compact: true,
		        });
        }
        openingWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
        openingWithPosition = await ensureZiffleOpeningProof(openingWithPosition, options);
        openingWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
        rememberLocalRevealedOpening(openingWithPosition, {
	          objectId: openingWithPosition.objectId,
	          position: openingWithPosition.position,
	          positionCommitment: openingWithPosition.positionCommitment,
	          ziffleContext: openingWithPosition.ziffleContext,
	        });
        rememberZiffleOpeningPosition(localSeat, originalSlot, entry.position);
        openings.push(openingWithPosition);
        seen.add(key);
      }
    }
    return { openings, handledRequirements };
  }

  async function privateOpeningsForLocalViewer(requirements = [], audit = {}, options = {}) {
    const localSeat = resolveLocalCryptoPlayerIndex();
    const {
      openings,
      handledRequirements,
    } = await batchedOwnerPrivateZiffleOpeningsForLocalViewer(requirements, options);
    const seen = new Set();
    for (const opening of openings) {
      seen.add([
        Number(opening.owner),
        Number(opening.slot),
        Number(opening.objectId ?? -1),
      ].join(":"));
    }
    for (const requirement of requirements || []) {
      if (handledRequirements.has(requirement)) continue;
      if (String(requirement?.type || "") !== "private_open") continue;
      if (Number(requirement.viewer) !== Number(localSeat)) continue;
      let opening = await privateOpeningFromProof(requirement, audit, options);
      if (!opening && Number(requirement.owner) === Number(localSeat)) {
        opening = (await buildLocalOpeningFromRequirement(requirement, null, options)).opening;
      }
      if (!opening) continue;
      opening = await sanitizeObjectBoundOpening(opening);
      opening = await ensureZiffleOpeningProof(opening, options);
      opening = await sanitizeObjectBoundOpening(opening);
      const key = [
        Number(opening.owner),
        Number(opening.slot),
        Number(opening.objectId ?? requirement.objectId ?? -1),
      ].join(":");
      if (seen.has(key)) continue;
      seen.add(key);
      openings.push(opening);
    }
    return openings;
  }

	  function hiddenPositionBatchRevealFromOpening(opening) {
	    if (!opening || opening.owner == null || opening.slot == null || !opening.card) return null;
	    const owner = Number(opening.owner);
	    const originalSlot = Number(opening.slot);
	    const cardName = String(opening.card || "").trim();
	    const positionCommitment = String(
	      opening.positionCommitment || opening.position_commitment || ""
	    );
	    const position = Number(
	      zifflePositionFromCommitment(positionCommitment) ?? opening.position
	    );
    if (
      !Number.isSafeInteger(owner)
      || owner < 0
      || !Number.isSafeInteger(position)
      || position < 0
      || position > 65535
      || !Number.isSafeInteger(originalSlot)
      || originalSlot < 0
      || originalSlot > 65535
      || !cardName
      || !ziffleDeckHashFromCommitment(positionCommitment)
    ) {
      return null;
    }
    const commitment = String(opening.commitment || "");
    return {
      owner,
      position,
      originalSlot,
      cardName,
      positionCommitment,
      ...(commitment ? { commitment } : {}),
      recomputeDecision: false,
    };
  }

  async function revealPrivateOpeningsForInjection(privateOpenings = [], options = {}) {
    const currentGame = gameRef.current;
    const openingList = Array.isArray(privateOpenings) ? privateOpenings : [];
    if (openingList.length === 0) return;
    if (typeof currentGame?.revealHiddenPositions !== "function") {
      await revealAuditOpenings(openingList, options);
      return;
    }
    const batchItems = [];
    const fallbackOpenings = [];
    for (const opening of openingList) {
      const reveal = hiddenPositionBatchRevealFromOpening(opening);
      if (reveal) {
        batchItems.push({ opening, reveal });
      } else {
        fallbackOpenings.push(opening);
      }
    }
    if (batchItems.length === 0) {
      await revealAuditOpenings(openingList, options);
      return;
    }
    try {
      await currentGame.revealHiddenPositions({
        reveals: batchItems.map((entry) => entry.reveal),
        recomputeDecision: false,
      });
      for (const { opening } of batchItems) {
        rememberLocalRevealedOpening(opening, {
          objectId: opening.objectId,
          position: opening.position,
          positionCommitment: opening.positionCommitment,
          ziffleContext: opening.ziffleContext,
        });
        rememberZiffleOpeningPosition(opening.owner, opening.slot, opening.position);
      }
    } catch {
      await revealAuditOpenings(openingList, options);
      return;
    }
    if (fallbackOpenings.length > 0) {
      await revealAuditOpenings(fallbackOpenings, options);
    }
  }

  async function injectCryptoMaterialForRequirements(requirements = [], audit = {}, options = {}) {
    const currentGame = gameRef.current;
    if (!currentGame) return;
    const seeds = [];
    const libraryShuffles = [];
    for (const requirement of requirements || []) {
      const type = String(requirement?.type || "");
      if (type === "verifiable_shuffle") {
        const proof = (audit.shuffleProofs || []).find((entry) =>
          shuffleProofMatchesRequirement(entry, requirement)
        );
        if (proof?.deckHash) seeds.push(String(proof.deckHash));
        const beforeOrder = normalizeShuffleOrder(proof?.beforeOrder ?? proof?.before_order);
        const afterOrder = normalizeShuffleOrder(proof?.afterOrder ?? proof?.after_order);
        if (beforeOrder.length > 0 && beforeOrder.length === afterOrder.length) {
          libraryShuffles.push({
            owner: Number(proof.owner ?? requirement.owner),
            beforeOrder,
            afterOrder,
          });
        }
      } else if (type === "fair_random") {
        const reveal = (audit.rngReveals || []).find(
          (entry) => String(entry?.requirementId || "") === String(requirement.id || "")
        );
        if (reveal?.combinedSeedHex) seeds.push(String(reveal.combinedSeedHex));
      }
    }
    if (
      (seeds.length > 0 || libraryShuffles.length > 0)
      && typeof currentGame.injectTranscriptRandomSeeds === "function"
    ) {
      await currentGame.injectTranscriptRandomSeeds({
        seeds,
        libraryShuffles,
      });
    }
    const privateOpenings = await privateOpeningsForLocalViewer(requirements, audit, options);
    if (privateOpenings.length > 0) {
      await revealPrivateOpeningsForInjection(privateOpenings, options);
    }
  }

	  const buildLocalPrivateViewProofsForRequirements = useCallback(async (requirements = [], options = {}) => {
	    const currentGame = gameRef.current;
	    const proofs = [];
	    const localSeat = resolveLocalCryptoPlayerIndex();
	    // The viewer (not the deck owner) is responsible for non-owner private
	    // views: it aggregates the other players' reveal tokens locally, so the
	    // owner never decrypts a card it is not entitled to see.
	    const privateOpenRequirements = (requirements || []).filter((requirement) =>
	      String(requirement?.type || "") === "private_open"
	      && !isOwnerPrivateViewRequirement(requirement)
	      && cryptoMaterialResponsibleSeat(requirement) === Number(localSeat)
	    );

	    let liveState = options.liveState || options.uiState || stateRef.current || null;
	    if (!options.liveState && !options.uiState && currentGame && typeof currentGame.uiState === "function") {
	      try {
	        liveState = await currentGame.uiState();
	      } catch {
	        // Use the last React state snapshot if the live engine snapshot is unavailable.
	      }
	    }
	    const viewedCards = liveState?.viewed_cards || liveState?.active_viewed_cards || null;
	    const existingPrivateOpenKeys = new Set(
	      privateOpenRequirements
	        .map((requirement) => {
	          const objectId = Number(requirement?.objectId ?? requirement?.object_id);
	          return Number.isSafeInteger(objectId) && objectId > 0
	            ? `${Number(requirement.owner)}:${Number(requirement.viewer)}:${objectId}`
	            : null;
	        })
	        .filter(Boolean)
	    );
	    const syntheticPrivateOpenRequirements = [];
	    for (const requirement of requirements || []) {
	      if (String(requirement?.type || "") !== "private_view_window") continue;
	      if (isOwnerPrivateViewRequirement(requirement)) continue;
	      if (cryptoMaterialResponsibleSeat(requirement) !== Number(localSeat)) continue;
	      if (!viewedCards || String(viewedCards.visibility || "") !== "private") continue;
	      if (Number(viewedCards.viewer) !== Number(requirement.viewer)) continue;
	      if (Number(viewedCards.subject) !== Number(requirement.owner)) continue;
	      if (String(viewedCards.zone || "").toLowerCase() !== String(requirement.zone || "").toLowerCase()) {
	        continue;
	      }
	      const count = Math.max(0, Number(requirement.count || 0));
	      const cards = Array.isArray(viewedCards.cards) ? viewedCards.cards : [];
	      const cardIds = Array.isArray(viewedCards.card_ids) ? viewedCards.card_ids : [];
	      for (let index = 0; index < cards.length && syntheticPrivateOpenRequirements.length < count; index += 1) {
	        const card = cards[index] || {};
	        const objectId = Number(cardIds[index] ?? card.id ?? card.objectId ?? card.object_id);
	        if (!Number.isSafeInteger(objectId) || objectId <= 0) continue;
	        const key = `${Number(requirement.owner)}:${Number(requirement.viewer)}:${objectId}`;
	        if (existingPrivateOpenKeys.has(key)) continue;
	        let exported = null;
	        if (currentGame && typeof currentGame.exportHiddenCardOpening === "function") {
	          try {
	            exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(objectId));
	          } catch {
	            exported = null;
	          }
	        }
	        const metadata = exported ? null : await currentHiddenCardMetadataForObject(objectId);
	        const slot = Number(exported?.slot ?? metadata?.slot);
	        const commitment = String(exported?.commitment || metadata?.commitment || "");
	        if (!Number.isSafeInteger(slot) || slot < 0 || !commitment) continue;
	        syntheticPrivateOpenRequirements.push({
	          ...cloneMultiplayerPayload(requirement),
	          type: "private_open",
	          id: `${String(requirement.id || "private_view_window")}:object:${objectId}`,
	          objectId,
	          slot,
	          commitment,
	          publicSlot: exported?.publicSlot ?? exported?.public_slot ?? metadata?.publicSlot ?? null,
	          publicCommitment:
	            exported?.publicCommitment
	            || exported?.public_commitment
	            || metadata?.publicCommitment
	            || "",
	          card: String(exported?.card || card.name || ""),
	        });
	        existingPrivateOpenKeys.add(key);
	      }
	    }
	    const allPrivateOpenRequirements = [
	      ...privateOpenRequirements,
	      ...syntheticPrivateOpenRequirements,
	    ];

	    const privateOpeningProofs = (await Promise.all(allPrivateOpenRequirements.map(async (requirement) => {
	      const viewer = Number(requirement.viewer);
	      const recipientPublicKey = auditEncryptionPublicKeyForPlayer(viewer);
	      if (!recipientPublicKey) {
	        throw new Error(`Player ${viewer + 1} is missing a private-view encryption key`);
	      }
	      let exported = null;
	      if (currentGame && typeof currentGame.exportHiddenCardOpening === "function" && requirement.objectId != null) {
	        try {
	          exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(requirement.objectId));
	        } catch {
	          exported = null;
	        }
	      }
	      const {
	        opening,
	        owner,
	        position,
	        positionCommitment,
	      } = await buildLocalOpeningFromRequirement(requirement, exported, options);
	      const privateOpening = await sanitizeObjectBoundOpening({
	        ...opening,
	        objectId: Number(requirement.objectId),
	        timing: "private",
	      });
	      const openingPayload = {
	        type: "private_view_opening",
	        matchId: currentAuditMatchId(),
	        requirementId: String(requirement.id || ""),
	        owner,
	        viewer,
	        zone: String(requirement.zone || ""),
	        objectId: Number(requirement.objectId),
	        opening: privateOpening,
	      };
	      const encryptedOpening = await encryptPrivateAuditPayload({
	        recipientPublicKey,
	        payload: openingPayload,
	      });
	      rememberPrivateViewDisclosure({
	        type: "private_view_opening_disclosure",
	        matchId: currentAuditMatchId(),
	        seq: Number(options.seq ?? 0),
	        requirementId: String(requirement.id || ""),
	        owner,
	        viewer,
	        zone: String(requirement.zone || ""),
	        objectId: Number(requirement.objectId),
	        plaintextHash: String(encryptedOpening.plaintextHash || ""),
	        payload: openingPayload,
	      });
	      const proof = {
	        type: "encrypted_private_opening",
	        requirementId: String(requirement.id || ""),
	        owner,
	        viewer,
	        zone: String(requirement.zone || ""),
	        objectId: Number(requirement.objectId),
	        slot: Number(opening.slot),
	        commitment: opening.commitment,
	        disclosurePolicy: "postgame_or_dispute",
	        encryptedOpening,
	      };
	      if (position != null) {
	        proof.position = position;
	        proof.positionCommitment = positionCommitment;
	      }
	      return proof;
	    }))).filter(Boolean);
	    proofs.push(...privateOpeningProofs);
	    for (const requirement of requirements || []) {
	      if (String(requirement?.type || "") !== "private_view_window") continue;
	      if (isOwnerPrivateViewRequirement(requirement)) continue;
	      if (cryptoMaterialResponsibleSeat(requirement) !== Number(localSeat)) continue;
	      const openingHashes = privateOpeningProofs
	        .filter((entry) =>
	          Number(entry.owner) === Number(requirement.owner)
	          && Number(entry.viewer) === Number(requirement.viewer)
	          && String(entry.zone || "") === String(requirement.zone || "")
	        )
	        .map((entry) => entry.encryptedOpening.plaintextHash);
	      const proof = {
	        type: "encrypted_private_view",
	        requirementId: String(requirement.id || ""),
	        owner: Number(requirement.owner),
	        viewer: Number(requirement.viewer),
	        zone: String(requirement.zone || ""),
	        count: Number(requirement.count || 0),
	        reason: String(requirement.reason || ""),
	        openingHashes,
	        disclosurePolicy: "postgame_or_dispute",
	      };
	      proof.materialHash = await sha256Hex(canonicalMultiplayerPayload(proof));
	      proofs.push(proof);
	    }
	    return proofs;
	  }, [
	    auditEncryptionPublicKeyForPlayer,
	    buildLocalOpeningFromRequirement,
	    currentAuditMatchId,
	    currentHiddenCardMetadataForObject,
	    sanitizeObjectBoundOpening,
	    rememberPrivateViewDisclosure,
	    resolveLocalCryptoPlayerIndex,
	  ]);

	  const buildLocalCryptoMaterialForRequirements = useCallback(async (requirements = [], options = {}) => {
	    const openings = mergeAuditOpenings(
	      await buildLocalOpeningsForCommand({}, requirements, options),
	      await buildLocalRequirementOpeningsForRequirements(requirements, options)
	    );
	    const privateViewProofs = await buildLocalPrivateViewProofsForRequirements(requirements, options);
	    return { openings, privateViewProofs };
	  }, [
	    buildLocalOpeningsForCommand,
	    buildLocalPrivateViewProofsForRequirements,
	    buildLocalRequirementOpeningsForRequirements,
	  ]);

  const derivePostApplyCryptoRequirementsForRequest = useCallback(async ({
    command,
    seq,
    actorIndex,
    liveState,
  }) => {
    const currentGame = gameRef.current;
    if (
      !currentGame
      || typeof currentGame.exportSyncCheckpoint !== "function"
      || typeof currentGame.importSyncCheckpoint !== "function"
      || typeof applySyncedCommand !== "function"
    ) {
      throw new Error("Cannot authorize post-apply hidden-card material without sandbox replay");
    }
    const checkpoint = await currentGame.exportSyncCheckpoint();
    const previousState = cloneMultiplayerPayload(stateRef.current);
    const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
    try {
      const appliedState = await applySyncedCommand(command, "", {
        actorIndex,
        sequence: seq,
        publishState: false,
        validationOnly: true,
      });
      return filterCryptoRequirementsForCommand(
        command,
        liveState,
        freshCryptoRequirementsForSequence(
          seq,
          cryptoRequirementsFromState(appliedState)
        )
      );
    } finally {
      await currentGame.importSyncCheckpoint(
        checkpoint,
        localPlayer ?? multiplayerRef.current.localPlayerIndex ?? 0
      );
      const restoredState = typeof currentGame.uiState === "function"
        ? await currentGame.uiState()
        : previousState;
      stateRef.current = restoredState;
      setState(restoredState);
    }
  }, [
    applySyncedCommand,
    freshCryptoRequirementsForSequence,
    setState,
  ]);

  const authorizedCryptoMaterialRequirementsForRequest = useCallback(async (conn, message) => {
    const session = multiplayerRef.current;
    if (!session.matchStarted) {
      throw new Error("Cryptographic material request received before match start");
    }
    if (String(message?.matchId || "") !== currentAuditMatchId()) {
      throw new Error("Cryptographic material request belongs to a different match");
    }

    const requester = playerIndexForPeerId(conn?.peer);
    if (requester == null) {
      throw new Error("Cryptographic material requester is not a match player");
    }
    if (normalizePlayerIndex(message?.requesterIndex) !== requester) {
      throw new Error("Cryptographic material requester index does not match the peer");
    }

    const actorIndex = normalizePlayerIndex(message?.actorIndex);
    if (actorIndex == null || actorIndex !== requester) {
      throw new Error("Cryptographic material requester is not the acting player");
    }

    const seq = Number(message?.seq);
    const expectedSeq = Number(session.lastAppliedSequence || 0) + 1;
    if (!Number.isSafeInteger(seq) || seq !== expectedSeq) {
      throw new Error("Cryptographic material request has an invalid action sequence");
    }
    if (String(message?.prevStateHash || "") !== String(auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH)) {
      throw new Error("Cryptographic material request is not based on the local transcript head");
    }
    if (!String(message?.publicCheckpointHash || "")) {
      throw new Error("Cryptographic material request is missing the public checkpoint hash");
    }

    const command = message?.command;
    if (!command || typeof command !== "object") {
      throw new Error("Cryptographic material request is missing the signed command preview");
    }

    const liveState = gameRef.current && typeof gameRef.current.uiState === "function"
      ? await gameRef.current.uiState()
      : stateRef.current;
    const decision = liveState?.decision || null;
    if (
      decision?.player !== null
      && decision?.player !== undefined
      && Number(decision.player) !== Number(actorIndex)
    ) {
      throw new Error("Cryptographic material requester is not the active decision player");
    }
    if (!isDecisionCommandCompatible(decision, command)) {
      throw new Error("Cryptographic material request command is not available locally");
    }
    await verifyCurrentPublicCheckpointHash(
      message.publicCheckpointHash,
      "Cryptographic material request public checkpoint does not match local state"
    );

	    const localSeat = resolveLocalCryptoPlayerIndex();
	    const previewedRequirements = filterCryptoRequirementsForCommand(
	      command,
	      liveState,
	      freshCryptoRequirementsForSequence(
	        seq,
	        await previewRequirementsForCommand(command)
	      )
	    );
	    const locallyKnownRequestedPublicOpenRequirements = (
	      Array.isArray(message.requirements) ? message.requirements : []
	    ).filter((requirement) =>
	      String(requirement?.type || "") === "public_open"
	      && Number(requirement.owner) === Number(localSeat)
	      && localRevealedOpeningForRequirement(requirement)
	    );
	    try {
	      const authorizedRequirements = authorizeCryptoMaterialRequestRequirements({
	        localSeat,
	        requestedRequirements: message.requirements,
	        previewedRequirements: [
	          ...previewedRequirements,
	          ...locallyKnownRequestedPublicOpenRequirements,
	        ],
	      });
      const actionIntent = await verifySignedActionIntent(message.actionIntent, {
        matchId: currentAuditMatchId(),
        seq,
        actorIndex,
        prevStateHash: message.prevStateHash,
        preActionPublicCheckpointHash: message.publicCheckpointHash,
        command,
      });
      return { requirements: authorizedRequirements, actionIntent };
    } catch (err) {
      const postApplyRequirements = await derivePostApplyCryptoRequirementsForRequest({
        command,
        seq,
        actorIndex,
        liveState,
      });
      if (postApplyRequirements.length === 0) {
        throw err;
      }
	      const authorizedRequirements = authorizeCryptoMaterialRequestRequirements({
	        localSeat,
	        requestedRequirements: message.requirements,
	        previewedRequirements: [
	          ...previewedRequirements,
	          ...postApplyRequirements,
	          ...locallyKnownRequestedPublicOpenRequirements,
	        ],
	      });
      const actionIntent = await verifySignedActionIntent(message.actionIntent, {
        matchId: currentAuditMatchId(),
        seq,
        actorIndex,
        prevStateHash: message.prevStateHash,
        preActionPublicCheckpointHash: message.publicCheckpointHash,
        command,
      });
      return { requirements: authorizedRequirements, actionIntent };
    }
		  }, [
			    currentAuditMatchId,
	    derivePostApplyCryptoRequirementsForRequest,
		    freshCryptoRequirementsForSequence,
	    localRevealedOpeningForRequirement,
		    resolveLocalCryptoPlayerIndex,
    verifyCurrentPublicCheckpointHash,
	  ]);

  const answerCryptoMaterialRequest = useCallback(async (conn, message) => {
    const requestPerf = {
      request_id: String(message?.requestId || ""),
      requester: message?.requesterIndex == null ? null : Number(message.requesterIndex),
      local_seat: resolveLocalCryptoPlayerIndex(),
      command: summarizePeerCommand(message?.command),
      requested_requirements: summarizeCryptoRequirementsForPerf(message?.requirements || []),
      request_bytes: payloadSizeBytes(message),
    };
    recordPeerSyncPerf("crypto_material_request:received", requestPerf);
    try {
      const { requirements, actionIntent } = await timePeerSyncPhase(
        "crypto_material_request:authorize",
        requestPerf,
        () => authorizedCryptoMaterialRequirementsForRequest(conn, message)
      );
      const authorizedPerf = {
        ...requestPerf,
        authorized_requirements: summarizeCryptoRequirementsForPerf(requirements),
      };
      const requestPayload = cloneMultiplayerPayload(message);
      if (requirements.length > 0) {
        await timePeerSyncPhase("crypto_material_request:remember_intent", authorizedPerf, async () =>
          rememberPendingActionIntent(actionIntent, {
          requestType: "crypto_material_request",
          requestId: String(message.requestId || ""),
          requestPayload,
          requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
          responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
          requestedAtMs: Date.now(),
          })
        );
      }
      setStatus("Generating hidden-card opening payloads for peer action");
      const material = await timePeerSyncPhase(
        "crypto_material_request:build_local_material",
        authorizedPerf,
        () => buildLocalCryptoMaterialForRequirements(requirements, {
          cryptoMaterialRequestId: message.requestId,
          command: message.command || null,
          seq: message.seq,
          actorIndex: message.actorIndex,
          requesterIndex: message.requesterIndex,
          actionIntent,
        })
      );
      recordPeerSyncPerf("crypto_material_request:send_response", {
        ...authorizedPerf,
        material: summarizeCryptoMaterialForPerf(material),
      });
      safeSend(conn, {
        type: "crypto_material_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        openings: material.openings,
        privateViewProofs: material.privateViewProofs,
      });
    } catch (err) {
      recordPeerSyncPerf("crypto_material_request:error_response", {
        ...requestPerf,
        error: toErrorMessage(err),
      });
      safeSend(conn, {
        type: "crypto_material_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }, [
    authorizedCryptoMaterialRequirementsForRequest,
    buildLocalCryptoMaterialForRequirements,
  ]);

  const collectRemoteCryptoMaterialForRequirements = useCallback(async (requirements = [], options = {}) => {
	    const localSeat = resolveLocalCryptoPlayerIndex();
    const materialByOwner = new Map();
    for (const requirement of requirements || []) {
      const type = String(requirement?.type || "");
      if (!["public_open", "private_open", "private_view_window"].includes(type)) continue;
      if (isOwnerPrivateViewRequirement(requirement)) continue;
      // Private views addressed to another player are produced by the viewer
      // (mental-poker flow); everything else by the deck owner.
      const seat = cryptoMaterialResponsibleSeat(requirement);
      if (!Number.isInteger(seat) || seat === Number(localSeat)) continue;
      if (!materialByOwner.has(seat)) materialByOwner.set(seat, []);
      materialByOwner.get(seat).push(requirement);
    }
    const command = options.command || null;
    const seq = options.seq;
    const actorIndex = options.actorIndex;
    const requestPreview = Boolean(options.requestPreview);
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    if (command && requestPreview) {
      for (const player of players) {
        const owner = Number(player?.index);
        if (!Number.isInteger(owner) || owner === Number(localSeat)) continue;
        if (!materialByOwner.has(owner)) materialByOwner.set(owner, []);
      }
    }
    if (materialByOwner.size === 0) {
      recordPeerSyncPerf("collect_remote_crypto_material:skip", {
        command: summarizePeerCommand(options.command),
        requirements: summarizeCryptoRequirementsForPerf(requirements),
        reason: "no_remote_material_required",
      });
      return { openings: [], privateViewProofs: [] };
    }
    const prevStateHash = String(
      options.prevStateHash ?? auditStateHashRef.current ?? INITIAL_AUDIT_STATE_HASH
    );
    const publicCheckpointHash = String(
      options.publicCheckpointHash || await currentPublicAuditCheckpointHash()
    );
    const actionIntent = command
      ? (
          options.actionIntent
          || await signActionIntentForCommand({
            seq,
            actorIndex,
            command,
            prevStateHash,
            preActionPublicCheckpointHash: publicCheckpointHash,
          })
        )
      : null;

    const responses = [];
    recordPeerSyncPerf("collect_remote_crypto_material:start", {
      command: summarizePeerCommand(command),
      seq: seq == null ? null : Number(seq),
      actor: actorIndex == null ? null : Number(actorIndex),
      owners: [...materialByOwner.keys()].map(Number),
      requirements: summarizeCryptoRequirementsForPerf(requirements),
      request_preview: requestPreview,
    });
    // Fan out one request per responsible seat concurrently: a hidden action in
    // a 3-4 player game needs material from multiple peers, and awaiting each in
    // series made the actor's latency scale linearly with player count. The
    // reveal-token collector already uses this pattern; the waiter infra is
    // keyed by requestId so concurrent waiters are safe.
    if (materialByOwner.size > 0) {
      setStatus("Waiting for players to generate hidden-card opening payloads");
    }
    const ownerResponses = await Promise.all(
      [...materialByOwner].map(async ([owner, ownerRequirements]) => {
        const player = players.find((entry) => Number(entry.index) === owner);
        const routePeerId = routePeerIdForPlayer(player);
        if (!routePeerId) {
          throw new Error(`Missing peer route for cryptographic material from player ${owner + 1}`);
        }
        const requestId = makeZiffleRequestId("crypto-material");
        const playerLabel = player.name || `Player ${owner + 1}`;
        const requestedAtMs = Date.now();
        const waiter = waitForCryptoMaterial(requestId, PROTOCOL_RESPONSE_TIMEOUT_MS, {
          peerIndex: owner,
          peerName: playerLabel,
          description:
            `${playerLabel} is generating hidden-card opening payloads for this action.`,
        });
        outboundCryptoMaterialRequestsRef.current.set(requestId, {
          owner,
          targetSeat: owner,
          peerId: routePeerId,
          requirements: cloneMultiplayerPayload(ownerRequirements),
          command: command ? cloneMultiplayerPayload(command) : null,
          seq,
          actorIndex,
          prevStateHash,
          publicCheckpointHash,
          actionIntent: actionIntent ? cloneMultiplayerPayload(actionIntent) : null,
          createdAt: requestedAtMs,
        });
        const requestPayload = {
          type: "crypto_material_request",
          protocolVersion: PROTOCOL_VERSION,
          requestId,
          matchId: currentAuditMatchId(),
          requesterIndex: localSeat,
          ...(seq !== null && seq !== undefined ? { seq: Number(seq) } : {}),
          ...(actorIndex !== null && actorIndex !== undefined ? { actorIndex: Number(actorIndex) } : {}),
          prevStateHash,
          publicCheckpointHash,
          requirements: ownerRequirements,
          ...(command ? { command: cloneMultiplayerPayload(command) } : {}),
          ...(actionIntent ? { actionIntent: cloneMultiplayerPayload(actionIntent) } : {}),
        };
        const responsePerf = {
          owner,
          peer_id: routePeerId,
          request_id: requestId,
          command: summarizePeerCommand(command),
          seq: seq == null ? null : Number(seq),
          actor: actorIndex == null ? null : Number(actorIndex),
          requirements: summarizeCryptoRequirementsForPerf(ownerRequirements),
          request_preview: requestPreview,
          request_bytes: payloadSizeBytes(requestPayload),
        };
        recordPeerSyncPerf("collect_remote_crypto_material:send_request", responsePerf);
        await sendDirectProtocolMessage(routePeerId, requestPayload);
        try {
          const response = await timePeerSyncPhase(
            "collect_remote_crypto_material:wait_response",
            responsePerf,
            () => waitForProtocolResponse(waiter, {
              basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
              targetPlayerIndex: owner,
              targetPeerId: player.peerId,
              requesterIndex: localSeat,
              requestType: requestPayload.type,
              requestId,
              requestPayload,
              responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
              requestedAtMs,
            })
          );
          recordPeerSyncPerf("collect_remote_crypto_material:received_response", {
            ...responsePerf,
            material: summarizeCryptoMaterialForPerf(response),
          });
          return response;
        } finally {
          outboundCryptoMaterialRequestsRef.current.delete(requestId);
        }
      })
    );
    responses.push(...ownerResponses);

    const merged = {
      openings: mergeAuditOpenings(...responses.map((response) => response.openings || [])),
      privateViewProofs: mergePrivateViewProofs(
        ...responses.map((response) => response.privateViewProofs || [])
      ),
    };
    recordPeerSyncPerf("collect_remote_crypto_material:done", {
      command: summarizePeerCommand(command),
      seq: seq == null ? null : Number(seq),
      actor: actorIndex == null ? null : Number(actorIndex),
      owners: [...materialByOwner.keys()].map(Number),
      material: summarizeCryptoMaterialForPerf(merged),
    });
    return merged;
  }, [
    currentAuditMatchId,
    currentPublicAuditCheckpointHash,
    makeZiffleRequestId,
    resolveLocalCryptoPlayerIndex,
    setStatus,
    waitForCryptoMaterial,
    waitForZiffleRoute,
  ]);

  const resolvePeerResyncWaitersIfIdle = useCallback(() => {
    if (resyncingPeerIdsRef.current.size > 0) return;
    const waiters = resyncWaitersRef.current;
    resyncWaitersRef.current = [];
    for (const resolve of waiters) {
      resolve();
    }
  }, []);

  const finishPeerResync = useCallback(
    (peerId) => {
      if (peerId) {
        resyncingPeerIdsRef.current.delete(peerId);
      }
      resolvePeerResyncWaitersIfIdle();
    },
    [resolvePeerResyncWaitersIfIdle]
  );

  const clearAllPeerResyncs = useCallback(() => {
    resyncingPeerIdsRef.current.clear();
    resolvePeerResyncWaitersIfIdle();
  }, [resolvePeerResyncWaitersIfIdle]);

  const waitForPeerResyncs = useCallback(() => {
    if (resyncingPeerIdsRef.current.size === 0) {
      return Promise.resolve();
    }
    return new Promise((resolve) => {
      resyncWaitersRef.current.push(resolve);
    });
  }, []);

  const teardownPeer = useCallback(() => {
    clearAllConnectionHeartbeats();
    clearAllPeerResyncs();
    clearAllPendingActionIntents();
    ignoredActionIntentKeysRef.current.clear();
    for (const challenge of reconnectChallengesRef.current.values()) {
      if (challenge?.timeoutId) {
        window.clearTimeout(challenge.timeoutId);
      }
    }
    reconnectChallengesRef.current.clear();
    privateViewDisclosuresRef.current.clear();
    awaitingStateResyncRef.current = false;
    relayedActionIdsRef.current.clear();
    applyingSequencedActionsRef.current.clear();
    pendingSequencedActionsRef.current.clear();
    drainingPendingSequencedActionsRef.current = false;
    localZiffleRevealInFlightRef.current = null;

    const hostConn = hostConnectionRef.current;
    hostConnectionRef.current = null;
    if (hostConn) {
      try {
        hostConn.close();
      } catch (err) {
        void err;
      }
    }

    for (const conn of clientConnectionsRef.current.values()) {
      try {
        conn.close();
      } catch (err) {
        void err;
      }
    }
    clientConnectionsRef.current.clear();

    for (const conn of peerConnectionsRef.current.values()) {
      try {
        conn.close();
      } catch (err) {
        void err;
      }
    }
    peerConnectionsRef.current.clear();

    const peer = peerRef.current;
    peerRef.current = null;
    if (peer) {
      try {
        peer.destroy();
      } catch (err) {
        void err;
      }
    }
  }, [clearAllConnectionHeartbeats, clearAllPeerResyncs]);

  const leaveLobby = useCallback(
    (message = "Left lobby", options = {}) => {
      if (options.clearStoredPlayer !== false) {
        clearStoredPlayerIndex(multiplayerRef.current.lobbyId || multiplayerRef.current.hostPeerId);
      }
      teardownPeer();
      matchStartPayloadRef.current = null;
      liveZiffleCeremoniesRef.current.clear();
      localZiffleCeremonyLookupRef.current.clear();
      ziffleRevealTokenCacheRef.current.clear();
      verifiedAuditOpeningsRef.current.clear();
      verifiedShuffleProofsRef.current.clear();
      ziffleOpeningPositionsRef.current.clear();
      localRevealedOpeningsRef.current.clear();
      localDisconnectObservationsRef.current.clear();
      clearAllPendingActionIntents();
      ignoredActionIntentKeysRef.current.clear();
      reconnectChallengesRef.current.clear();
      actionHistoryRef.current = [];
      actionCryptoRequirementsRef.current.clear();
      matchClockObservationExemptSequenceRef.current = 0;
      initialPublicCheckpointHashRef.current = "";
      updateMultiplayer(createEmptyState());
      if (message) {
        setStatus(message, Boolean(options.isError));
      }
    },
    [setStatus, teardownPeer, updateMultiplayer]
  );

  const broadcastToClients = useCallback((payload) => {
    for (const conn of clientConnectionsRef.current.values()) {
      safeSend(conn, payload);
    }
  }, []);

  const broadcastMatchPresence = useCallback((peerId, connected, details = {}) => {
    const player = (multiplayerRef.current.players || []).find((entry) =>
      String(entry?.peerId || "") === String(peerId || "")
    );
    broadcastToClients({
      type: "player_presence",
      protocolVersion: PROTOCOL_VERSION,
      peerId,
      playerIndex: player?.index == null ? undefined : Number(player.index),
      connected: Boolean(connected),
      disconnectedAtMs: details.disconnectedAtMs == null
        ? undefined
        : Number(details.disconnectedAtMs),
      autoForfeitAtMs: details.autoForfeitAtMs == null
        ? undefined
        : Number(details.autoForfeitAtMs),
    });
  }, [broadcastToClients]);

  const sendMatchStartToClients = useCallback((payload) => {
    for (const conn of clientConnectionsRef.current.values()) {
      safeSend(conn, redactedMatchPayloadForPeer(payload, conn.peer));
    }
  }, []);

  const buildHostedResyncPayload = useCallback(() => {
    const session = multiplayerRef.current;
    const basePayload = matchStartPayloadRef.current;
    if (session.role !== "host" || !session.matchStarted || !basePayload) {
      return null;
    }

    const clockRuntime = matchClockRef.current;
    const clockState = stateRef.current;
    const clockPolicy = normalizeMatchClockPolicy(
      clockRuntime.policy || matchClockConfigRef.current
    );
    const clockPlayerCount = Math.max(
      0,
      Number(
        (session.players || []).length
        || clockState?.players?.length
        || clockRuntime.playerCount
        || 0
      )
    );
    const activePlayerIndex = matchClockActivePlayerFromState(clockState);
    const clockNowMs = nowMonotonicMs();
    const activeChanged = Number(clockRuntime.activePlayerIndex) !== Number(activePlayerIndex);
    matchClockRef.current = {
      policy: clockPolicy,
      playerCount: clockPlayerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining(
        clockRuntime.baseRemainingMsByPlayer,
        clockPlayerCount,
        clockPolicy.initialMs
      ),
      activePlayerIndex,
      epochStartedAtMs: activePlayerIndex == null
        ? null
        : (activeChanged ? clockNowMs : clockRuntime.epochStartedAtMs ?? clockNowMs),
      clockHash: String(clockRuntime.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(clockRuntime.lastSequence || 0),
    };
    const currentMatchClock = createMatchClockSnapshot({
      policy: matchClockRef.current.policy,
      playerCount: matchClockRef.current.playerCount,
      baseRemainingMsByPlayer: matchClockRef.current.baseRemainingMsByPlayer,
      activePlayerIndex: matchClockRef.current.activePlayerIndex,
      epochStartedAtMs: matchClockRef.current.epochStartedAtMs,
      clockHash: matchClockRef.current.clockHash,
      lastSequence: matchClockRef.current.lastSequence,
      nowMs: clockNowMs,
    });

    const currentHostPeerId = session.localPeerId || session.hostPeerId || basePayload.hostPeerId || "";
    const currentHostPlayerIndex = resolveLocalPlayerIndex(session);
    const currentPlayers = toPublicPlayers(session.players).map((player) =>
      currentHostPlayerIndex != null && Number(player.index) === Number(currentHostPlayerIndex)
        ? {
            ...player,
            currentPeerId: currentHostPeerId || player.currentPeerId || player.peerId,
            connected: true,
          }
        : player
    );

    return {
      ...cloneMultiplayerPayload(basePayload),
      protocolVersion: PROTOCOL_VERSION,
      lobbyId: basePayload.lobbyId || session.lobbyId || "",
      hostPeerId: basePayload.hostPeerId || "",
      currentHostPeerId,
      currentHostPlayerIndex:
        currentHostPlayerIndex == null ? undefined : Number(currentHostPlayerIndex),
      currentPlayers,
      currentMatchClock,
      format: normalizeMatchFormat(basePayload.format || session.format),
      startingLife: Number(basePayload.startingLife || session.startingLife || 20),
      securityMode: sessionSecurityMode(session, matchPayloadSecurityMode(basePayload)),
      players: cloneMultiplayerPayload(basePayload.players || []),
    };
  }, []);

  const sendHostedStateMessage = useCallback(
    async (conn, payload) => {
      const session = multiplayerRef.current;
      const peerPlayer = session.players.find((entry) =>
        String(entry.peerId || "") === String(conn.peer || "")
        || String(entry.currentPeerId || "") === String(conn.peer || "")
      );
      if (!peerPlayer) {
        throw new Error("Cannot resync an unknown peer");
      }
      const peerIndex = normalizePlayerIndex(peerPlayer?.index);
      const currentGame = gameRef.current;
	      if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
	        throw new Error("Game engine cannot export a resync checkpoint");
	      }
	      const securityMode = sessionSecurityMode(
	        session,
	        matchPayloadSecurityMode(payload.match, MULTIPLAYER_SECURITY_VERIFIED)
	      );
	      const checkpoint =
	        isVerifiedMultiplayerSecurityMode(securityMode)
	        && peerIndex != null
	        && typeof currentGame.exportRedactedSyncCheckpoint === "function"
	          ? await currentGame.exportRedactedSyncCheckpoint(peerIndex)
	          : await currentGame.exportSyncCheckpoint();
	      const serializedCheckpoint = cloneMultiplayerPayload(checkpoint);
	      const actions = (actionHistoryRef.current || [])
	        .map((entry) => cloneMultiplayerPayload(entry));
	      const lastSequence = Number(actions.at(-1)?.seq ?? 0);
	      const resyncEnvelope = isVerifiedMultiplayerSecurityMode(securityMode)
        ? await buildSignedResyncEnvelope({
            keyPair: auditKeyPairRef.current,
            matchId: payload.match?.auditMatchId || currentAuditMatchId(),
            signer: resolveLocalPlayerIndex(session) ?? 0,
            lastSequence,
            finalStateHash: auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH,
            checkpoint: serializedCheckpoint,
            actions,
          })
        : null;
      safeSend(conn, {
        ...payload,
        lastSequence,
        match: redactedMatchPayloadForPeer(
          {
            ...payload.match,
            securityMode,
          },
          conn.peer,
          peerIndex
        ),
        checkpoint: serializedCheckpoint,
        actions,
        ...(resyncEnvelope ? { resyncEnvelope } : {}),
      });
    },
    [currentAuditMatchId]
  );

  function sequencedActionRelayKey(message) {
    if (isTrustedMultiplayerSecurityMode(message?.securityMode)) {
      return [
        MULTIPLAYER_SECURITY_TRUSTED,
        Number(message?.seq || 0),
        Number(message?.actorIndex ?? -1),
        canonicalMultiplayerPayload(message?.command || {}),
      ].join(":");
    }
    return [
      String(message?.audit?.matchId || currentAuditMatchId()),
      Number(message?.seq || 0),
      String(message?.audit?.signature || ""),
    ].join(":");
  }

  function relaySequencedAction(message) {
    if (!message || message.type !== "apply_action") return;
    const relayKey = sequencedActionRelayKey(message);
    if (relayedActionIdsRef.current.has(relayKey)) return;
    relayedActionIdsRef.current.add(relayKey);

    const session = multiplayerRef.current;
    ensureDirectPeerConnections(session.players || []);
    for (const player of session.players || []) {
      const peerId = routePeerIdForPlayer(player);
      if (!peerId || peerId === session.localPeerId) continue;
      const sent = sendDirectPeerMessage(peerId, {
        ...cloneMultiplayerPayload(message),
        relayedBy: session.localPeerId || "",
      });
      recordPeerSyncPerf("apply_action:relay_send", {
        seq: Number(message.seq || 0),
        actor: Number(message.actorIndex ?? message.audit?.actor ?? -1),
        target_player_index: player?.index == null ? null : Number(player.index),
        target_peer_id: peerId,
        local_peer_id: String(session.localPeerId || ""),
        role: String(session.role || ""),
        sent,
      });
    }
  }

  function playerCountForClock(uiState) {
    const sessionCount = multiplayerRef.current?.players?.length || 0;
    const stateCount = uiState?.players?.length || 0;
    return Math.max(0, Number(sessionCount || stateCount || matchClockRef.current.playerCount || 0));
  }

  function runtimeMatchClockSnapshot(nowMs = nowMonotonicMs()) {
    const runtime = matchClockRef.current;
    return createMatchClockSnapshot({
      policy: runtime.policy || matchClockConfigRef.current,
      playerCount: runtime.playerCount,
      baseRemainingMsByPlayer: runtime.baseRemainingMsByPlayer,
      activePlayerIndex: runtime.activePlayerIndex,
      epochStartedAtMs: runtime.epochStartedAtMs,
      clockHash: runtime.clockHash || INITIAL_MATCH_CLOCK_HASH,
      lastSequence: runtime.lastSequence || 0,
      nowMs,
    });
  }

  function publishMatchClockSnapshot(snapshot) {
    updateMultiplayer((prev) => ({
      ...prev,
      matchClock: snapshot,
      actionTimer: actionTimerSnapshotFromMatchClock(snapshot),
    }));
    return snapshot;
  }

  function resetMatchClockForMatch(payload, uiState) {
    const policy = matchClockPolicyFromPayload(payload, matchClockConfigRef.current);
    matchClockConfigRef.current = policy;
    const playerCount = Math.max(
      payload?.players?.length || 0,
      playerCountForClock(uiState)
    );
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining([], playerCount, policy.initialMs),
      activePlayerIndex: null,
      epochStartedAtMs: null,
      clockHash: INITIAL_MATCH_CLOCK_HASH,
      lastSequence: 0,
    };
    return updateMatchClockForState(uiState, { reset: true, policy });
  }

  function updateMatchClockForState(uiState, options = {}) {
    const policy = normalizeMatchClockPolicy(options.policy || matchClockConfigRef.current);
    matchClockConfigRef.current = policy;
    const playerCount = playerCountForClock(uiState);
    const activePlayerIndex = matchClockActivePlayerFromState(uiState);
    const current = matchClockRef.current;
    const reset = Boolean(options.reset);
    const baseRemainingMsByPlayer = reset
      ? normalizeMatchClockRemaining([], playerCount, policy.initialMs)
      : normalizeMatchClockRemaining(
          current.baseRemainingMsByPlayer,
          playerCount,
          policy.initialMs
        );
    const nowMs = nowMonotonicMs();
    const activeChanged = Number(current.activePlayerIndex) !== Number(activePlayerIndex);
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer,
      activePlayerIndex,
      epochStartedAtMs: activePlayerIndex == null
        ? null
        : (reset || activeChanged ? nowMs : current.epochStartedAtMs ?? nowMs),
      clockHash: reset
        ? INITIAL_MATCH_CLOCK_HASH
        : String(current.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: reset ? 0 : Number(current.lastSequence || 0),
    };
    return publishMatchClockSnapshot(runtimeMatchClockSnapshot(nowMs));
  }

  function currentMatchClockSnapshot() {
    return runtimeMatchClockSnapshot();
  }

  async function buildMatchClockAuditForCommand({
    command,
    seq,
    actorIndex,
    uiState,
  }) {
    const snapshot = updateMatchClockForState(uiState);
    if (!snapshot.enabled) return null;
    const runtime = matchClockRef.current;
    const activePlayer = snapshot.activePlayerIndex;
    const isTimeoutForfeit = isActionTimeoutForfeitCommand(command);
    const isDisconnectForfeit = isDisconnectTimeoutForfeitCommand(command);
    const isProtocolTimeoutForfeit = isProtocolResponseTimeoutForfeitCommand(command);
    const baseRemaining = normalizeMatchClockRemaining(
      runtime.baseRemainingMsByPlayer,
      runtime.playerCount,
      runtime.policy.initialMs
    );
    let elapsedMs = 0;
    if (activePlayer != null) {
      const observedElapsed = Math.max(
        0,
        Math.floor(nowMonotonicMs() - Number(runtime.epochStartedAtMs ?? nowMonotonicMs()))
      );
      elapsedMs = isTimeoutForfeit
        ? Number(baseRemaining[activePlayer] || 0)
        : (isDisconnectForfeit || isProtocolTimeoutForfeit)
          ? 0
          : Math.min(Number(baseRemaining[activePlayer] || 0), observedElapsed);
    }
    const clock = {
      type: MATCH_CLOCK_AUDIT_TYPE,
      version: 1,
      matchId: currentAuditMatchId(),
      seq: Number(seq),
      actor: Number(actorIndex),
      reason: isTimeoutForfeit
        ? "timeout_claim"
        : isDisconnectForfeit
          ? "disconnect_timeout_claim"
          : isProtocolTimeoutForfeit
            ? "protocol_response_timeout_claim"
            : "action",
      policy: matchClockPolicyPayload(runtime.policy),
      activePlayer: activePlayer == null ? null : Number(activePlayer),
      elapsedMs,
      remainingMsByPlayer: debitMatchClockRemaining(baseRemaining, activePlayer, elapsedMs),
      previousClockHash: String(runtime.clockHash || INITIAL_MATCH_CLOCK_HASH),
      basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
    };
    clock.clockHash = await matchClockAuditHash(clock);
    return clock;
  }

	  async function verifyMatchClockAuditForAction({
	    clock,
	    command,
	    seq,
	    actorIndex,
	    uiState,
	    skewMs = 0,
	    enforceObservationBounds = true,
	    enforceUnderreportBounds = enforceObservationBounds,
	    skipTimeoutCertificate = false,
	  }) {
    const snapshot = updateMatchClockForState(uiState);
    if (!snapshot.enabled) {
      if (clock) throw new Error("Sequenced audit unexpectedly included match clock data");
      return;
    }
    if (!clock || typeof clock !== "object") {
      throw new Error("Sequenced audit is missing match clock data");
    }
    if (String(clock.type || "") !== MATCH_CLOCK_AUDIT_TYPE) {
      throw new Error("Sequenced audit has unsupported match clock data");
    }
    if (String(clock.matchId || "") !== currentAuditMatchId()) {
      throw new Error("Match clock audit belongs to a different match");
    }
    if (Number(clock.seq) !== Number(seq) || Number(clock.actor) !== Number(actorIndex)) {
      throw new Error("Match clock audit does not match broadcast action");
    }
    const runtime = matchClockRef.current;
    if (String(clock.previousClockHash || "") !== String(runtime.clockHash || INITIAL_MATCH_CLOCK_HASH)) {
      throw new Error("Match clock hash chain does not match local transcript");
    }
    const expectedHash = await matchClockAuditHash(clock);
    if (expectedHash !== String(clock.clockHash || "")) {
      throw new Error("Match clock audit hash is invalid");
    }
    const policy = normalizeMatchClockPolicy(clock.policy || {});
    if (
      Number(policy.initialMs) !== Number(runtime.policy.initialMs)
      || Number(policy.graceMs) !== Number(runtime.policy.graceMs)
    ) {
      throw new Error("Match clock policy does not match the match genesis");
    }
    const activePlayer = snapshot.activePlayerIndex;
    const clockActivePlayer = clock.activePlayer == null ? null : Number(clock.activePlayer);
    if (clockActivePlayer !== activePlayer) {
      throw new Error("Match clock active player does not match the current decision");
    }
    const baseRemaining = normalizeMatchClockRemaining(
      runtime.baseRemainingMsByPlayer,
      runtime.playerCount,
      runtime.policy.initialMs
    );
    const rawElapsedMs = Number(clock.elapsedMs ?? 0);
    if (!Number.isFinite(rawElapsedMs) || rawElapsedMs < 0) {
      throw new Error("Match clock elapsed time is invalid");
    }
    const elapsedMs = Math.floor(rawElapsedMs);
    if (activePlayer == null && elapsedMs !== 0) {
      throw new Error("Match clock elapsed time requires an active decision");
    }
    const expectedRemaining = debitMatchClockRemaining(baseRemaining, activePlayer, elapsedMs);
    const actualRemaining = normalizeMatchClockRemaining(
      clock.remainingMsByPlayer,
      runtime.playerCount,
      runtime.policy.initialMs
    );
    if (activePlayer != null) {
      const activeRemaining = Number(baseRemaining[activePlayer] || 0);
      if (elapsedMs > activeRemaining) {
        throw new Error("Match clock elapsed time exceeds remaining time");
      }
      const observedElapsedMs = snapshot.startedAtMs == null
        ? 0
        : Math.max(0, Math.floor(nowMonotonicMs() - Number(snapshot.startedAtMs)));
      const signedElapsedIsSelfDisadvantageous = isDisadvantageousActivePlayerClockAdvance({
        actorIndex,
        activePlayerIndex: activePlayer,
        elapsedMs,
        observedElapsedMs,
        previousRemainingMsByPlayer: baseRemaining,
        submittedRemainingMsByPlayer: actualRemaining,
        isTimeoutForfeit: isActionTimeoutForfeitCommand(command),
        skewMs,
      });
      if (
        enforceObservationBounds
        && elapsedMs > observedElapsedMs + Number(skewMs || 0)
        && !signedElapsedIsSelfDisadvantageous
      ) {
        throw new Error("Match clock elapsed time exceeds local observation");
      }
      const underreportSkewMs = Math.max(
        Number(skewMs || 0),
        MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS
      );
	      if (
	        enforceUnderreportBounds
	        && elapsedMs + underreportSkewMs < observedElapsedMs
	      ) {
        throw new Error("Match clock elapsed time is below local observation");
      }
    }
    if (canonicalJson(expectedRemaining) !== canonicalJson(actualRemaining)) {
      throw new Error("Match clock remaining time does not match elapsed time");
    }
    if (!isActionTimeoutForfeitCommand(command)) return;
    const forfeitedPlayer = Number(command.player);
    if (activePlayer == null || forfeitedPlayer !== activePlayer) {
      throw new Error("Timeout forfeit does not match the active match clock");
    }
    if (actualRemaining[activePlayer] !== 0) {
      throw new Error("Timeout forfeit did not exhaust the player's match clock");
    }
    if (!skipTimeoutCertificate) {
      await verifyTimeoutCertificate(command, uiState);
    }
    const liveRemaining = Number(snapshot.remainingMs ?? runtime.policy.initialMs);
    if (liveRemaining > Number(runtime.policy.graceMs || 0) + Number(skewMs || 0)) {
      throw new Error("Match clock has not expired");
    }
  }

  function commitMatchClockAudit(clock, uiState) {
    if (!clock || typeof clock !== "object") {
      return updateMatchClockForState(uiState);
    }
    const policy = normalizeMatchClockPolicy(clock.policy || matchClockConfigRef.current);
    const playerCount = playerCountForClock(uiState);
    matchClockConfigRef.current = policy;
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining(
        clock.remainingMsByPlayer,
        playerCount,
        policy.initialMs
      ),
      activePlayerIndex: null,
      epochStartedAtMs: null,
      clockHash: String(clock.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(clock.seq || 0),
    };
    return updateMatchClockForState(uiState, { policy });
  }

  function stageLocalMatchClockAudit(clock) {
    if (!clock || typeof clock !== "object") return null;
    const previous = {
      ...matchClockRef.current,
      baseRemainingMsByPlayer: [...(matchClockRef.current.baseRemainingMsByPlayer || [])],
    };
    const policy = normalizeMatchClockPolicy(clock.policy || matchClockConfigRef.current);
    const playerCount = Math.max(
      previous.playerCount || 0,
      Array.isArray(clock.remainingMsByPlayer) ? clock.remainingMsByPlayer.length : 0
    );
    matchClockConfigRef.current = policy;
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining(
        clock.remainingMsByPlayer,
        playerCount,
        policy.initialMs
      ),
      activePlayerIndex: null,
      epochStartedAtMs: null,
      clockHash: String(clock.clockHash || previous.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(clock.seq || previous.lastSequence || 0),
    };
    publishMatchClockSnapshot(runtimeMatchClockSnapshot());
    return previous;
  }

  function restoreMatchClockRuntime(runtime, uiState) {
    if (!runtime) return null;
    matchClockRef.current = {
      ...runtime,
      baseRemainingMsByPlayer: [...(runtime.baseRemainingMsByPlayer || [])],
    };
    return updateMatchClockForState(uiState || stateRef.current, {
      policy: runtime.policy || matchClockConfigRef.current,
    });
  }

  function latestMatchClockAuditFromActions(actions = []) {
    for (let index = (actions || []).length - 1; index >= 0; index -= 1) {
      const entry = actions[index] || {};
      const clock = entry.clock || entry.audit?.clock || null;
      if (clock && typeof clock === "object") return clock;
    }
    return null;
  }

  function restoreMatchClockRuntimeFromActionTranscript(actions = [], uiState, matchPayload = null) {
    const clock = latestMatchClockAuditFromActions(actions);
    const finalSequence = Number(actions.at(-1)?.seq || 0);
    if (!clock) {
      const policy = matchClockPolicyFromPayload(matchPayload, matchClockConfigRef.current);
      const playerCount = Math.max(
        Array.isArray(matchPayload?.players) ? matchPayload.players.length : 0,
        playerCountForClock(uiState)
      );
      matchClockConfigRef.current = policy;
      matchClockRef.current = {
        policy,
        playerCount,
        baseRemainingMsByPlayer: normalizeMatchClockRemaining([], playerCount, policy.initialMs),
        activePlayerIndex: null,
        epochStartedAtMs: null,
        clockHash: INITIAL_MATCH_CLOCK_HASH,
        lastSequence: finalSequence,
      };
      return updateMatchClockForState(uiState, { policy });
    }

    const policy = normalizeMatchClockPolicy(clock.policy || matchClockConfigRef.current);
    const playerCount = Math.max(
      playerCountForClock(uiState),
      Array.isArray(clock.remainingMsByPlayer) ? clock.remainingMsByPlayer.length : 0
    );
    matchClockConfigRef.current = policy;
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: normalizeMatchClockRemaining(
        clock.remainingMsByPlayer,
        playerCount,
        policy.initialMs
      ),
      activePlayerIndex: null,
      epochStartedAtMs: null,
      clockHash: String(clock.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(clock.seq || finalSequence),
    };
    return updateMatchClockForState(uiState, { policy });
  }

  function alignMatchClockObservationFromHostSnapshot(hostSnapshot, uiState) {
    if (!hostSnapshot || typeof hostSnapshot !== "object") {
      return updateMatchClockForState(uiState);
    }
    const runtime = matchClockRef.current;
    const policy = normalizeMatchClockPolicy(runtime.policy || matchClockConfigRef.current);
    const playerCount = playerCountForClock(uiState);
    const activePlayerIndex = matchClockActivePlayerFromState(uiState);
    const hostClockHash = String(hostSnapshot.clockHash || "");
    const hostSequence = Number(hostSnapshot.lastSequence || 0);
    if (
      hostClockHash
      && hostClockHash !== String(runtime.clockHash || INITIAL_MATCH_CLOCK_HASH)
    ) {
      return updateMatchClockForState(uiState);
    }
    if (hostSequence && hostSequence !== Number(runtime.lastSequence || 0)) {
      return updateMatchClockForState(uiState);
    }
    const baseRemaining = normalizeMatchClockRemaining(
      runtime.baseRemainingMsByPlayer,
      playerCount,
      policy.initialMs
    );
    const hostRemaining = normalizeMatchClockRemaining(
      hostSnapshot.remainingMsByPlayer,
      playerCount,
      policy.initialMs
    );
    const nowMs = nowMonotonicMs();
    const activeElapsed = activePlayerIndex == null
      ? 0
      : Math.max(
          0,
          Number(baseRemaining[activePlayerIndex] || 0)
            - Number(hostRemaining[activePlayerIndex] || 0)
        );
    matchClockConfigRef.current = policy;
    matchClockRef.current = {
      policy,
      playerCount,
      baseRemainingMsByPlayer: baseRemaining,
      activePlayerIndex,
      epochStartedAtMs: activePlayerIndex == null ? null : nowMs - activeElapsed,
      clockHash: String(runtime.clockHash || INITIAL_MATCH_CLOCK_HASH),
      lastSequence: Number(runtime.lastSequence || 0),
    };
    return publishMatchClockSnapshot(runtimeMatchClockSnapshot(nowMs));
  }

	  async function signTimeoutVoteForSnapshot({
    basisSequence,
    forfeitedPlayer,
    activePlayer,
    clockHash,
    remainingMs,
  }) {
    const { keyPair } = await ensureAuditIdentity();
    const voter = resolveLocalPlayerIndex(multiplayerRef.current);
    if (voter == null || Number(voter) === Number(forfeitedPlayer)) {
      throw new Error("Timed-out player cannot sign their own timeout certificate");
    }
    const payload = timeoutVotePayload({
      matchId: currentAuditMatchId(),
      basisSequence,
      forfeitedPlayer,
      activePlayer,
      clockHash,
      remainingMs,
      voter,
    });
    return {
      ...payload,
      signatureAlgorithm: "ecdsa-p256-sha256",
      signature: await signAuditPayload(keyPair, payload),
    };
  }

  async function verifyTimeoutVote(vote, expected) {
    const voter = normalizePlayerIndex(vote?.voter);
    if (voter == null) {
      throw new Error("Timeout certificate contains an invalid voter");
    }
    const payload = timeoutVotePayload({
      ...expected,
      voter,
    });
    if (
      String(vote?.domain || "") !== TIMEOUT_VOTE_DOMAIN
      || String(vote?.matchId || "") !== payload.matchId
      || Number(vote?.basisSequence) !== payload.basisSequence
      || Number(vote?.forfeitedPlayer) !== payload.forfeitedPlayer
      || Number(vote?.activePlayer) !== payload.activePlayer
      || String(vote?.clockHash || "") !== payload.clockHash
      || Math.max(0, Math.floor(Number(vote?.remainingMs || 0))) !== payload.remainingMs
    ) {
      throw new Error("Timeout certificate vote does not match the timeout claim");
    }
    const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(voter));
    const valid = await verifyAuditPayload(publicKey, payload, vote.signature || "");
    if (!valid) {
      throw new Error("Timeout certificate vote signature is invalid");
    }
    return voter;
  }

  async function verifyTimeoutCertificate(command, uiState) {
    if (!isActionTimeoutForfeitCommand(command)) return;
    const playerCount = playerCountForClock(uiState);
    if (playerCount < 3) return;
    const forfeitedPlayer = Number(command.player);
    const certificate = timeoutCertificateFromCommand(command);
    if (!certificate || String(certificate.type || "") !== "match_clock_timeout_quorum_v1") {
      throw new Error("Multiplayer timeout forfeit is missing its quorum certificate");
    }
    const expected = {
      matchId: currentAuditMatchId(),
      basisSequence: Number(command.basis_sequence ?? certificate.basisSequence ?? 0),
      forfeitedPlayer,
      activePlayer: forfeitedPlayer,
      clockHash: String(command.match_clock_hash || certificate.clockHash || ""),
      remainingMs: Number(command.remaining_ms ?? certificate.remainingMs ?? 0),
    };
    const requiredVoters = expectedTimeoutVoters(
      matchStartPayloadRef.current?.players || multiplayerRef.current.players || [],
      forfeitedPlayer
    );
    const votes = Array.isArray(certificate.votes) ? certificate.votes : [];
    if (votes.length !== requiredVoters.length) {
      throw new Error("Timeout certificate must include every non-timed-out player");
    }
    const seen = new Set();
    for (const vote of votes) {
      const voter = await verifyTimeoutVote(vote, expected);
      if (!requiredVoters.includes(voter)) {
        throw new Error("Timeout certificate contains an ineligible voter");
      }
      if (seen.has(voter)) {
        throw new Error("Timeout certificate contains a duplicate voter");
      }
      seen.add(voter);
    }
    for (const voter of requiredVoters) {
      if (!seen.has(voter)) {
        throw new Error("Timeout certificate is missing a required voter");
      }
    }
  }

  async function validateTimeoutForfeitCommand(command, uiState, options = {}) {
    if (!isActionTimeoutForfeitCommand(command)) return;
    const forfeitedPlayer = Number(command.player);
    const activePlayer = matchClockActivePlayerFromState(uiState);
    if (activePlayer == null) {
      throw new Error("No active decision can be forfeited by timeout");
    }
    if (activePlayer !== forfeitedPlayer) {
      throw new Error("Timeout forfeit does not match the current decision player");
    }

    const timer = currentMatchClockSnapshot();
    if (timer.activePlayerIndex == null || Number(timer.activePlayerIndex) !== forfeitedPlayer) {
      throw new Error("Timeout forfeit does not match the active match clock");
    }

    const skewMs = Number(options.skewMs ?? 0);
    if (Number(timer.remainingMs ?? 0) > Number(timer.graceMs || 0) + skewMs) {
      throw new Error("Match clock has not expired");
    }
    if (!options.skipCertificate) {
      await verifyTimeoutCertificate(command, uiState);
    }
  }

  async function answerTimeoutVoteRequest(conn, message) {
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      const localVoter = resolveLocalPlayerIndex(multiplayerRef.current);
      const forfeitedPlayer = Number(message?.forfeitedPlayer);
      const basisSequence = Number(message?.basisSequence);
      if (requester == null) {
        throw new Error("Timeout vote requester is not a match player");
      }
      if (Number(requester) === forfeitedPlayer) {
        throw new Error("Timed-out player cannot request their own timeout certificate");
      }
      if (localVoter == null || Number(localVoter) === forfeitedPlayer) {
        throw new Error("Timed-out player cannot vote on their own timeout");
      }
      if (String(message?.matchId || "") !== currentAuditMatchId()) {
        throw new Error("Timeout vote request belongs to a different match");
      }
      if (basisSequence !== Number(multiplayerRef.current.lastAppliedSequence || 0)) {
        throw new Error("Timeout vote request is not based on the local transcript head");
      }
      const liveState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
      const activePlayer = matchClockActivePlayerFromState(liveState);
      updateMatchClockForState(liveState);
      const timer = currentMatchClockSnapshot();
      if (activePlayer == null || Number(activePlayer) !== forfeitedPlayer) {
        throw new Error("Timeout vote does not match the active decision player");
      }
      if (Number(timer.activePlayerIndex) !== forfeitedPlayer) {
        throw new Error("Timeout vote does not match the active match clock");
      }
      if (String(message?.clockHash || "") !== String(timer.clockHash || "")) {
        throw new Error("Timeout vote clock hash does not match the local clock head");
      }
      if (Number(timer.remainingMs ?? 0) > Number(timer.graceMs || 0) + MATCH_CLOCK_CLAIM_SKEW_MS) {
        throw new Error("Match clock has not expired");
      }
      const vote = await signTimeoutVoteForSnapshot({
        basisSequence,
        forfeitedPlayer,
        activePlayer,
        clockHash: String(timer.clockHash || ""),
        remainingMs: Number(timer.remainingMs || 0),
      });
      safeSend(conn, {
        type: "timeout_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        vote,
      });
    } catch (err) {
      safeSend(conn, {
        type: "timeout_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  async function answerDisconnectForfeitVoteRequest(conn, message) {
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      const localVoter = resolveLocalPlayerIndex(multiplayerRef.current);
      const forfeitedPlayer = normalizePlayerIndex(message?.forfeitedPlayer);
      const basisSequence = Number(message?.basisSequence);
      if (requester == null) {
        throw new Error("Disconnect forfeit requester is not a match player");
      }
      if (normalizePlayerIndex(message?.requesterIndex) !== requester) {
        throw new Error("Disconnect forfeit requester index does not match the peer");
      }
      if (forfeitedPlayer == null) {
        throw new Error("Disconnect forfeit request targets an invalid player");
      }
      if (localVoter == null || Number(localVoter) === Number(forfeitedPlayer)) {
        throw new Error("Disconnected player cannot vote on their own disconnect forfeit");
      }
      if (String(message?.matchId || "") !== currentAuditMatchId()) {
        throw new Error("Disconnect forfeit vote request belongs to a different match");
      }
      if (basisSequence !== Number(multiplayerRef.current.lastAppliedSequence || 0)) {
        throw new Error("Disconnect forfeit vote request is not based on the local transcript head");
      }
      const vote = await signDisconnectForfeitVoteForCommand({
        type: "forfeit_player",
        player: forfeitedPlayer,
        reason: DISCONNECT_FORFEIT_REASON,
        basis_sequence: basisSequence,
        disconnected_peer_id: String(message?.forfeitedPeerId || ""),
        disconnect_timeout_ms: Number(message?.disconnectTimeoutMs || DISCONNECT_AUTO_FORFEIT_MS),
      });
      safeSend(conn, {
        type: "disconnect_forfeit_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        vote,
      });
    } catch (err) {
      safeSend(conn, {
        type: "disconnect_forfeit_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  async function collectTimeoutCertificateForCommand(command) {
    if (!isActionTimeoutForfeitCommand(command)) return null;
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    if (players.length < 3) return null;
    if (!isCurrentAuditPlayerCount(players.length)) {
      throw new Error("Current protocol requires 2, 3, or 4 players");
    }
    const forfeitedPlayer = Number(command.player);
    const basisSequence = Number(command.basis_sequence || multiplayerRef.current.lastAppliedSequence || 0);
    const expected = {
      matchId: currentAuditMatchId(),
      basisSequence,
      forfeitedPlayer,
      activePlayer: forfeitedPlayer,
      clockHash: String(command.match_clock_hash || ""),
      remainingMs: Number(command.remaining_ms || 0),
    };
    const requiredVoters = expectedTimeoutVoters(players, forfeitedPlayer);
    const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
    const votes = [];
    for (const voter of requiredVoters) {
      if (Number(voter) === Number(localPlayer)) {
        votes.push(await signTimeoutVoteForSnapshot(expected));
        continue;
      }
      const player = players.find((entry) => Number(entry.index) === Number(voter));
      const routePeerId = routePeerIdForPlayer(player);
      if (!routePeerId) {
        throw new Error(`Missing peer route for timeout voter ${voter + 1}`);
      }
      const conn = await waitForZiffleRoute(routePeerId);
      const requestId = makeZiffleRequestId("timeout-vote");
      const voterLabel = player.name || `Player ${Number(voter) + 1}`;
      const waiter = waitForTimeoutVote(requestId, 15000, {
        peerIndex: voter,
        peerName: voterLabel,
        description:
          `${voterLabel} must sign the timeout vote before this forfeit claim can be submitted.`,
      });
      safeSend(conn, {
        type: "timeout_vote_request",
        protocolVersion: PROTOCOL_VERSION,
        requestId,
        ...expected,
        requesterIndex: localPlayer,
      });
      const vote = await waiter;
      await verifyTimeoutVote(vote, expected);
      votes.push(vote);
    }
    votes.sort((left, right) => Number(left.voter) - Number(right.voter));
    return {
      type: "match_clock_timeout_quorum_v1",
      matchId: expected.matchId,
      basisSequence,
      forfeitedPlayer,
      clockHash: expected.clockHash,
      remainingMs: expected.remainingMs,
      voters: requiredVoters,
      votes,
    };
  }

  function forfeitedPlayersForQuorum(pendingAction = null) {
    const forfeited = new Set();
    for (const entry of actionHistoryRef.current || []) {
      if (isForfeitCommand(entry?.command)) {
        forfeited.add(Number(entry.command.player));
      }
    }
    if (pendingAction && isForfeitCommand(pendingAction.command)) {
      forfeited.add(Number(pendingAction.command.player));
    }
    return forfeited;
  }

  function actionQuorumRoster(action = null) {
    const forfeited = forfeitedPlayersForQuorum(action);
    return reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || [])
      .filter((player) => !forfeited.has(Number(player.index)));
  }

  function disconnectForfeitRoster(forfeitedPlayer) {
    const target = Number(forfeitedPlayer);
    return actionQuorumRoster()
      .filter((player) => Number(player.index) !== target);
  }

  function rememberLocalDisconnectObservation(peerId, details = {}) {
    const normalizedPeerId = String(peerId || "").trim();
    if (!normalizedPeerId) return null;
    const player = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || [])
      .find((entry) => String(entry?.peerId || "").trim() === normalizedPeerId);
    const playerIndex = normalizePlayerIndex(details.playerIndex ?? player?.index);
    const observedAtMs = Math.max(
      0,
      Math.floor(Number(details.disconnectedAtMs ?? Date.now()))
    );
    if (playerIndex == null || !observedAtMs) return null;
    const observation = {
      playerIndex,
      peerId: normalizedPeerId,
      disconnectedAtMs: observedAtMs,
      source: String(details.source || "direct_peer"),
    };
    localDisconnectObservationsRef.current.set(`player:${playerIndex}`, observation);
    localDisconnectObservationsRef.current.set(`peer:${normalizedPeerId}`, observation);
    return observation;
  }

  function clearLocalDisconnectObservation(peerId, playerIndex = null) {
    const normalizedPeerId = String(peerId || "").trim();
    const normalizedPlayerIndex = normalizePlayerIndex(playerIndex);
    if (normalizedPeerId) {
      localDisconnectObservationsRef.current.delete(`peer:${normalizedPeerId}`);
    }
    if (normalizedPlayerIndex != null) {
      localDisconnectObservationsRef.current.delete(`player:${normalizedPlayerIndex}`);
    }
  }

  function localDisconnectObservationForPlayer(forfeitedPlayer, peerId = "") {
    const target = normalizePlayerIndex(forfeitedPlayer);
    const normalizedPeerId = String(peerId || "").trim();
    const byPlayer = target == null
      ? null
      : localDisconnectObservationsRef.current.get(`player:${target}`);
    const byPeer = normalizedPeerId
      ? localDisconnectObservationsRef.current.get(`peer:${normalizedPeerId}`)
      : null;
    const observation = byPlayer || byPeer || null;
    if (!observation) return null;
    if (target != null && Number(observation.playerIndex) !== target) return null;
    if (normalizedPeerId && String(observation.peerId || "") !== normalizedPeerId) return null;
    return observation;
  }

  function playerForDisconnectForfeit(forfeitedPlayer) {
    const target = Number(forfeitedPlayer);
    return reindexPlayers(multiplayerRef.current.players || [])
      .find((player) => Number(player.index) === target)
      || reindexPlayers(matchStartPayloadRef.current?.players || [])
        .find((player) => Number(player.index) === target)
      || null;
  }

  async function signDisconnectForfeitVoteForCommand(command) {
    const { keyPair } = await ensureAuditIdentity();
    const voter = resolveLocalPlayerIndex(multiplayerRef.current);
    const forfeitedPlayer = normalizePlayerIndex(command?.player);
    if (voter == null || forfeitedPlayer == null || Number(voter) === Number(forfeitedPlayer)) {
      throw new Error("Disconnected player cannot sign their own disconnect forfeit");
    }
    const target = playerForDisconnectForfeit(forfeitedPlayer);
    const forfeitedPeerId = String(command?.disconnected_peer_id || target?.peerId || "").trim();
    const disconnectTimeoutMs = Math.max(
      0,
      Math.floor(Number(command?.disconnect_timeout_ms || DISCONNECT_AUTO_FORFEIT_MS))
    );
    if (!forfeitedPeerId) {
      throw new Error("Disconnect forfeit is missing the disconnected peer id");
    }
    if (!target || String(target.peerId || "").trim() !== forfeitedPeerId) {
      throw new Error("Disconnect forfeit peer id does not match the local player record");
    }
    const observation = localDisconnectObservationForPlayer(forfeitedPlayer, forfeitedPeerId);
    if (!observation) {
      throw new Error("Disconnect forfeit requires an independently observed disconnect");
    }
    const disconnectedAtMs = Math.max(
      0,
      Math.floor(Number(observation.disconnectedAtMs || 0))
    );
    if (!disconnectedAtMs) {
      throw new Error("Disconnect forfeit is missing a disconnect observation timestamp");
    }
    const eligibleAtMs = disconnectedAtMs + disconnectTimeoutMs;
    const signedAtMs = Date.now();
    if (signedAtMs + MATCH_CLOCK_CLAIM_SKEW_MS < eligibleAtMs) {
      throw new Error("Disconnect timeout has not elapsed");
    }
    return buildSignedDisconnectForfeitVote({
      keyPair,
      matchId: currentAuditMatchId(),
      basisSequence: Number(command?.basis_sequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
      forfeitedPlayer,
      forfeitedPeerId,
      disconnectTimeoutMs,
      disconnectedAtMs,
      eligibleAtMs,
      signedAtMs,
      voter,
    });
  }

  async function validateDisconnectForfeitCommand(command, options = {}) {
    if (!isDisconnectTimeoutForfeitCommand(command)) return;
    const forfeitedPlayer = normalizePlayerIndex(command.player);
    const actorIndex = normalizePlayerIndex(options.actorIndex);
    if (forfeitedPlayer == null) {
      throw new Error("Disconnect forfeit targets an invalid player");
    }
    if (actorIndex != null && actorIndex === forfeitedPlayer) {
      throw new Error("Disconnected player cannot claim their own disconnect forfeit");
    }
    const expectedBasisSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    if (Number(command.basis_sequence ?? expectedBasisSequence) !== expectedBasisSequence) {
      throw new Error("Disconnect forfeit is not based on the local transcript head");
    }
    const certificate = disconnectCertificateFromCommand(command);
    const target = playerForDisconnectForfeit(forfeitedPlayer);
    const forfeitedPeerId = String(
      command.disconnected_peer_id
      || certificate?.forfeitedPeerId
      || target?.peerId
      || ""
    ).trim();
    const disconnectTimeoutMs = Math.max(
      0,
      Math.floor(Number(command.disconnect_timeout_ms ?? certificate?.disconnectTimeoutMs ?? DISCONNECT_AUTO_FORFEIT_MS))
    );
    const claimedDisconnectedAtMs = Math.max(
      0,
      Math.floor(Number(command.disconnected_at_ms ?? certificate?.disconnectedAtMs ?? 0))
    );
    if (!target) {
      throw new Error("Disconnect forfeit target is missing from the local player record");
    }
    if (!forfeitedPeerId || String(target.peerId || "").trim() !== forfeitedPeerId) {
      throw new Error("Disconnect forfeit peer id does not match the local player record");
    }
    const observation = localDisconnectObservationForPlayer(forfeitedPlayer, forfeitedPeerId);
    const observedDisconnectedAtMs = Math.max(
      0,
      Math.floor(Number(observation?.disconnectedAtMs || 0))
    );
    if (!observedDisconnectedAtMs) {
      throw new Error("Disconnect forfeit is missing a local disconnect observation timestamp");
    }
    if (
      claimedDisconnectedAtMs
      && Math.abs(claimedDisconnectedAtMs - observedDisconnectedAtMs) > DISCONNECT_AUTO_FORFEIT_MS
    ) {
      throw new Error("Disconnect forfeit does not match the local disconnect observation");
    }
    if (Date.now() + MATCH_CLOCK_CLAIM_SKEW_MS < observedDisconnectedAtMs + disconnectTimeoutMs) {
      throw new Error("Disconnect timeout has not elapsed");
    }
    const roster = disconnectForfeitRoster(forfeitedPlayer);
    if (options.skipCertificate) return;
    await verifyDisconnectForfeitCertificate({
      certificate,
      command: {
        ...command,
        disconnected_peer_id: forfeitedPeerId,
        disconnect_timeout_ms: disconnectTimeoutMs,
        matchId: currentAuditMatchId(),
        nowMs: Date.now(),
        maxFutureSkewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
      },
      players: roster,
      threshold: disconnectForfeitVoteThreshold(roster.length),
    });
  }

  async function collectDisconnectForfeitCertificateForCommand(command) {
    if (!isDisconnectTimeoutForfeitCommand(command)) return null;
    const forfeitedPlayer = normalizePlayerIndex(command.player);
    if (forfeitedPlayer == null) {
      throw new Error("Disconnect forfeit targets an invalid player");
    }
    const roster = disconnectForfeitRoster(forfeitedPlayer);
    const threshold = disconnectForfeitVoteThreshold(roster.length);
    if (threshold <= 0) return null;

    const target = playerForDisconnectForfeit(forfeitedPlayer);
    const expected = {
      matchId: currentAuditMatchId(),
      basisSequence: Number(command.basis_sequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
      forfeitedPlayer,
      forfeitedPeerId: String(command.disconnected_peer_id || target?.peerId || ""),
      disconnectTimeoutMs: Math.max(
        0,
        Math.floor(Number(command.disconnect_timeout_ms || DISCONNECT_AUTO_FORFEIT_MS))
      ),
      disconnectedAtMs: null,
      nowMs: Date.now(),
      maxFutureSkewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
    };
    const votes = [];
    const seen = new Set();
    const addVote = async (vote) => {
      const voter = await verifyDisconnectForfeitVote({
        vote,
        expected,
        players: roster,
      });
      if (seen.has(voter)) return;
      seen.add(voter);
      votes.push(cloneMultiplayerPayload(vote));
    };

    const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
    if (roster.some((player) => Number(player.index) === Number(localPlayer))) {
      await addVote(await signDisconnectForfeitVoteForCommand({
        ...command,
        disconnected_peer_id: expected.forfeitedPeerId,
        disconnect_timeout_ms: expected.disconnectTimeoutMs,
      }));
    }

    if (votes.length < threshold) {
      ensureDirectPeerConnections(roster);
      const pendingWaitRequestIds = [];
      let collectingVotes = true;
      const pending = roster
        .filter((player) => Number(player.index) !== Number(localPlayer))
        .map(async (player) => {
          const routePeerId = routePeerIdForPlayer(player);
          if (!routePeerId) {
            throw new Error(`Missing peer route for disconnect voter ${Number(player.index) + 1}`);
          }
          const conn = await waitForZiffleRoute(routePeerId);
          if (!collectingVotes) {
            throw new Error("Disconnect-forfeit vote collection is complete");
          }
          const requestId = makeZiffleRequestId("disconnect-forfeit-vote");
          pendingWaitRequestIds.push(requestId);
          const voterLabel = player.name || `Player ${Number(player.index) + 1}`;
          const waiter = waitForTimeoutVote(requestId, 15000, {
            peerIndex: Number(player.index),
            peerName: voterLabel,
            description:
              `${voterLabel} must sign the disconnect-forfeit vote before this claim can be submitted.`,
          });
          safeSend(conn, {
            type: "disconnect_forfeit_vote_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            ...expected,
            requesterIndex: localPlayer,
          });
          return waiter;
        });
      const unsettled = new Set(pending);
      while (votes.length < threshold && unsettled.size > 0) {
        const settled = await Promise.race([...unsettled].map((promise) =>
          promise.then(
            (vote) => ({ promise, vote }),
            (err) => ({ promise, err })
          )
        ));
        unsettled.delete(settled.promise);
        if (settled.err) {
          if (protocolResponseTimeoutClaimFromError(settled.err)) throw settled.err;
          continue;
        }
        await addVote(settled.vote);
      }
      collectingVotes = false;
      for (const promise of unsettled) {
        promise.catch(() => {});
      }
      for (const requestId of pendingWaitRequestIds) {
        clearPeerWait(requestId);
      }
    }

    if (votes.length < threshold) {
      throw new Error(
        `Disconnect forfeit certificate has ${votes.length} vote(s), expected at least ${threshold}`
      );
    }
    votes.sort((left, right) => Number(left.voter) - Number(right.voter));
    return {
      type: "ironsmith-disconnect-forfeit-v1",
      matchId: expected.matchId,
      basisSequence: expected.basisSequence,
      forfeitedPlayer: expected.forfeitedPlayer,
      forfeitedPeerId: expected.forfeitedPeerId,
      disconnectTimeoutMs: expected.disconnectTimeoutMs,
      threshold,
      voters: votes.map((vote) => Number(vote.voter)),
      votes,
    };
  }

  function protocolResponseTimeoutRoster(forfeitedPlayer) {
    const target = Number(forfeitedPlayer);
    return actionQuorumRoster()
      .filter((player) => Number(player.index) !== target);
  }

  function playerForProtocolResponseTimeout(forfeitedPlayer) {
    const target = Number(forfeitedPlayer);
    return reindexPlayers(multiplayerRef.current.players || [])
      .find((player) => Number(player.index) === target)
      || reindexPlayers(matchStartPayloadRef.current?.players || [])
        .find((player) => Number(player.index) === target)
      || null;
  }

  async function signProtocolResponseTimeoutVoteForCommand(command) {
    const { keyPair } = await ensureAuditIdentity();
    const voter = resolveLocalPlayerIndex(multiplayerRef.current);
    const forfeitedPlayer = normalizePlayerIndex(command?.player);
    if (voter == null || forfeitedPlayer == null || Number(voter) === Number(forfeitedPlayer)) {
      throw new Error("Timed-out protocol responder cannot sign their own response-timeout forfeit");
    }
    const target = playerForProtocolResponseTimeout(forfeitedPlayer);
    const forfeitedPeerId = String(
      command?.timed_out_peer_id
      || command?.forfeited_peer_id
      || target?.peerId
      || ""
    ).trim();
    const responseTimeoutMs = Math.max(
      1,
      Math.floor(Number(command?.response_timeout_ms || PROTOCOL_RESPONSE_TIMEOUT_MS))
    );
    const requestedAtMs = Math.max(0, Math.floor(Number(command?.requested_at_ms || 0)));
    const eligibleAtMs = requestedAtMs + responseTimeoutMs;
    const signedAtMs = Date.now();
    if (!forfeitedPeerId) {
      throw new Error("Protocol response timeout is missing the timed-out peer id");
    }
    if (!target || String(target.peerId || "").trim() !== forfeitedPeerId) {
      throw new Error("Protocol response timeout peer id does not match the local player record");
    }
    if (!String(command?.request_type || "") || !String(command?.request_id || "")) {
      throw new Error("Protocol response timeout is missing request identity");
    }
    if (!String(command?.request_payload_hash || "")) {
      throw new Error("Protocol response timeout is missing request evidence");
    }
    if (signedAtMs + MATCH_CLOCK_CLAIM_SKEW_MS < eligibleAtMs) {
      throw new Error("Protocol response timeout has not elapsed");
    }
    return buildSignedProtocolResponseTimeoutVote({
      keyPair,
      matchId: currentAuditMatchId(),
      basisSequence: Number(command?.basis_sequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
      forfeitedPlayer,
      forfeitedPeerId,
      requestType: String(command.request_type || ""),
      requestId: String(command.request_id || ""),
      requestPayloadHash: String(command.request_payload_hash || ""),
      responseTimeoutMs,
      requestedAtMs,
      eligibleAtMs,
      signedAtMs,
      voter,
    });
  }

  async function validateProtocolResponseTimeoutCommand(command, options = {}) {
    if (!isProtocolResponseTimeoutForfeitCommand(command)) return;
    const forfeitedPlayer = normalizePlayerIndex(command.player);
    const actorIndex = normalizePlayerIndex(options.actorIndex);
    if (forfeitedPlayer == null) {
      throw new Error("Protocol response timeout targets an invalid player");
    }
    if (actorIndex != null && actorIndex === forfeitedPlayer) {
      throw new Error("Timed-out protocol responder cannot claim their own response-timeout forfeit");
    }
    const expectedBasisSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    if (Number(command.basis_sequence ?? expectedBasisSequence) !== expectedBasisSequence) {
      throw new Error("Protocol response timeout is not based on the local transcript head");
    }
    const certificate = command.protocol_timeout_certificate || command.protocolTimeoutCertificate;
    const target = playerForProtocolResponseTimeout(forfeitedPlayer);
    const forfeitedPeerId = String(
      command.timed_out_peer_id
      || command.forfeited_peer_id
      || certificate?.forfeitedPeerId
      || target?.peerId
      || ""
    ).trim();
    const responseTimeoutMs = Math.max(
      1,
      Math.floor(Number(command.response_timeout_ms ?? certificate?.responseTimeoutMs ?? PROTOCOL_RESPONSE_TIMEOUT_MS))
    );
    const requestedAtMs = Math.max(
      0,
      Math.floor(Number(command.requested_at_ms ?? certificate?.requestedAtMs ?? 0))
    );
    const eligibleAtMs = requestedAtMs + responseTimeoutMs;
    if (!target) {
      throw new Error("Protocol response timeout target is missing from the local player record");
    }
    if (!forfeitedPeerId || String(target.peerId || "").trim() !== forfeitedPeerId) {
      throw new Error("Protocol response timeout peer id does not match the local player record");
    }
    if (!String(command.request_type || certificate?.requestType || "")) {
      throw new Error("Protocol response timeout is missing request type");
    }
    if (!String(command.request_id || certificate?.requestId || "")) {
      throw new Error("Protocol response timeout is missing request id");
    }
    if (!String(command.request_payload_hash || certificate?.requestPayloadHash || "")) {
      throw new Error("Protocol response timeout is missing request evidence");
    }
    if (Date.now() + MATCH_CLOCK_CLAIM_SKEW_MS < eligibleAtMs) {
      throw new Error("Protocol response timeout has not elapsed");
    }
    const roster = protocolResponseTimeoutRoster(forfeitedPlayer);
    if (options.skipCertificate) return;
    await verifyProtocolResponseTimeoutCertificate({
      certificate,
      command: {
        ...command,
        timed_out_peer_id: forfeitedPeerId,
        response_timeout_ms: responseTimeoutMs,
        matchId: currentAuditMatchId(),
        nowMs: Date.now(),
        maxFutureSkewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
      },
      players: roster,
      threshold: protocolResponseTimeoutVoteThreshold(roster.length),
    });
  }

  async function validateTrustedSequencedAction({
    command,
    actorIndex,
    seq,
    clock,
    uiState,
    enforceMatchClockObservationBounds = false,
  }) {
    const normalizedActor = normalizePlayerIndex(actorIndex);
    if (normalizedActor == null) {
      throw new Error("Trusted action actor is not a match player");
    }
    const liveState = uiState || stateRef.current;
    if (isUnauthorizedAddCardCommand(command)) {
      const actorName = playerNameForIndex(multiplayerRef.current.players, normalizedActor);
      throw new Error(`Unauthorized add-card action from ${actorName}`);
    }
    const expectedActor = liveState?.decision?.player;
    const isTimeoutForfeit = isActionTimeoutForfeitCommand(command);
    const isDisconnectForfeit = isDisconnectTimeoutForfeitCommand(command);
    const isProtocolTimeoutForfeit = isProtocolResponseTimeoutForfeitCommand(command);
    const isSelfForfeit = isSelfForfeitCommand(command, normalizedActor);
    if (isSelfForfeit && !isSorcerySpeedForfeitState(liveState, normalizedActor)) {
      throw new Error("Surrender is only available at sorcery speed");
    }
    if (
      isForfeitCommand(command)
      && !isTimeoutForfeit
      && !isDisconnectForfeit
      && !isProtocolTimeoutForfeit
    ) {
      if (Number(command.player) !== Number(normalizedActor)) {
        throw new Error("A player can only forfeit themselves");
      }
    }
    if (isTimeoutForfeit) {
      await validateTimeoutForfeitCommand(command, liveState, {
        skewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
        skipCertificate: true,
      });
    } else if (isDisconnectForfeit) {
      await validateDisconnectForfeitCommand(command, {
        actorIndex: normalizedActor,
        skipCertificate: true,
      });
    } else if (isProtocolTimeoutForfeit) {
      await validateProtocolResponseTimeoutCommand(command, {
        actorIndex: normalizedActor,
        skipCertificate: true,
      });
    } else {
      if (!isDecisionCommandCompatible(liveState?.decision, command)) {
        throw new Error("Trusted action is no longer available");
      }
      if (
        expectedActor !== null
        && expectedActor !== undefined
        && Number(expectedActor) !== Number(normalizedActor)
      ) {
        throw new Error("Sequenced action actor is not the current decision player");
      }
    }

    const skipMatchClockObservationBounds =
      Number(seq) === Number(matchClockObservationExemptSequenceRef.current || 0);
    await verifyMatchClockAuditForAction({
      clock,
      command,
      seq,
      actorIndex: normalizedActor,
      uiState: liveState,
      skewMs: Math.max(
        MATCH_CLOCK_CLAIM_SKEW_MS,
        MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS
      ),
      enforceObservationBounds:
        enforceMatchClockObservationBounds
        && !skipMatchClockObservationBounds,
      enforceUnderreportBounds:
        enforceMatchClockObservationBounds
        && !skipMatchClockObservationBounds,
      skipTimeoutCertificate: true,
    });
  }

  async function collectProtocolResponseTimeoutCertificateForCommand(command) {
    if (!isProtocolResponseTimeoutForfeitCommand(command)) return null;
    const forfeitedPlayer = normalizePlayerIndex(command.player);
    if (forfeitedPlayer == null) {
      throw new Error("Protocol response timeout targets an invalid player");
    }
    const roster = protocolResponseTimeoutRoster(forfeitedPlayer);
    const threshold = protocolResponseTimeoutVoteThreshold(roster.length);
    if (threshold <= 0) return null;

    const target = playerForProtocolResponseTimeout(forfeitedPlayer);
    const expected = {
      matchId: currentAuditMatchId(),
      basisSequence: Number(command.basis_sequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
      forfeitedPlayer,
      forfeitedPeerId: String(command.timed_out_peer_id || command.forfeited_peer_id || target?.peerId || ""),
      requestType: String(command.request_type || ""),
      requestId: String(command.request_id || ""),
      requestPayloadHash: String(command.request_payload_hash || ""),
      responseTimeoutMs: Math.max(
        1,
        Math.floor(Number(command.response_timeout_ms || PROTOCOL_RESPONSE_TIMEOUT_MS))
      ),
      requestedAtMs: Math.max(0, Math.floor(Number(command.requested_at_ms || 0))),
      nowMs: Date.now(),
      maxFutureSkewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
    };
    const votes = [];
    const seen = new Set();
    const addVote = async (vote) => {
      const voter = await verifyProtocolResponseTimeoutVote({
        vote,
        expected: {
          ...expected,
          nowMs: Date.now(),
        },
        players: roster,
      });
      if (seen.has(voter)) return;
      seen.add(voter);
      votes.push(cloneMultiplayerPayload(vote));
    };

    const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
    if (roster.some((player) => Number(player.index) === Number(localPlayer))) {
      await addVote(await signProtocolResponseTimeoutVoteForCommand({
        ...command,
        timed_out_peer_id: expected.forfeitedPeerId,
        response_timeout_ms: expected.responseTimeoutMs,
      }));
    }

    if (votes.length < threshold) {
      ensureDirectPeerConnections(roster);
      const pendingWaitRequestIds = [];
      let collectingVotes = true;
      const pending = roster
        .filter((player) => Number(player.index) !== Number(localPlayer))
        .map(async (player) => {
          const routePeerId = routePeerIdForPlayer(player);
          if (!routePeerId) {
            throw new Error(`Missing peer route for protocol-timeout voter ${Number(player.index) + 1}`);
          }
          const conn = await waitForZiffleRoute(routePeerId);
          if (!collectingVotes) {
            throw new Error("Protocol-timeout vote collection is complete");
          }
          const requestId = makeZiffleRequestId("protocol-timeout-vote");
          pendingWaitRequestIds.push(requestId);
          const voterLabel = player.name || `Player ${Number(player.index) + 1}`;
          const waiter = waitForTimeoutVote(requestId, PROTOCOL_RESPONSE_TIMEOUT_VOTE_WAIT_MS, {
            peerIndex: Number(player.index),
            peerName: voterLabel,
            description:
              `${voterLabel} must sign the protocol-timeout vote before this claim can be submitted.`,
          });
          const { requestId: timedOutRequestId, ...voteRequestExpected } = expected;
          safeSend(conn, {
            type: "protocol_timeout_vote_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            ...voteRequestExpected,
            timedOutRequestId,
            requesterIndex: localPlayer,
          });
          return waiter;
        });
      const unsettled = new Set(pending);
      while (votes.length < threshold && unsettled.size > 0) {
        const settled = await Promise.race([...unsettled].map((promise) =>
          promise.then(
            (vote) => ({ promise, vote }),
            (err) => ({ promise, err })
          )
        ));
        unsettled.delete(settled.promise);
        if (settled.err) {
          if (protocolResponseTimeoutClaimFromError(settled.err)) throw settled.err;
          continue;
        }
        await addVote(settled.vote);
      }
      collectingVotes = false;
      for (const promise of unsettled) {
        promise.catch(() => {});
      }
      for (const requestId of pendingWaitRequestIds) {
        clearPeerWait(requestId);
      }
    }

    if (votes.length < threshold) {
      throw new Error(
        `Protocol response timeout certificate has ${votes.length} vote(s), expected at least ${threshold}`
      );
    }
    votes.sort((left, right) => Number(left.voter) - Number(right.voter));
    return {
      type: "ironsmith-protocol-response-timeout-v1",
      matchId: expected.matchId,
      basisSequence: expected.basisSequence,
      forfeitedPlayer: expected.forfeitedPlayer,
      forfeitedPeerId: expected.forfeitedPeerId,
      requestType: expected.requestType,
      requestId: expected.requestId,
      requestPayloadHash: expected.requestPayloadHash,
      responseTimeoutMs: expected.responseTimeoutMs,
      requestedAtMs: expected.requestedAtMs,
      eligibleAtMs: expected.requestedAtMs + expected.responseTimeoutMs,
      threshold,
      voters: votes.map((vote) => Number(vote.voter)),
      votes,
    };
  }

  async function answerProtocolResponseTimeoutVoteRequest(conn, message) {
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      const localVoter = resolveLocalPlayerIndex(multiplayerRef.current);
      const forfeitedPlayer = normalizePlayerIndex(message?.forfeitedPlayer);
      const basisSequence = Number(message?.basisSequence);
      if (requester == null) {
        throw new Error("Protocol-timeout vote requester is not a match player");
      }
      if (normalizePlayerIndex(message?.requesterIndex) !== requester) {
        throw new Error("Protocol-timeout requester index does not match the peer");
      }
      if (forfeitedPlayer == null) {
        throw new Error("Protocol-timeout vote request targets an invalid player");
      }
      if (localVoter == null || Number(localVoter) === Number(forfeitedPlayer)) {
        throw new Error("Timed-out protocol responder cannot vote on their own forfeit");
      }
      if (String(message?.matchId || "") !== currentAuditMatchId()) {
        throw new Error("Protocol-timeout vote request belongs to a different match");
      }
      if (basisSequence !== Number(multiplayerRef.current.lastAppliedSequence || 0)) {
        throw new Error("Protocol-timeout vote request is not based on the local transcript head");
      }
      const vote = await signProtocolResponseTimeoutVoteForCommand({
        type: "forfeit_player",
        player: forfeitedPlayer,
        reason: PROTOCOL_RESPONSE_TIMEOUT_REASON,
        basis_sequence: basisSequence,
        timed_out_peer_id: String(message?.forfeitedPeerId || ""),
        request_type: String(message?.requestType || ""),
        request_id: String(message?.timedOutRequestId || message?.originalRequestId || ""),
        request_payload_hash: String(message?.requestPayloadHash || ""),
        response_timeout_ms: Number(message?.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS),
        requested_at_ms: Number(message?.requestedAtMs || 0),
      });
      safeSend(conn, {
        type: "protocol_timeout_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        vote,
      });
    } catch (err) {
      safeSend(conn, {
        type: "protocol_timeout_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  function actionQuorumThresholdForMessage(message, players = actionQuorumRoster(message)) {
    const forfeited = forfeitedPlayersForQuorum();
    const activePlayerCount = reindexPlayers(
      matchStartPayloadRef.current?.players || multiplayerRef.current.players || []
    )
      .filter((player) => !forfeited.has(Number(player.index)))
      .length;
    if (activePlayerCount < 3) return 0;
    if (isForfeitCommand(message?.command)) {
      const target = Number(message.command.player);
      if (Number(message?.actorIndex) === target) return 0;
      if (isDisconnectTimeoutForfeitCommand(message.command)) return 0;
      if (isProtocolResponseTimeoutForfeitCommand(message.command)) return 0;
      return players.length;
    }
    return actionQuorumThreshold(players.length);
  }

  function actionQuorumVoteCacheKey(vote) {
    return [
      String(vote?.matchId || ""),
      Number(vote?.seq || 0),
      Number(vote?.voter),
    ].join(":");
  }

  function actionQuorumVoteConflict(left, right) {
    if (!left || !right) return false;
    return (
      String(left.prevStateHash || "") !== String(right.prevStateHash || "")
      || String(left.nextStateHash || "") !== String(right.nextStateHash || "")
      || String(left.publicCheckpointHash || "") !== String(right.publicCheckpointHash || "")
      || String(left.actionSignature || "") !== String(right.actionSignature || "")
      || Number(left.actor) !== Number(right.actor)
    );
  }

  function rememberSignedActionQuorumVote(vote) {
    const key = actionQuorumVoteCacheKey(vote);
    const existing =
      signedActionQuorumVotesRef.current.get(key)
      || readStoredActionQuorumVote(vote?.matchId, vote?.seq, vote?.voter);
    if (existing && actionQuorumVoteConflict(existing, vote)) {
      throw new Error("Refusing to sign conflicting action quorum votes for the same sequence");
    }
    signedActionQuorumVotesRef.current.set(key, cloneMultiplayerPayload(vote));
    writeStoredActionQuorumVote(vote);
    return vote;
  }

  async function signActionQuorumVoteForMessage(message) {
    const localVoter = resolveLocalPlayerIndex(multiplayerRef.current);
    if (localVoter == null) {
      throw new Error("Local player cannot sign an action quorum vote");
    }
    const stored =
      signedActionQuorumVotesRef.current.get([
        String(message?.audit?.matchId || currentAuditMatchId()),
        Number(message?.audit?.seq || message?.seq || 0),
        Number(localVoter),
      ].join(":"))
      || readStoredActionQuorumVote(
        message?.audit?.matchId || currentAuditMatchId(),
        message?.audit?.seq || message?.seq,
        localVoter
      );
    const expectedVote = {
      matchId: String(message?.audit?.matchId || ""),
      seq: Number(message?.audit?.seq || message?.seq || 0),
      actor: Number(message?.audit?.actor ?? message?.actorIndex ?? 0),
      voter: Number(localVoter),
      prevStateHash: String(message?.audit?.prevStateHash || ""),
      nextStateHash: String(message?.audit?.nextStateHash || ""),
      publicCheckpointHash: String(message?.audit?.publicCheckpointHash || ""),
      actionSignature: String(message?.audit?.signature || ""),
    };
    if (stored) {
      if (actionQuorumVoteConflict(stored, expectedVote)) {
        throw new Error("Refusing to sign conflicting action quorum votes for the same sequence");
      }
      return stored;
    }
    const { keyPair } = await ensureAuditIdentity();
    return rememberSignedActionQuorumVote(await buildSignedActionQuorumVote({
      keyPair,
      action: message,
      voter: localVoter,
    }));
  }

  async function verifyActionQuorumForMessage(message) {
    const players = actionQuorumRoster(message);
    const threshold = actionQuorumThresholdForMessage(message, players);
    if (threshold <= 0) return;
    await verifyActionQuorumCertificate({
      certificate: message?.audit?.quorumCertificate || message?.quorumCertificate,
      action: message,
      players,
      threshold,
    });
  }

  async function verifyActionQuorumVoteForMessage(vote, message) {
    return verifyActionQuorumVote({
      vote,
      action: message,
      players: actionQuorumRoster(message),
    });
  }

  async function collectActionQuorumCertificate(message) {
    const players = actionQuorumRoster(message);
    const threshold = actionQuorumThresholdForMessage(message, players);
    if (threshold <= 0) return null;

    const votes = [];
    const seen = new Set();
    const addVote = async (vote) => {
      const voter = await verifyActionQuorumVoteForMessage(vote, message);
      if (seen.has(voter)) return;
      seen.add(voter);
      votes.push(cloneMultiplayerPayload(vote));
    };

    await addVote(await signActionQuorumVoteForMessage(message));
    if (votes.length < threshold) {
      ensureDirectPeerConnections(players);
      const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
      const pendingWaitRequestIds = [];
      let collectingVotes = true;
      const pending = players
        .filter((player) => Number(player.index) !== Number(localPlayer))
        .map(async (player) => {
          const routePeerId = routePeerIdForPlayer(player);
          if (!routePeerId) {
            throw new Error(`Missing peer route for quorum voter ${Number(player.index) + 1}`);
          }
          const conn = await waitForZiffleRoute(routePeerId);
          if (!collectingVotes) {
            throw new Error("Action quorum vote collection is complete");
          }
          const requestId = makeZiffleRequestId("action-quorum");
          const requestedAtMs = Date.now();
          pendingWaitRequestIds.push(requestId);
          const voterLabel = player.name || `Player ${Number(player.index) + 1}`;
          const waiter = waitForActionQuorumVote(requestId, PROTOCOL_RESPONSE_TIMEOUT_MS, {
            peerIndex: Number(player.index),
            peerName: voterLabel,
            description:
              `${voterLabel} is verifying and signing the action payload.`,
          });
          const requestPayload = {
            type: "action_quorum_vote_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            requesterIndex: localPlayer,
            action: cloneMultiplayerPayload(message),
          };
          const quorumPerf = {
            request_id: requestId,
            voter: Number(player.index),
            requester: localPlayer,
            action: summarizeSequencedActionForPerf(message),
            request_bytes: payloadSizeBytes(requestPayload),
          };
          recordPeerSyncPerf("action_quorum:send_vote_request", quorumPerf);
          safeSend(conn, requestPayload);
          return timePeerSyncPhase(
            "action_quorum:wait_vote_response",
            quorumPerf,
            () => waitForProtocolResponse(waiter, {
              basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
              targetPlayerIndex: Number(player.index),
              targetPeerId: player.peerId,
              requesterIndex: localPlayer,
              requestType: requestPayload.type,
              requestId,
              requestPayload,
              responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
              requestedAtMs,
            })
          );
        });
      const unsettled = new Set(pending);
      while (votes.length < threshold && unsettled.size > 0) {
        const settled = await Promise.race([...unsettled].map((promise) =>
          promise.then(
            (vote) => ({ promise, vote }),
            (err) => ({ promise, err })
          )
        ));
        unsettled.delete(settled.promise);
        if (settled.err) {
          if (protocolResponseTimeoutClaimFromError(settled.err)) throw settled.err;
          continue;
        }
        await addVote(settled.vote);
      }
      collectingVotes = false;
      for (const promise of unsettled) {
        promise.catch(() => {});
      }
      for (const requestId of pendingWaitRequestIds) {
        clearPeerWait(requestId);
      }
    }

    if (votes.length < threshold) {
      throw new Error(
        `Action quorum certificate has ${votes.length} vote(s), expected at least ${threshold}`
      );
    }
    votes.sort((left, right) => Number(left.voter) - Number(right.voter));
    const audit = message.audit || {};
    return {
      type: "ironsmith-action-quorum-v1",
      matchId: String(audit.matchId || ""),
      seq: Number(audit.seq || message.seq || 0),
      actor: Number(audit.actor ?? message.actorIndex ?? 0),
      prevStateHash: String(audit.prevStateHash || ""),
      nextStateHash: String(audit.nextStateHash || ""),
      publicCheckpointHash: String(audit.publicCheckpointHash || ""),
      actionSignature: String(audit.signature || ""),
      threshold,
      voters: votes.map((vote) => Number(vote.voter)),
      votes,
    };
  }

  async function answerActionQuorumVoteRequest(conn, message) {
    const quorumPerf = {
      request_id: String(message?.requestId || ""),
      requester: message?.requesterIndex == null ? null : Number(message.requesterIndex),
      action: summarizeSequencedActionForPerf(message?.action),
      request_bytes: payloadSizeBytes(message),
    };
    recordPeerSyncPerf("action_quorum:received_vote_request", quorumPerf);
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      if (requester == null) {
        throw new Error("Action quorum requester is not a match player");
      }
      if (normalizePlayerIndex(message?.requesterIndex) !== requester) {
        throw new Error("Action quorum requester index does not match the peer");
      }
      const action = message?.action;
      if (!action || action.type !== "apply_action") {
        throw new Error("Action quorum request is missing an action");
      }
      await timePeerSyncPhase(
        "action_quorum:dry_run_apply_action",
        quorumPerf,
        () => applySequencedActionMessage(action, {
          relay: false,
          dryRun: true,
          skipQuorumCertificate: true,
          throwOnOrderMismatch: true,
        })
      );
      const requestPayload = cloneMultiplayerPayload(message);
      await refreshPendingActionIntentEvidenceForAction(action, {
        requestType: "action_quorum_vote_request",
        requestId: String(message.requestId || ""),
        requestPayload,
        requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
        responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
        requestedAtMs: Date.now(),
      });
      const vote = await timePeerSyncPhase(
        "action_quorum:sign_vote",
        quorumPerf,
        () => signActionQuorumVoteForMessage(action)
      );
      recordPeerSyncPerf("action_quorum:send_vote_response", quorumPerf);
      safeSend(conn, {
        type: "action_quorum_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        vote,
      });
    } catch (err) {
      const failureReason = toErrorMessage(err);
      recordPeerSyncPerf("action_quorum:error_response", {
        ...quorumPerf,
        error: failureReason,
      });
      if (message?.action && isRejectedActionCheatReason(failureReason)) {
        const actorName = playerNameForIndex(
          multiplayerRef.current.players,
          message.action.actorIndex
        );
        const status = `Cheat detected from ${actorName}: ${failureReason}`;
        emitSyncFailureNotice("Cheat detected", status);
        setStatus(status, true);
      }
      safeSend(conn, {
        type: "action_quorum_vote_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: failureReason,
      });
    }
  }

  function actionHistoryEntryForSequence(seq) {
    return (actionHistoryRef.current || []).find(
      (entry) => Number(entry?.seq) === Number(seq)
    ) || null;
  }

  function rememberActionCryptoRequirements(seq, requirements = []) {
    const sequence = Number(seq);
    if (!Number.isSafeInteger(sequence) || sequence <= 0) return;
    const merged = new Map();
    for (const requirement of [
      ...(actionCryptoRequirementsRef.current.get(sequence) || []),
      ...(requirements || []),
    ]) {
      if (!requirement || typeof requirement !== "object") continue;
      const key = String(requirement.id || canonicalMultiplayerPayload(requirement));
      if (!merged.has(key)) {
        merged.set(key, cloneMultiplayerPayload(requirement));
      }
    }
    actionCryptoRequirementsRef.current.set(sequence, [...merged.values()]);
    const minRetainedSequence = Math.max(1, sequence - 64);
    for (const existingSeq of actionCryptoRequirementsRef.current.keys()) {
      if (Number(existingSeq) < minRetainedSequence) {
        actionCryptoRequirementsRef.current.delete(existingSeq);
      }
    }
  }

  function cryptoRequirementReplayKey(requirement) {
    if (!requirement || typeof requirement !== "object") return "";
    return canonicalMultiplayerPayload({
      id: requirement.id ?? null,
      type: requirement.type ?? requirement.requirement_type ?? null,
      owner: requirement.owner ?? null,
      viewer: requirement.viewer ?? null,
      zone: requirement.zone ?? null,
      slot: requirement.slot ?? null,
      objectId: requirement.objectId ?? requirement.object_id ?? null,
      commitment: requirement.commitment ?? null,
      count: requirement.count ?? null,
      from: requirement.from ?? null,
      to: requirement.to ?? null,
      beforeOrder: normalizeShuffleOrder(requirement.beforeOrder ?? requirement.before_order),
      afterOrder: normalizeShuffleOrder(requirement.afterOrder ?? requirement.after_order),
      randomCountBefore: requirement.randomCountBefore ?? requirement.random_count_before ?? null,
      randomCountAfter: requirement.randomCountAfter ?? requirement.random_count_after ?? null,
      visibility: requirement.visibility ?? null,
      reason: requirement.reason ?? null,
    });
  }

  function freshCryptoRequirementsForSequence(seq, requirements = []) {
    const sequence = Number(seq);
    if (!Number.isSafeInteger(sequence) || sequence <= 0) {
      return Array.isArray(requirements) ? requirements : [];
    }
    const previous = new Set();
    for (const [existingSeq, existingRequirements] of actionCryptoRequirementsRef.current.entries()) {
      if (Number(existingSeq) >= sequence) continue;
      for (const requirement of existingRequirements || []) {
        const key = cryptoRequirementReplayKey(requirement);
        if (key) previous.add(key);
      }
    }
    return (Array.isArray(requirements) ? requirements : []).filter((requirement) => {
      const key = cryptoRequirementReplayKey(requirement);
      return !key || !previous.has(key);
    });
  }

  function shuffleProofReplayKey(proof) {
    if (!proof || typeof proof !== "object") return "";
    return canonicalMultiplayerPayload({
      requirementId: proof.requirementId ?? proof.requirement_id ?? null,
      owner: proof.owner ?? null,
      zone: proof.zone ?? "library",
      deckHash: proof.deckHash ?? proof.deck_hash ?? null,
      context: proof.context ?? null,
      keyContext: proof.keyContext ?? proof.key_context ?? null,
      beforeOrder: normalizeShuffleOrder(proof.beforeOrder ?? proof.before_order),
      afterOrder: normalizeShuffleOrder(proof.afterOrder ?? proof.after_order),
    });
  }

  function shuffleProofAlreadyAppliedBefore(seq, proof) {
    const sequence = Number(seq);
    const key = shuffleProofReplayKey(proof);
    if (!key || !Number.isSafeInteger(sequence) || sequence <= 0) return false;
    return (actionHistoryRef.current || []).some((entry) =>
      Number(entry?.seq) < sequence
      && (entry?.audit?.shuffleProofs || []).some((existing) =>
        shuffleProofReplayKey(existing) === key
      )
    );
  }

  function shuffleProofRequirementAlreadyRecordedBefore(seq, proof) {
    const sequence = Number(seq);
    if (!Number.isSafeInteger(sequence) || sequence <= 0) return false;
    const proofAfterOrder = normalizeShuffleOrder(proof?.afterOrder ?? proof?.after_order);
    return [...actionCryptoRequirementsRef.current.entries()].some(([existingSeq, requirements]) =>
      Number(existingSeq) < sequence
      && (requirements || []).some((requirement) =>
        String(requirement?.type || requirement?.requirement_type || "") === "verifiable_shuffle"
        && shuffleProofMatchesRequirement(proof, requirement)
        && sameShuffleOrder(
          proofAfterOrder,
          requirement.afterOrder ?? requirement.after_order
        )
      )
    );
  }

  function actionCryptoRequirementsForSequence(seq) {
    const requirements = actionCryptoRequirementsRef.current.get(Number(seq));
    return Array.isArray(requirements) ? requirements : [];
  }

  function stateHashBeforeSequence(seq) {
    const previousSeq = Number(seq) - 1;
    if (previousSeq <= 0) return INITIAL_AUDIT_STATE_HASH;
    const previous = actionHistoryEntryForSequence(previousSeq);
    return String(previous?.audit?.nextStateHash || "");
  }

  function sequencedActionsEquivalent(left, right) {
    if (!left || !right) return false;
    return (
      Number(left.seq || 0) === Number(right.seq || 0)
      && Number(left.actorIndex) === Number(right.actorIndex)
      && canonicalMultiplayerPayload(left.command) === canonicalMultiplayerPayload(right.command)
      && String(left.audit?.prevStateHash || "") === String(right.audit?.prevStateHash || "")
      && String(left.audit?.nextStateHash || "") === String(right.audit?.nextStateHash || "")
    );
  }

  function markMatchDisputed(reason, evidence = {}) {
    const body = String(reason || "Match transcript fork detected");
    const dispute = evidence?.dispute || null;
    ignoreAndClearAllPendingActionIntents("match_disputed");
    if (dispute && liveAuditTranscriptRef.current) {
      const existingDisputes = Array.isArray(liveAuditTranscriptRef.current.disputes)
        ? liveAuditTranscriptRef.current.disputes
        : [];
      liveAuditTranscriptRef.current = {
        ...liveAuditTranscriptRef.current,
        disputes: [
          ...existingDisputes,
          cloneMultiplayerPayload(dispute),
        ],
      };
    }
    emitSyncFailureNotice("Match disputed", body);
    timeoutClaimInFlightRef.current = "";
    updateMultiplayer((prev) => ({
      ...prev,
      mode: "disputed",
      matchStarted: false,
      submittingAction: false,
      matchDisputed: {
        reason: body,
        evidence: cloneMultiplayerPayload(evidence),
        accusedPlayers: Array.isArray(dispute?.accusedPlayers)
          ? dispute.accusedPlayers.map(Number)
          : [],
        at: Date.now(),
      },
    }));
    setStatus(body, true);
  }

  async function handleHistoricalSequencedAction(message) {
    const seq = Number(message?.seq || 0);
    const existing = actionHistoryEntryForSequence(seq);
    if (!existing) return true;
    if (sequencedActionsEquivalent(existing, message)) return true;

    const expectedPrevStateHash = stateHashBeforeSequence(seq);
    if (!expectedPrevStateHash) return true;
    try {
      await verifySequencedActionAudit({
        audit: message.audit,
        seq,
        actorIndex: message.actorIndex,
        command: message.command,
        expectedPrevStateHash,
      });
    } catch {
      return true;
    }

    const reason = `Match disputed: two different valid actions were signed for sequence ${seq}.`;
    const dispute = buildActionForkDisputeEvidence({
      sequence: seq,
      reason,
      existingAction: existing,
      conflictingAction: message,
    });
    markMatchDisputed(
      reason,
      {
        sequence: seq,
        accusedPlayers: dispute.accusedPlayers,
        dispute,
      }
    );
    return true;
  }

  async function appendAppliedSequencedAction(message) {
    const nextSequence = Number(message.seq || 0);
    actionHistoryRef.current = [
      ...actionHistoryRef.current,
      {
        seq: nextSequence,
        actorIndex: Number(message.actorIndex),
        command: cloneMultiplayerPayload(message.command),
        label: String(message.label || ""),
        securityMode: normalizeMultiplayerSecurityMode(
          message.securityMode,
          message.audit ? MULTIPLAYER_SECURITY_VERIFIED : MULTIPLAYER_SECURITY_TRUSTED
        ),
        clock: cloneMultiplayerPayload(message.clock),
        audit: cloneMultiplayerPayload(message.audit),
      },
    ];
    if (liveAuditTranscriptRef.current) {
      liveAuditTranscriptRef.current = {
        ...liveAuditTranscriptRef.current,
        actions: actionHistoryRef.current.map((entry) =>
          cloneMultiplayerPayload(entry)
        ),
      };
    }
    clearPendingActionIntent({
      matchId: message.audit?.matchId || currentAuditMatchId(),
      seq: nextSequence,
      actorIndex: message.audit?.actor ?? message.actorIndex,
    });
    if (message.audit?.nextStateHash) {
      auditStateHashRef.current = message.audit.nextStateHash;
    }
    updateMultiplayer((prev) => ({
      ...prev,
      lastAppliedSequence: nextSequence,
      submittingAction: false,
    }));
  }

  async function publishCurrentRuntimeState(stateHint = null) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.uiState !== "function") return null;
    const localIndex = resolveLocalPlayerIndex(multiplayerRef.current);
    if (
      localIndex != null
      && typeof currentGame.setPerspective === "function"
    ) {
      try {
        await currentGame.setPerspective(localIndex);
      } catch {
        // Keep the state publish best-effort; the engine should already have
        // the correct perspective during normal multiplayer action application.
      }
    }
    const nextState = await preserveViewedCardsFromHint(
      await currentGame.uiState(),
      stateHint,
      currentGame,
    );
    stateRef.current = nextState;
    setState(nextState);
    return nextState;
  }

  async function createSequencedActionValidationSnapshot() {
    const currentGame = gameRef.current;
    if (
      !currentGame
      || typeof currentGame.exportSyncCheckpoint !== "function"
      || typeof currentGame.importSyncCheckpoint !== "function"
    ) {
      throw new Error("Game engine cannot sandbox action quorum validation");
    }
    return {
      checkpoint: await currentGame.exportSyncCheckpoint(),
      state: cloneMultiplayerPayload(stateRef.current),
      actionHistory: actionHistoryRef.current.map((entry) => cloneMultiplayerPayload(entry)),
      liveAuditTranscript: liveAuditTranscriptRef.current
        ? cloneMultiplayerPayload(liveAuditTranscriptRef.current)
        : null,
      matchStartPayload: matchStartPayloadRef.current
        ? cloneMultiplayerPayload(matchStartPayloadRef.current)
        : null,
      auditStateHash: auditStateHashRef.current,
      initialPublicCheckpointHash: initialPublicCheckpointHashRef.current,
      lastAppliedSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
      matchClock: cloneMultiplayerPayload(matchClockRef.current),
      matchClockConfig: cloneMultiplayerPayload(matchClockConfigRef.current),
      actionCryptoRequirements: new Map(
        [...actionCryptoRequirementsRef.current.entries()].map(([seq, requirements]) => [
          seq,
          cloneMultiplayerPayload(requirements),
        ])
      ),
	      relayedActionIds: [...relayedActionIdsRef.current],
	      ziffleHandRevealKey: ziffleHandRevealKeyRef.current,
	      ziffleHandRevealQuickKey: ziffleHandRevealQuickKeyRef.current,
	    };
  }

  async function restoreSequencedActionValidationSnapshot(snapshot) {
    if (!snapshot) return;
    const currentGame = gameRef.current;
    const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
    if (
      currentGame
      && snapshot.checkpoint
      && typeof currentGame.importSyncCheckpoint === "function"
    ) {
      await currentGame.importSyncCheckpoint(
        snapshot.checkpoint,
        localPlayer ?? multiplayerRef.current.localPlayerIndex ?? 0
      );
    }
    actionHistoryRef.current = snapshot.actionHistory.map((entry) => cloneMultiplayerPayload(entry));
    liveAuditTranscriptRef.current = snapshot.liveAuditTranscript
      ? cloneMultiplayerPayload(snapshot.liveAuditTranscript)
      : null;
    matchStartPayloadRef.current = snapshot.matchStartPayload
      ? cloneMultiplayerPayload(snapshot.matchStartPayload)
      : null;
    auditStateHashRef.current = snapshot.auditStateHash;
    initialPublicCheckpointHashRef.current = snapshot.initialPublicCheckpointHash || "";
    matchClockConfigRef.current = cloneMultiplayerPayload(snapshot.matchClockConfig);
    matchClockRef.current = cloneMultiplayerPayload(snapshot.matchClock);
    actionCryptoRequirementsRef.current = new Map(
      [...snapshot.actionCryptoRequirements.entries()].map(([seq, requirements]) => [
        seq,
        cloneMultiplayerPayload(requirements),
      ])
    );
	    relayedActionIdsRef.current = new Set(snapshot.relayedActionIds || []);
	    ziffleHandRevealKeyRef.current = snapshot.ziffleHandRevealKey;
	    ziffleHandRevealQuickKeyRef.current = snapshot.ziffleHandRevealQuickKey || "";
    const restoredState = currentGame && typeof currentGame.uiState === "function"
      ? await currentGame.uiState()
      : cloneMultiplayerPayload(snapshot.state);
    stateRef.current = restoredState;
    setState(restoredState);
    publishMatchClockSnapshot(runtimeMatchClockSnapshot());
    updateMultiplayer((prev) => ({
      ...prev,
      lastAppliedSequence: snapshot.lastAppliedSequence,
      submittingAction: false,
    }));
  }


  return { actionCryptoRequirementsForSequence, actionHistoryEntryForSequence, actionQuorumRoster, actionQuorumThresholdForMessage, actionQuorumVoteCacheKey, actionQuorumVoteConflict, alignMatchClockObservationFromHostSnapshot, answerActionQuorumVoteRequest, answerCryptoMaterialRequest, answerDisconnectForfeitVoteRequest, answerProtocolResponseTimeoutVoteRequest, answerTimeoutVoteRequest, appendAppliedSequencedAction, authorizedCryptoMaterialRequirementsForRequest, batchedOwnerPrivateZiffleOpeningsForLocalViewer, broadcastMatchPresence, broadcastToClients, buildHostedResyncPayload, buildLocalCryptoMaterialForRequirements, buildLocalPrivateViewProofsForRequirements, buildMatchClockAuditForCommand, clearAllPeerResyncs, clearLocalDisconnectObservation, collectActionQuorumCertificate, collectDisconnectForfeitCertificateForCommand, collectProtocolResponseTimeoutCertificateForCommand, collectRemoteCryptoMaterialForRequirements, collectTimeoutCertificateForCommand, commandObjectHiddenRefs, commandObjectStableIds, commitMatchClockAudit, createSequencedActionValidationSnapshot, cryptoRequirementReplayKey, currentHiddenRefForObjectId, currentMatchClockSnapshot, currentObjectIdForHiddenRef, currentObjectIdForStableId, currentStableIdForObjectId, derivePostApplyCryptoRequirementsForRequest, disconnectForfeitRoster, filterOpeningsForCommandHiddenRefs, finishPeerResync, forfeitedPlayersForQuorum, freshCryptoRequirementsForSequence, handleHistoricalSequencedAction, hiddenPositionBatchRevealFromOpening, injectCryptoMaterialForRequirements, latestMatchClockAuditFromActions, leaveLobby, localDisconnectObservationForPlayer, markMatchDisputed, openingMatchesCommandHiddenRef, playerCountForClock, playerForDisconnectForfeit, playerForProtocolResponseTimeout, privateOpeningFromEncryptedProof, privateOpeningFromProof, privateOpeningsForLocalViewer, protocolResponseTimeoutRoster, publishCurrentRuntimeState, publishMatchClockSnapshot, relaySequencedAction, remapCommandForLocalHiddenOpening, remapPriorityCommandForLocalHiddenOpening, remapSelectObjectsCommandForLocalHiddenOpening, rememberActionCryptoRequirements, rememberLocalDisconnectObservation, rememberSignedActionQuorumVote, resetMatchClockForMatch, resolvePeerResyncWaitersIfIdle, restoreMatchClockRuntime, restoreMatchClockRuntimeFromActionTranscript, restoreSequencedActionValidationSnapshot, revealPrivateAuditProofsForLocalViewer, revealPrivateOpeningsForInjection, runtimeMatchClockSnapshot, sendHostedStateMessage, sendMatchStartToClients, sequencedActionRelayKey, sequencedActionsEquivalent, shuffleProofAlreadyAppliedBefore, shuffleProofReplayKey, shuffleProofRequirementAlreadyRecordedBefore, signActionQuorumVoteForMessage, signDisconnectForfeitVoteForCommand, signProtocolResponseTimeoutVoteForCommand, signTimeoutVoteForSnapshot, stageLocalMatchClockAudit, stateHashBeforeSequence, teardownPeer, updateMatchClockForState, validateDisconnectForfeitCommand, validateProtocolResponseTimeoutCommand, validateTimeoutForfeitCommand, validateTrustedSequencedAction, verifyActionQuorumForMessage, verifyActionQuorumVoteForMessage, verifyMatchClockAuditForAction, verifyTimeoutCertificate, verifyTimeoutVote, waitForPeerResyncs };
}
