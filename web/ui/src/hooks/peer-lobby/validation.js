import { assertMatchNotDisputed, isMatchDisputed } from "./match-lifecycle.js";
import { acceptedZiffleEpochs, assertZiffleEpochInputs, assertZiffleEpochVerification, buildZiffleInputDeck, isPrivateZiffleEpoch, ziffleEpochMaterial, ziffleInputDeckFields } from "../../lib/ziffle-private-epochs.js";
import { assertRuntimeVersion } from "../../lib/runtime-version.js";
import { collectZiffleRevealTokenGroups } from "../../lib/ziffle-reveal-token-collection.js";
import { PUBLIC_FORMATS, isRelayId } from '../../lib/relay/formats.js';
import { loadFormatCatalog, assertFormatMatch } from '../../lib/relay/format-legality.js';
import {
  DEFAULT_OPENING_HAND_SIZE,
  INITIAL_AUDIT_STATE_HASH,
  MATCH_CLOCK_CLAIM_SKEW_MS,
  MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS,
  MULTIPLAYER_SECURITY_TRUSTED,
  MULTIPLAYER_SECURITY_VERIFIED,
  PROTOCOL_RESPONSE_TIMEOUT_MS,
  PROTOCOL_VERSION,
  actionIntentKey,
  actionTimerSnapshotFromMatchClock,
  alignShuffleProofsWithRequirements,
  buildDeckSlotOpening,
  buildZiffleOpeningProof,
  canonicalJson,
  canonicalMultiplayerPayload,
  clearStoredRevealedOpeningsForMatch,
  cloneMultiplayerPayload,
  compactZiffleCeremonyForDiagnostics,
  cryptoRequirementsFromState,
  emitSyncFailureNotice,
  fairRandomCombinedSeedHex,
  filterCryptoRequirementsForCommand,
  findLocalMatchPlayer,
  firstOnlinePeerId,
  hiddenCardMetadataForObjectFromCheckpoint,
  ziffleOriginAnchorFromMetadata,
  hiddenObjectIdForOpeningFromCheckpoint,
  isActionTimeoutForfeitCommand,
  isCurrentAuditPlayerCount,
  isDecisionCommandCompatible,
  isDisconnectTimeoutForfeitCommand,
  isForfeitCommand,
  isProtocolResponseTimeoutForfeitCommand,
  isWitnessForfeitCommand,
  isRejectedActionCheatReason,
  cheatOffenderForError,
  selectObjectCandidateForId,
  selectObjectCandidateRevealPolicy,
  isSelfForfeitCommand,
  isSorcerySpeedForfeitState,
  isSupportedZiffleDeckCount,
  isTrustedMultiplayerSecurityMode,
  isUnauthorizedAddCardCommand,
  isVerifiedMultiplayerSecurityMode,
  localizeShuffleOrder,
  matchPayloadSecurityMode,
  normalizeMatchFormat,
  normalizePlayerIndex,
  normalizeShuffleOrder,
  normalizeZiffleCardPositions,
  nowMonotonicMs,
  openingHasZifflePosition,
  payloadSizeBytes,
  playerNameForIndex,
  protocolResponseTimeoutClaimFromError,
  publicCheckpointHash,
  publicDeckManifest,
  randomAuditHex,
  recordPeerSyncPerf,
  reindexPlayers,
  resolveLocalPlayerIndex,
  rngCommitmentPayload,
  rngRevealPayload,
  safeSend,
  sameShuffleOrder,
  sequencedActionSecurityMode,
  pinnedSessionSecurityMode,
  sessionSecurityMode,
  sha256Hex,
  shuffleOrderIdMap,
  shuffleProofMatchesRequirement,
  signAuditPayload,
  sleep,
  summarizePeerCommand,
  timePeerSyncPhase,
  toErrorMessage,
  useCallback,
  publicDecklistsForMatchPayload,
  validationCommandersForMatchPayload,
  validationDecksForMatchPayload,
  validationPlanarDecksForMatchPayload,
  validationSideboardsForMatchPayload,
  verifyAuditPayload,
  verifyCardOpeningAgainstManifest,
  assertZiffleOpeningOriginMatchesMetadata,
  verifySignedMatchGenesis,
  wasmObjectIdArg,
  writeStoredPlayerIndex,
  ziffleContextFromCeremony,
  ziffleDeckHashFromCommitment,
  ziffleIdentityPositionFromSources,
  ziffleKeyContextForCeremony,
  zifflePositionFromCommitment,
  zifflePublicPositionFromSources,
  ziffleRevealTokenTimeoutMs,
  ziffleRuntimeCommitment,
} from "./shared.js";
import { recordDiagnosticEvent } from "../../lib/action-diagnostics.js";
import { disclosureDueForPlayer } from "./end-of-match-disclosure.js";
import { assertGenesisSeedRevealBinding, assertVerifiedMatchStartIsFresh, rememberStartedGenesis, rotateGenesisSeedNonce } from "./genesis-binding.js";
import { sequencedActionSignedPayload, initialAuditStateHashForMatch } from "../../lib/multiplayer-audit.js";
import { genesisAuditKeyLookup, signZiffleShuffleStep, verifyZiffleCeremonyStepSignatures } from "../../lib/ziffle-step-signatures.js";

export function usePeerLobbyValidation(base, servicesRef) {
  const { ziffleActionRevealLocksRef } = base;
  const { actionCryptoRequirementsRef, actionHistoryRef, applySyncedCommand, applyingSequencedActionsRef, auditEncryptionPublicKeyRef, auditPublicKeyRef, auditStateHashRef, awaitingStateResyncRef, clientConnectionsRef, drainingPendingSequencedActionsRef, gameRef, hostConnectionRef, initialPublicCheckpointHashRef, liveAuditTranscriptRef, liveZiffleCeremoniesRef, localRevealedOpeningsRef, localZiffleCeremonyLookupRef, localZiffleRevealInFlightRef, matchClockObservationExemptSequenceRef, matchStartPayloadRef, multiplayerRef, outboundCryptoMaterialRequestsRef, peerConnectionsRef, pendingSequencedActionsRef, privateViewDisclosuresRef, relayedActionIdsRef, rngCommitNoncesRef, rngRevealCommitSetLocksRef, setState, setStatus, signedRngCommitmentsRef, stateRef, verifiedAuditOpeningsRef, verifiedShuffleProofsRef, ziffleHandRevealKeyRef, ziffleHandRevealQuickKeyRef, ziffleOpeningPositionsRef, ziffleRevealTokenCacheRef, ziffleShufflePerfRef } = base;
  const actionCryptoRequirementsForSequence = useCallback((...args) => servicesRef.current.actionCryptoRequirementsForSequence(...args), [servicesRef]);
  const actionHistoryEntryForSequence = useCallback((...args) => servicesRef.current.actionHistoryEntryForSequence(...args), [servicesRef]);
  const appendAppliedSequencedAction = useCallback((...args) => servicesRef.current.appendAppliedSequencedAction(...args), [servicesRef]);
  const beginPeerWait = useCallback((...args) => servicesRef.current.beginPeerWait(...args), [servicesRef]);
  const buildDeckSlotOpeningForExport = useCallback((...args) => servicesRef.current.buildDeckSlotOpeningForExport(...args), [servicesRef]);
  const buildOpeningFromResolvedCommittedSlot = useCallback((...args) => servicesRef.current.buildOpeningFromResolvedCommittedSlot(...args), [servicesRef]);
  const cachedZiffleRevealTokens = useCallback((...args) => servicesRef.current.cachedZiffleRevealTokens(...args), [servicesRef]);
  const clearOwnerZiffleOpeningCache = useCallback((...args) => servicesRef.current.clearOwnerZiffleOpeningCache(...args), [servicesRef]);
  const clearPeerWait = useCallback((...args) => servicesRef.current.clearPeerWait(...args), [servicesRef]);
  const commitMatchClockAudit = useCallback((...args) => servicesRef.current.commitMatchClockAudit(...args), [servicesRef]);
  const connectDirectPeer = useCallback((...args) => servicesRef.current.connectDirectPeer(...args), [servicesRef]);
  const createSequencedActionValidationSnapshot = useCallback((...args) => servicesRef.current.createSequencedActionValidationSnapshot(...args), [servicesRef]);
  const cryptoRequirementReplayKey = useCallback((...args) => servicesRef.current.cryptoRequirementReplayKey(...args), [servicesRef]);
  const currentAuditMatchId = useCallback((...args) => servicesRef.current.currentAuditMatchId(...args), [servicesRef]);
  const currentHiddenCardMetadataForObject = useCallback((...args) => servicesRef.current.currentHiddenCardMetadataForObject(...args), [servicesRef]);
  const currentPublicAuditCheckpointHash = useCallback((...args) => servicesRef.current.currentPublicAuditCheckpointHash(...args), [servicesRef]);
  const ensureAuditIdentity = useCallback((...args) => servicesRef.current.ensureAuditIdentity(...args), [servicesRef]);
  const ensureDirectPeerConnections = useCallback((...args) => servicesRef.current.ensureDirectPeerConnections(...args), [servicesRef]);
  const ensureZiffleIdentity = useCallback((...args) => servicesRef.current.ensureZiffleIdentity(...args), [servicesRef]);
  const freshCryptoRequirementsForSequence = useCallback((...args) => servicesRef.current.freshCryptoRequirementsForSequence(...args), [servicesRef]);
  const handleHistoricalSequencedAction = useCallback((...args) => servicesRef.current.handleHistoricalSequencedAction(...args), [servicesRef]);
  const importCachedAuditPublicKey = useCallback((...args) => servicesRef.current.importCachedAuditPublicKey(...args), [servicesRef]);
  const injectCryptoMaterialForRequirements = useCallback((...args) => servicesRef.current.injectCryptoMaterialForRequirements(...args), [servicesRef]);
  const localRevealedOpeningForExport = useCallback((...args) => servicesRef.current.localRevealedOpeningForExport(...args), [servicesRef]);
  const makeZiffleRequestId = useCallback((...args) => servicesRef.current.makeZiffleRequestId(...args), [servicesRef]);
  const previewRequirementsForCommand = useCallback((...args) => servicesRef.current.previewRequirementsForCommand(...args), [servicesRef]);
  const privateDeckManifestForOwner = useCallback((...args) => servicesRef.current.privateDeckManifestForOwner(...args), [servicesRef]);
  const publicKeyForAuditSigner = useCallback((...args) => servicesRef.current.publicKeyForAuditSigner(...args), [servicesRef]);
  const publishCurrentRuntimeState = useCallback((...args) => servicesRef.current.publishCurrentRuntimeState(...args), [servicesRef]);
  const relaySequencedAction = useCallback((...args) => servicesRef.current.relaySequencedAction(...args), [servicesRef]);
  const remapCommandForLocalHiddenOpening = useCallback((...args) => servicesRef.current.remapCommandForLocalHiddenOpening(...args), [servicesRef]);
  const rememberActionCryptoRequirements = useCallback((...args) => servicesRef.current.rememberActionCryptoRequirements(...args), [servicesRef]);
  const rememberLocalRevealedOpening = useCallback((...args) => servicesRef.current.rememberLocalRevealedOpening(...args), [servicesRef]);
  const rememberLocalZiffleCeremonyForLookup = useCallback((...args) => servicesRef.current.rememberLocalZiffleCeremonyForLookup(...args), [servicesRef]);
  const rememberPendingActionIntent = useCallback((...args) => servicesRef.current.rememberPendingActionIntent(...args), [servicesRef]);
  const rememberZiffleOpeningPosition = useCallback((...args) => servicesRef.current.rememberZiffleOpeningPosition(...args), [servicesRef]);
  const rememberZiffleRevealTokens = useCallback((...args) => servicesRef.current.rememberZiffleRevealTokens(...args), [servicesRef]);
  const reportSyncFailure = useCallback((...args) => servicesRef.current.reportSyncFailure(...args), [servicesRef]);
  const resetMatchClockForMatch = useCallback((...args) => servicesRef.current.resetMatchClockForMatch(...args), [servicesRef]);
  const resolveCommittedSlotForZifflePosition = useCallback((...args) => servicesRef.current.resolveCommittedSlotForZifflePosition(...args), [servicesRef]);
  const resolveCommittedZiffleRevealSlot = useCallback((...args) => servicesRef.current.resolveCommittedZiffleRevealSlot(...args), [servicesRef]);
  const resolveLocalCryptoPlayerIndex = useCallback((...args) => servicesRef.current.resolveLocalCryptoPlayerIndex(...args), [servicesRef]);
  const restoreSequencedActionValidationSnapshot = useCallback((...args) => servicesRef.current.restoreSequencedActionValidationSnapshot(...args), [servicesRef]);
  const revealAuditOpenings = useCallback((...args) => servicesRef.current.revealAuditOpenings(...args), [servicesRef]);
  const revealPrivateAuditProofsForLocalViewer = useCallback((...args) => servicesRef.current.revealPrivateAuditProofsForLocalViewer(...args), [servicesRef]);
  const runtimeManifestForZiffleCeremony = useCallback((...args) => servicesRef.current.runtimeManifestForZiffleCeremony(...args), [servicesRef]);
  const sanitizeObjectBoundOpening = useCallback((...args) => servicesRef.current.sanitizeObjectBoundOpening(...args), [servicesRef]);
  const sendDirectPeerMessage = useCallback((...args) => servicesRef.current.sendDirectPeerMessage(...args), [servicesRef]);
  const sequencedActionsEquivalent = useCallback((...args) => servicesRef.current.sequencedActionsEquivalent(...args), [servicesRef]);
  const shuffleProofAlreadyAppliedBefore = useCallback((...args) => servicesRef.current.shuffleProofAlreadyAppliedBefore(...args), [servicesRef]);
  const shuffleProofRequirementAlreadyRecordedBefore = useCallback((...args) => servicesRef.current.shuffleProofRequirementAlreadyRecordedBefore(...args), [servicesRef]);
  const signActionIntentForCommand = useCallback((...args) => servicesRef.current.signActionIntentForCommand(...args), [servicesRef]);
  const signedZiffleKeysForPayload = useCallback((...args) => servicesRef.current.signedZiffleKeysForPayload(...args), [servicesRef]);
  const submitProtocolResponseTimeoutClaim = useCallback((...args) => servicesRef.current.submitProtocolResponseTimeoutClaim(...args), [servicesRef]);
  const updateMultiplayer = useCallback((...args) => servicesRef.current.updateMultiplayer(...args), [servicesRef]);
  const updatePeerWait = useCallback((...args) => servicesRef.current.updatePeerWait(...args), [servicesRef]);
  const validateDisconnectForfeitCommand = useCallback((...args) => servicesRef.current.validateDisconnectForfeitCommand(...args), [servicesRef]);
  const validateProtocolResponseTimeoutCommand = useCallback((...args) => servicesRef.current.validateProtocolResponseTimeoutCommand(...args), [servicesRef]);
  const validateTimeoutForfeitCommand = useCallback((...args) => servicesRef.current.validateTimeoutForfeitCommand(...args), [servicesRef]);
  const validateTrustedSequencedAction = useCallback((...args) => servicesRef.current.validateTrustedSequencedAction(...args), [servicesRef]);
  const verifyActionMatchesPendingIntent = useCallback((...args) => servicesRef.current.verifyActionMatchesPendingIntent(...args), [servicesRef]);
  const verifyActionQuorumForMessage = useCallback((...args) => servicesRef.current.verifyActionQuorumForMessage(...args), [servicesRef]);
  const verifyAuditSatisfiesCryptoRequirements = useCallback((...args) => servicesRef.current.verifyAuditSatisfiesCryptoRequirements(...args), [servicesRef]);
  const assertAuditOpeningsExpected = useCallback((...args) => servicesRef.current.assertAuditOpeningsExpected(...args), [servicesRef]);
  const commandObjectIdsForOpeningAllowList = useCallback((...args) => servicesRef.current.commandObjectIdsForOpeningAllowList(...args), [servicesRef]);
  const verifyCurrentPublicCheckpointHash = useCallback((...args) => servicesRef.current.verifyCurrentPublicCheckpointHash(...args), [servicesRef]);
  const verifyMatchClockAuditForAction = useCallback((...args) => servicesRef.current.verifyMatchClockAuditForAction(...args), [servicesRef]);
  const verifySequencedActionAudit = useCallback((...args) => servicesRef.current.verifySequencedActionAudit(...args), [servicesRef]);
  const verifySignedActionIntent = useCallback((...args) => servicesRef.current.verifySignedActionIntent(...args), [servicesRef]);
  const waitForProtocolResponse = useCallback((...args) => servicesRef.current.waitForProtocolResponse(...args), [servicesRef]);
  const waitForRngCommit = useCallback((...args) => servicesRef.current.waitForRngCommit(...args), [servicesRef]);
  const waitForRngReveal = useCallback((...args) => servicesRef.current.waitForRngReveal(...args), [servicesRef]);
  const waitForZiffleRevealToken = useCallback((...args) => servicesRef.current.waitForZiffleRevealToken(...args), [servicesRef]);
  const waitForZiffleShuffleStep = useCallback((...args) => servicesRef.current.waitForZiffleShuffleStep(...args), [servicesRef]);
  const ziffleCeremonyForOwner = useCallback((...args) => servicesRef.current.ziffleCeremonyForOwner(...args), [servicesRef]);
  const ziffleCeremonyHasObjectOrder = useCallback((...args) => servicesRef.current.ziffleCeremonyHasObjectOrder(...args), [servicesRef]);
  const zifflePositionForObjectId = useCallback((...args) => servicesRef.current.zifflePositionForObjectId(...args), [servicesRef]);
  const zifflePositionForOriginalSlot = useCallback((...args) => servicesRef.current.zifflePositionForOriginalSlot(...args), [servicesRef]);
  const zifflePublicKeysForPlayers = useCallback((...args) => servicesRef.current.zifflePublicKeysForPlayers(...args), [servicesRef]);
  const ziffleTokensForPosition = useCallback((...args) => servicesRef.current.ziffleTokensForPosition(...args), [servicesRef]);
  function sequencedActionValidationSnapshotStillCurrent(snapshot) {
    if (!snapshot) return false;
    const snapshotSequence = Number(snapshot.lastAppliedSequence || 0);
    const currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    const currentHistorySequence = Number(
      actionHistoryRef.current.at(-1)?.seq ?? currentSequence
    );
    return (
      currentSequence === snapshotSequence
      && currentHistorySequence === snapshotSequence
    );
  }

  async function restoreSequencedActionValidationSnapshotIfCurrent(snapshot) {
    if (!sequencedActionValidationSnapshotStillCurrent(snapshot)) {
      recordPeerSyncPerf("validation_snapshot:skip_restore", {
        snapshot_sequence: Number(snapshot?.lastAppliedSequence || 0),
        current_sequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
        current_history_sequence: Number(
          actionHistoryRef.current.at(-1)?.seq ?? multiplayerRef.current.lastAppliedSequence ?? 0
        ),
      });
      return false;
    }
    await restoreSequencedActionValidationSnapshot(snapshot);
    return true;
  }

  function bufferedSequencedActionOptions(options = {}) {
    return {
      ...(options.relay === false ? { relay: false } : {}),
      ...(options.skipQuorumCertificate === true ? { skipQuorumCertificate: true } : {}),
      ...(options.enforceMatchClockObservationBounds === false
        ? { enforceMatchClockObservationBounds: false }
        : {}),
      ...(options.failureResyncReason ? { failureResyncReason: options.failureResyncReason } : {}),
      ...(options.resyncReason ? { resyncReason: options.resyncReason } : {}),
      ...(options.allowWhileAwaitingResync ? { allowWhileAwaitingResync: true } : {}),
    };
  }

  function bufferFutureSequencedAction(message, options = {}) {
    const seq = Number(message?.seq || 0);
    if (!Number.isSafeInteger(seq) || seq <= 0) return false;
    const existing = pendingSequencedActionsRef.current.get(seq);
    if (existing) {
      if (sequencedActionsEquivalent(existing.message, message)) {
        return true;
      }
      return false;
    }
    pendingSequencedActionsRef.current.set(seq, {
      message: cloneMultiplayerPayload(message),
      options: bufferedSequencedActionOptions(options),
      receivedAtMs: Date.now(),
    });
    recordPeerSyncPerf("apply_action:buffer_future", {
      seq,
      expected: Number(multiplayerRef.current.lastAppliedSequence || 0) + 1,
      actor: Number(message?.actorIndex ?? message?.audit?.actor ?? -1),
      pending: pendingSequencedActionsRef.current.size,
    });
    return true;
  }

  async function drainPendingSequencedActions() {
    if (drainingPendingSequencedActionsRef.current) return;
    drainingPendingSequencedActionsRef.current = true;
    try {
      while (!awaitingStateResyncRef.current && !isMatchDisputed(multiplayerRef.current)) {
        const nextSequence = Number(multiplayerRef.current.lastAppliedSequence || 0) + 1;
        const pending = pendingSequencedActionsRef.current.get(nextSequence);
        if (!pending) break;
        pendingSequencedActionsRef.current.delete(nextSequence);
        recordPeerSyncPerf("apply_action:drain_future", {
          seq: nextSequence,
          pending: pendingSequencedActionsRef.current.size,
        });
        await applySequencedActionMessage(pending.message, {
          ...pending.options,
          fromBufferedSequencedAction: true,
        });
      }
    } finally {
      drainingPendingSequencedActionsRef.current = false;
    }
  }

  async function applySequencedActionMessage(message, options = {}) {
    assertMatchNotDisputed(multiplayerRef.current, "Sequenced action");
    const nextSequence = Number(message?.seq || 0);
    if (
      !options.dryRun
      && Number.isSafeInteger(nextSequence)
      && nextSequence > 0
    ) {
      const inFlight = applyingSequencedActionsRef.current.get(nextSequence);
      if (inFlight) {
        try {
          await inFlight.promise;
        } catch (err) {
          if (options.throwOnFailure) {
            throw err;
          }
          return { duplicate: true, coalesced: true, failed: true };
        }
        const existingAction = actionHistoryEntryForSequence(nextSequence);
        if (existingAction && sequencedActionsEquivalent(existingAction, message)) {
          if (options.relay !== false) {
            relaySequencedAction(message);
          }
          return { duplicate: true, coalesced: true };
        }
      }
      const promise = applySequencedActionMessageInner(message, options);
      applyingSequencedActionsRef.current.set(nextSequence, {
        message: cloneMultiplayerPayload(message),
        promise,
      });
      try {
        return await promise;
      } finally {
        const current = applyingSequencedActionsRef.current.get(nextSequence);
        if (current?.promise === promise) {
          applyingSequencedActionsRef.current.delete(nextSequence);
        }
      }
    }
    return applySequencedActionMessageInner(message, options);
  }

  async function applySequencedActionMessageInner(message, options = {}) {
    assertMatchNotDisputed(multiplayerRef.current, "Sequenced action");
    const nextSequence = Number(message?.seq || 0);
    const session = multiplayerRef.current;
    const dryRun = Boolean(options.dryRun);
    const throwOnFailure = Boolean(options.throwOnFailure);
    const existingAction = actionHistoryEntryForSequence(nextSequence);
    if (existingAction && sequencedActionsEquivalent(existingAction, message)) {
      if (options.relay !== false) {
        relaySequencedAction(message);
      }
      return { duplicate: true };
    }
    if (awaitingStateResyncRef.current && !options.allowWhileAwaitingResync) {
      if (dryRun || options.throwOnOrderMismatch || throwOnFailure) {
        throw new Error("Cannot validate action while awaiting state resync");
      }
      return;
    }
    if (nextSequence <= session.lastAppliedSequence) {
      if (dryRun || options.throwOnOrderMismatch || throwOnFailure) {
        throw new Error("Action quorum request is not ahead of the local transcript");
      }
      await handleHistoricalSequencedAction(message);
      return;
    }
    // A future payload must be signed before it can pause this match for
    // recovery. Its chain and material are verified again after the gap closes.
    if (nextSequence > session.lastAppliedSequence + 1
      && isVerifiedMultiplayerSecurityMode(sessionSecurityMode(session))) {
      if (nextSequence > session.lastAppliedSequence + 64
        || (pendingSequencedActionsRef.current.size >= 64
          && !pendingSequencedActionsRef.current.has(nextSequence))) {
        throw new Error("Future action exceeds the bounded recovery window");
      }
      const audit = message.audit;
      if (!audit || audit.matchId !== currentAuditMatchId()
        || Number(audit.seq) !== nextSequence || Number(audit.actor) !== Number(message.actorIndex)
        || Number(audit.signer ?? audit.actor) !== Number(message.actorIndex)
        || canonicalMultiplayerPayload(audit.command) !== canonicalMultiplayerPayload(message.command)) {
        throw new Error("Future action has an invalid signed envelope");
      }
      const key = await importCachedAuditPublicKey(publicKeyForAuditSigner(message.actorIndex));
      if (!await verifyAuditPayload(key, sequencedActionSignedPayload(message), audit.signature || "")) {
        throw new Error("Future action signature is invalid");
      }
      if (Number(multiplayerRef.current.lastAppliedSequence) !== Number(session.lastAppliedSequence)) {
        return applySequencedActionMessageInner(message, options);
      }
    }
    if (nextSequence !== session.lastAppliedSequence + 1) {
      if (
        !dryRun
        && !options.throwOnOrderMismatch
        && !throwOnFailure
        && nextSequence > session.lastAppliedSequence + 1
        && bufferFutureSequencedAction(message, options)
      ) {
        setStatus(`Waiting for action ${session.lastAppliedSequence + 1}`);
        servicesRef.current.requestMissingSequencedActions?.(message);
        if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
          servicesRef.current.requestResync("Recovering missing accepted actions", { forceCheckpoint: false });
        }
        return { buffered: true };
      }
      if (dryRun || options.throwOnOrderMismatch || throwOnFailure) {
        throw new Error(
          `Action order mismatch. Expected ${session.lastAppliedSequence + 1}, received ${nextSequence}.`
        );
      }
      reportSyncFailure(
        `Action order mismatch. Expected ${session.lastAppliedSequence + 1}, received ${nextSequence}.`,
        options.resyncReason || "Multiplayer action order mismatch. Resyncing with host...",
        "Multiplayer action order mismatch"
      );
      return;
    }

    // Every apply (dry run, trusted and audited) is transactional: a failure
    // after the engine or crypto bookkeeping has been mutated (post-reveal
    // checks, checkpoint hash mismatch, reveal-token timeouts) must leave this
    // seat exactly at N-1 so a later copy of action N, or a resync, applies on
    // a consistent base. Runtime savepoints make this an engine-side handle;
    // it is released as soon as the action commits and always in `finally`.
    const validationSnapshot = await createSequencedActionValidationSnapshot();
    let snapshotRestored = false;
    let snapshotReleased = false;
    const restoreValidationSnapshot = async () => {
      if (!validationSnapshot || snapshotRestored || snapshotReleased) return;
      snapshotRestored = await restoreSequencedActionValidationSnapshotIfCurrent(validationSnapshot);
      if (!snapshotRestored) {
        throw new Error("Action validation snapshot was superseded by a newer action");
      }
    };
    const releaseValidationSnapshot = async () => {
      if (snapshotReleased) return;
      snapshotReleased = true;
      await validationSnapshot?.release?.();
    };

    if (!dryRun) {
      updateMultiplayer((prev) => ({ ...prev, submittingAction: true }));
    }
    let applyPhase = "init";
    // Per-phase wall time for the received action, reported once when it lands.
    const applyPhaseNow = () => (globalThis.performance?.now?.() ?? Date.now());
    const applyPhaseStartedAt = applyPhaseNow();
    let applyPhaseMarkedAt = applyPhaseStartedAt;
    const applyPhaseMarks = [];
    const markApplyPhase = (name) => {
      assertMatchNotDisputed(multiplayerRef.current, "Sequenced action");
      const at = applyPhaseNow();
      applyPhaseMarks.push({ phase: applyPhase, ms: Math.round(at - applyPhaseMarkedAt) });
      applyPhaseMarkedAt = at;
      return name;
    };
    const applyPhaseReport = () => ({
      seq: Number(message?.seq),
      actor: message?.actorIndex,
      dry_run: Boolean(dryRun),
      total_ms: Math.round(applyPhaseNow() - applyPhaseStartedAt),
      phases: [...applyPhaseMarks, { phase: applyPhase, ms: Math.round(applyPhaseNow() - applyPhaseMarkedAt) }],
    });
    try {
      assertMatchNotDisputed(multiplayerRef.current, "Sequenced action");
      const localSecurityMode = sessionSecurityMode(
        session,
        matchPayloadSecurityMode(matchStartPayloadRef.current, MULTIPLAYER_SECURITY_VERIFIED)
      );
      const messageSecurityMode = sequencedActionSecurityMode(message, session);
      if (messageSecurityMode !== localSecurityMode) {
        throw new Error(
          `Sequenced action security mode mismatch. Expected ${localSecurityMode}, received ${messageSecurityMode}.`
        );
      }
      if (isTrustedMultiplayerSecurityMode(localSecurityMode)) {
        applyPhase = markApplyPhase("pre_apply_checks");
        // Reject a transcript gap or prefix mismatch before the engine mutates
        // anything: a failure here must leave the local state exactly as it was so
        // the host checkpoint resync that follows starts from a consistent seat.
        if (!dryRun) servicesRef.current.assertAcceptedActionExtendsTranscript(message);
        const liveStateForClock = gameRef.current
          ? await gameRef.current.uiState()
          : stateRef.current;
        await validateTrustedSequencedAction({
          command: message.command,
          actorIndex: message.actorIndex,
          seq: nextSequence,
          clock: message.clock,
          uiState: liveStateForClock,
          // Trusted mode uses the clock audit for convergence, but local observation
          // bounds are a Verified anti-cheat check and can diverge between peers.
          enforceMatchClockObservationBounds: false,
        });
        applyPhase = markApplyPhase("apply_command");
        const appliedState = await applySyncedCommand(message.command, message.label || "", {
          actorIndex: message.actorIndex,
          sequence: nextSequence,
          preState: liveStateForClock,
          publishState: false,
        });
        assertMatchNotDisputed(multiplayerRef.current, "Sequenced action");
        if (dryRun) {
          await restoreValidationSnapshot();
          return { trusted: true };
        }
        commitMatchClockAudit(message.clock, appliedState);
        await appendAppliedSequencedAction({
          ...message,
          securityMode: MULTIPLAYER_SECURITY_TRUSTED,
        });
        await releaseValidationSnapshot();
        recordDiagnosticEvent("apply_action:applied", applyPhaseReport());
        if (Number(nextSequence) === Number(matchClockObservationExemptSequenceRef.current || 0)) {
          matchClockObservationExemptSequenceRef.current = 0;
        }
        await publishCurrentRuntimeState(appliedState);
        if (options.relay !== false) {
          relaySequencedAction({
            ...message,
            securityMode: MULTIPLAYER_SECURITY_TRUSTED,
          });
        }
        servicesRef.current.notifySequencedActionRecovery?.();
      await drainPendingSequencedActions();
        return { trusted: true };
      }
      applyPhase = markApplyPhase("verify_audit");
      await verifySequencedActionAudit({
        audit: message.audit,
        seq: nextSequence,
        actorIndex: message.actorIndex,
        command: message.command,
      });
      applyPhase = markApplyPhase("verify_pending_intent");
      if (fairRandomRevealLockConflict({
        matchId: message.audit?.matchId,
        seq: message.audit?.seq ?? nextSequence,
        actorIndex: message.audit?.actor ?? message.actorIndex,
        prevStateHash: message.audit?.prevStateHash,
        command: message.command,
      })) {
        throw new Error(
          "Sequenced action does not match pending fair-random intent revealed for this sequence"
        );
      }
      const pendingIntentVerification = await verifyActionMatchesPendingIntent(message);
      if (!options.skipQuorumCertificate) {
        applyPhase = markApplyPhase("verify_quorum");
        await verifyActionQuorumForMessage(message);
      }
      applyPhase = markApplyPhase("pre_apply_checks");
      const liveStateForClock = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
      const auditSigner = Number(message.audit?.signer ?? message.actorIndex);
      if (auditSigner !== Number(message.actorIndex)) {
        throw new Error("Sequenced action must be signed by the acting player");
      }
      if (isUnauthorizedAddCardCommand(message.command)) {
        const actorName = playerNameForIndex(session.players, message.actorIndex);
        throw new Error(`Unauthorized add-card action from ${actorName}`);
      }
      const expectedActor = gameRef.current
        ? (await gameRef.current.uiState())?.decision?.player
        : null;
      const isTimeoutForfeit = isActionTimeoutForfeitCommand(message.command);
      const isDisconnectForfeit = isDisconnectTimeoutForfeitCommand(message.command);
      const isProtocolTimeoutForfeit = isProtocolResponseTimeoutForfeitCommand(message.command);
      const isWitnessForfeit = isWitnessForfeitCommand(message.command);
      const isSelfForfeit = isSelfForfeitCommand(message.command, message.actorIndex);
      if (isSelfForfeit && !isSorcerySpeedForfeitState(liveStateForClock, message.actorIndex)) {
        throw new Error("Surrender is only available at sorcery speed");
      }
      if (
        isForfeitCommand(message.command)
        && !isTimeoutForfeit
        && !isDisconnectForfeit
        && !isProtocolTimeoutForfeit
        && !isWitnessForfeit
      ) {
        if (Number(message.command.player) !== Number(message.actorIndex)) {
          throw new Error("A player can only forfeit themselves");
        }
      }
      if (isTimeoutForfeit) {
        await validateTimeoutForfeitCommand(message.command, liveStateForClock, {
          skewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
        });
      } else if (isDisconnectForfeit) {
        await validateDisconnectForfeitCommand(message.command, {
          actorIndex: message.actorIndex,
        });
      } else if (isProtocolTimeoutForfeit) {
        await validateProtocolResponseTimeoutCommand(message.command, {
          actorIndex: message.actorIndex,
        });
      } else if (isWitnessForfeit) {
        await servicesRef.current.validateWitnessForfeitCommand(message.command, {
          actorIndex: message.actorIndex,
        });
      } else if (
        expectedActor !== null
        && expectedActor !== undefined
        && Number(expectedActor) !== Number(message.actorIndex)
      ) {
        throw new Error("Sequenced action actor is not the current decision player");
      }
	      const skipMatchClockObservationBounds =
	        Number(nextSequence) === Number(matchClockObservationExemptSequenceRef.current || 0);
	      const actionCarriesCryptoMaterial = Boolean(
	        (message.audit?.openings || []).length
	        || (message.audit?.privateViewProofs || []).length
	        || (message.audit?.shuffleProofs || []).length
	        || (message.audit?.rngReveals || []).length
	      );
	      const actionWasDelayedByProtocolWork = Boolean(
	        pendingIntentVerification?.intentWasHeldForCryptoMaterial
	      );
	      if (!isUnauthorizedAddCardCommand(message.command)) {
	        await verifyMatchClockAuditForAction({
	          clock: message.audit?.clock,
          command: message.command,
          seq: nextSequence,
          actorIndex: message.actorIndex,
          uiState: liveStateForClock,
	          skewMs: Math.max(
	            MATCH_CLOCK_CLAIM_SKEW_MS,
	            MATCH_CLOCK_ELAPSED_UNDERREPORT_SKEW_MS
	          ),
	          // The post-resync exemption forgives an actor claiming less elapsed
	          // time than observed (resync alignment is imprecise). It lets the next
	          // action debit more time than this seat observed only when this seat
	          // never observed the running epoch (fresh rejoin), and such a seat
	          // also refuses to vouch for a timeout on it.
	          enforceObservationBounds:
	            options.enforceMatchClockObservationBounds !== false
	            && !(skipMatchClockObservationBounds
	              && !isActionTimeoutForfeitCommand(message.command)
	              && servicesRef.current.captureMatchClockObservation?.()?.epochHostDerived),
	          enforceUnderreportBounds:
	            options.enforceMatchClockObservationBounds !== false
	            && !skipMatchClockObservationBounds
	            && !actionCarriesCryptoMaterial
	            && !actionWasDelayedByProtocolWork,
	        });
	      }
      applyPhase = markApplyPhase("reveal_pre_openings");
      // Command objects are resolved against the pre-command state; they bound
      // which unrequired-looking openings the actor may ship (see
      // assertAuditOpeningsExpected below).
      const openingAllowedObjectIds = await commandObjectIdsForOpeningAllowList(
        message.command,
        liveStateForClock
      );
      const observedActionRequirements = [];
      const futureEpochHashes = new Set((message.audit?.shuffleProofs || []).filter(isPrivateZiffleEpoch).map(proof => String(proof.deckHash)));
      const immediatePreOpenings = (message.audit?.openings || []).filter(opening =>
        !futureEpochHashes.has(ziffleDeckHashFromCommitment(opening.positionCommitment)));
      await revealAuditOpenings(immediatePreOpenings, {
        timing: "pre",
        command: message.command,
        shuffleProofs: message.audit?.shuffleProofs || [],
        uiState: liveStateForClock,
        updateState: false,
      });
      applyPhase = markApplyPhase("reveal_private_proofs");
      await revealPrivateAuditProofsForLocalViewer(message.audit || {}, {
        updateState: false,
        persistDisclosure: !dryRun,
        deferPrivateEpochDeckHashes: [...futureEpochHashes],
      });
      applyPhase = markApplyPhase("remap_local_command");
      const localCommand = await remapCommandForLocalHiddenOpening(
        message.command,
        message.audit?.openings || [],
        message.actorIndex
      );
      applyPhase = markApplyPhase("verify_public_selections_opened");
      await assertPublicSelectionsOpened(localCommand, liveStateForClock, message.actorIndex);
      applyPhase = markApplyPhase("preview_requirements");
      let cryptoRequirements = filterCryptoRequirementsForCommand(
        localCommand,
        liveStateForClock,
        freshCryptoRequirementsForSequence(
          nextSequence,
          await previewRequirementsForCommand(localCommand)
        )
      );
      for (const objectId of await commandObjectIdsForOpeningAllowList(localCommand, liveStateForClock)) {
        openingAllowedObjectIds.add(Number(objectId));
      }
      observedActionRequirements.push(...cryptoRequirements);
      const preparedActionMaterial = futureEpochHashes.size > 0
        || (message.audit?.rngReveals || []).length > 0
        || (Number(matchStartPayloadRef.current?.protocolVersion) >= 15
          && cryptoRequirements.some(requirement => ["verifiable_shuffle", "fair_random"].includes(ziffleRequirementType(requirement))));
      if (preparedActionMaterial) {
        cryptoRequirements = await previewZiffleActionRequirements({ command: localCommand, seq: nextSequence,
          shuffleProofs: message.audit?.shuffleProofs || [], openings: message.audit?.openings || [],
          rngReveals: message.audit?.rngReveals || [] }, cryptoRequirements, { install: true });
        observedActionRequirements.push(...cryptoRequirements);
        await revealAuditOpenings(message.audit.openings || [], { timing: "pre", command: localCommand,
          shuffleProofs: message.audit.shuffleProofs, uiState: liveStateForClock, updateState: false });
      }
      rememberActionCryptoRequirements(nextSequence, cryptoRequirements);
      applyPhase = markApplyPhase("verify_shuffle_proofs");
      await verifyShuffleProofsForRequirements(
        cryptoRequirements,
        message.audit?.shuffleProofs || [],
        { allowAfterOrderMismatch: true, seq: nextSequence }
      );
      if (message.audit?.shuffleProofs?.length > 0 && !preparedActionMaterial) {
        await injectCryptoMaterialForRequirements(cryptoRequirements, message.audit, { randomnessOnly: true });
        cryptoRequirements = filterCryptoRequirementsForCommand(
          localCommand, liveStateForClock,
          freshCryptoRequirementsForSequence(nextSequence, await previewRequirementsForCommand(localCommand))
        );
        rememberActionCryptoRequirements(nextSequence, cryptoRequirements, { replace: true });
      }
      applyPhase = markApplyPhase("verify_crypto_requirements");
      await verifyAuditSatisfiesCryptoRequirements({
        requirements: cryptoRequirements,
        audit: message.audit,
      });
      applyPhase = markApplyPhase("inject_crypto_material");
      await injectCryptoMaterialForRequirements(cryptoRequirements, message.audit || {}, {
        command: localCommand,
        seq: nextSequence,
        actorIndex: message.actorIndex,
        requirements: cryptoRequirements,
        updateState: false,
        persistDisclosure: !dryRun,
        skipRandomness: preparedActionMaterial || message.audit?.shuffleProofs?.length > 0,
      });
      const liveStateBeforeApply = gameRef.current
        ? await gameRef.current.uiState()
        : liveStateForClock;
      const expectedActorBeforeApply = liveStateBeforeApply?.decision?.player;
      if (
        expectedActorBeforeApply !== null
        && expectedActorBeforeApply !== undefined
        && Number(expectedActorBeforeApply) !== Number(message.actorIndex)
      ) {
        throw new Error("Sequenced action actor is not the current decision player");
      }
      applyPhase = markApplyPhase("apply_command");
      const publishAppliedStateImmediately = false;
      const appliedState = await applySyncedCommand(localCommand, message.label || "", {
        actorIndex: message.actorIndex,
        sequence: nextSequence,
        publishState: publishAppliedStateImmediately,
      });
      const appliedCryptoRequirements = filterCryptoRequirementsForCommand(
        localCommand,
        liveStateForClock,
        freshCryptoRequirementsForSequence(
          nextSequence,
          cryptoRequirementsFromState(appliedState)
        )
      );
      rememberActionCryptoRequirements(nextSequence, appliedCryptoRequirements);
      await verifyShuffleProofsForRequirements(
        appliedCryptoRequirements,
        message.audit?.shuffleProofs || [],
        { seq: nextSequence }
      );
      const actionCryptoRequirements = [...cryptoRequirements, ...appliedCryptoRequirements];
      const actionShuffleProofs = (message.audit?.shuffleProofs || []).filter((proof) =>
        !shuffleProofAlreadyAppliedBefore(nextSequence, proof)
        && !shuffleProofRequirementAlreadyRecordedBefore(nextSequence, proof)
        &&
        actionCryptoRequirements.some((requirement) =>
          shuffleProofMatchesRequirement(proof, requirement)
        )
      );
      const actionShuffleApplicationRequirements = [
        ...appliedCryptoRequirements,
        ...cryptoRequirements,
      ];
      const localizedActionShuffleProofs = alignShuffleProofsWithRequirements(
        actionShuffleProofs,
        actionShuffleApplicationRequirements
      );
      await applyVerifiedShuffleProofs(localizedActionShuffleProofs, {
        requirements: actionCryptoRequirements,
      });
      // Post openings are revealed only after the verified shuffles are applied,
      // matching the actor, which reseals before building its post openings: a
      // card still in the library after the shuffle (Courser of Kruphix, Future
      // Sight, shuffle-then-reveal-top) is opened at its post-shuffle ziffle
      // position, and revealing it before the reseal would be undone by the
      // reseal's redaction.
      const remotePostOpeningState = await revealAuditOpenings(message.audit?.openings || [], {
        timing: "post",
        shuffleProofs: message.audit?.shuffleProofs || [],
        updateState: false,
      });
      await verifyAuditSatisfiesCryptoRequirements({
        requirements: appliedCryptoRequirements,
        audit: message.audit,
      });
      assertAuditOpeningsExpected({
        openings: message.audit?.openings || [],
        requirements: [
          ...observedActionRequirements,
          ...cryptoRequirements,
          ...appliedCryptoRequirements,
        ],
        commandObjectIds: openingAllowedObjectIds,
        owner: message.actorIndex,
        exemptDeckHashes: [...futureEpochHashes],
      });
      await revealPrivateAuditProofsForLocalViewer(message.audit || {}, {
        updateState: false,
        persistDisclosure: !dryRun,
      });
      await revealLocalZiffleHand(matchStartPayloadRef.current, {
        skipIfHandUnchanged: true,
        stateHint: viewedCardsStateHint(remotePostOpeningState, appliedState),
        command: localCommand,
        seq: nextSequence,
        actorIndex: message.actorIndex,
        actionAudit: message.audit,
        requirements: actionCryptoRequirements,
        updateState: false,
      });
      await verifyCurrentPublicCheckpointHash(
        message.audit?.publicCheckpointHash,
        "Sequenced action public checkpoint hash does not match local state"
      );
      assertMatchNotDisputed(multiplayerRef.current, "Sequenced action");
      if (dryRun) {
        await restoreValidationSnapshot();
        recordDiagnosticEvent("action_quorum:dry_run", applyPhaseReport());
        return {
          verified: true,
          publicCheckpointHash: String(message.audit?.publicCheckpointHash || ""),
          nextStateHash: String(message.audit?.nextStateHash || ""),
        };
      }
      commitMatchClockAudit(message.audit?.clock, appliedState);
      await appendAppliedSequencedAction(message);
      await releaseValidationSnapshot();
      recordDiagnosticEvent("apply_action:applied", applyPhaseReport());
      if (Number(nextSequence) === Number(matchClockObservationExemptSequenceRef.current || 0)) {
        matchClockObservationExemptSequenceRef.current = 0;
      }
      if (options.relay !== false) {
        relaySequencedAction(message);
      }
      await publishCurrentRuntimeState(
        viewedCardsStateHint(remotePostOpeningState, appliedState)
      );
      servicesRef.current.notifySequencedActionRecovery?.();
      await drainPendingSequencedActions();
	    } catch (err) {
	      const failureReason = err instanceof Error ? err.message : String(err);
	      const rejectedActionCheat = isRejectedActionCheatReason(failureReason);
	      const unauthorizedAddCardCheat = isUnauthorizedAddCardCommand(message?.command);
	      if (!rejectedActionCheat && !unauthorizedAddCardCheat) {
	        recordDiagnosticEvent("apply_action:failed", { ...applyPhaseReport(), error: failureReason });
	        console.error("[ironsmith] apply_action:failed", {
	          seq: nextSequence,
	          actor: Number(message?.actorIndex ?? -1),
	          phase: applyPhase,
	          command: summarizePeerCommand(message?.command),
	          error: failureReason,
	        });
	      }
      try {
        await restoreValidationSnapshot();
      } catch {
        // Preserve the validation error as the actionable failure.
      }
      if (!snapshotRestored) {
        updateMultiplayer((prev) => ({
          ...prev,
          submittingAction: false,
        }));
      }
      if (!dryRun) await servicesRef.current.failOptimisticAction?.(nextSequence, failureReason);
      if (isMatchDisputed(multiplayerRef.current)) {
        if (dryRun || throwOnFailure) throw err;
        setStatus(multiplayerRef.current.matchDisputed?.reason || failureReason, true);
        return { disputed: true };
      }
      if (dryRun) {
        throw err;
      }
      const protocolTimeoutClaim = protocolResponseTimeoutClaimFromError(err);
      if (protocolTimeoutClaim) {
        if (throwOnFailure) {
          throw err;
        }
        await submitProtocolResponseTimeoutClaim(protocolTimeoutClaim);
        return;
      }
	      if (unauthorizedAddCardCheat) {
	        if (throwOnFailure) {
	          throw err;
	        }
	        const actorName = playerNameForIndex(multiplayerRef.current.players, message.actorIndex);
	        const status = isTrustedMultiplayerSecurityMode(message?.securityMode)
	          ? `Rejected add-card cheat from ${actorName}: ${failureReason}`
	          : `Rejected signed add-card cheat from ${actorName}: ${failureReason}`;
	        emitSyncFailureNotice("Cheat detected", status);
	        setStatus(status, true);
	        return;
	      }
	      if (rejectedActionCheat) {
	        if (throwOnFailure) {
	          throw err;
	        }
	        // An obligation violation in a post opening is the cheat of the
	        // seat that supplied the opening, not necessarily the actor's.
	        const actorName = playerNameForIndex(
	          multiplayerRef.current.players,
	          cheatOffenderForError(err, failureReason, message.actorIndex)
	        );
	        if (isTrustedMultiplayerSecurityMode(sequencedActionSecurityMode(message, multiplayerRef.current))) {
	          const status = `Trusted action rejected from ${actorName}: ${failureReason}`;
	          const resynced = reportSyncFailure(
	            status,
	            options.failureResyncReason || "Trusted action mismatch. Resyncing with host...",
	            status
	          );
	          if (!resynced) {
	            setStatus(status, true);
	          }
	          return { rejected: true, reason: failureReason };
	        }
	        const status = `Cheat detected from ${actorName}: ${failureReason}`;
	        emitSyncFailureNotice("Cheat detected", status);
	        setStatus(status, true);
	        return;
	      }
      if (throwOnFailure) {
        throw err;
      }
      const resynced = reportSyncFailure(
        failureReason,
        options.failureResyncReason || "Failed to apply synced action. Resyncing with host..."
      );
      if (!resynced) {
        throw err;
      }
    } finally {
      await releaseValidationSnapshot();
    }
  }

  // Forced reveals of a determined set ("reveal the cards you discard",
  // "discard your hand", "reveal the first card you draw") and every other
  // selection made under a public reveal policy: the actor must open each
  // selected card it owns before its answer is replayed. An answer that names
  // a card without the opening would let the owner move or discard a card
  // every other peer still holds as a placeholder, so it is a detected cheat.
  // The engine already requires the full set (min == max, no repeats).
  async function assertPublicSelectionsOpened(command, uiState, actorIndex) {
    if (command?.type !== "select_objects" || !Array.isArray(command.object_ids)) return;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.hiddenCardOpenState !== "function") return;
    const decision = uiState?.decision || null;
    for (const objectId of command.object_ids) {
      const candidate = selectObjectCandidateForId(decision, objectId);
      if (selectObjectCandidateRevealPolicy(decision, candidate) !== "public") continue;
      const numeric = Number(objectId);
      if (!Number.isSafeInteger(numeric) || numeric <= 0) continue;
      let openState = null;
      try {
        openState = await currentGame.hiddenCardOpenState(wasmObjectIdArg(numeric));
      } catch {
        openState = null;
      }
      if (!openState?.tracked || openState.open) continue;
      if (Number(openState.owner) !== Number(actorIndex)) continue;
      throw new Error(
        `Forced public reveal omitted an opening: player ${Number(actorIndex) + 1} `
        + `selected hidden object ${numeric} without revealing it`
      );
    }
  }

  async function buildLocalZiffleShuffleStep(request) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleBuildShuffleStep !== "function") {
      throw new Error("Ziffle shuffle backend is not available");
    }
    const step = await currentGame.ziffleBuildShuffleStep({
      ...cloneMultiplayerPayload(request),
      entropyHex: randomAuditHex(32),
    });
    // Ziffle proofs need no secret, so the step is attributable only through
    // this signature by our genesis audit key over the exact ceremony, input,
    // and output (lib/ziffle-step-signatures.js).
    const { keyPair } = await ensureAuditIdentity();
    const signature = await signZiffleShuffleStep(
      keyPair,
      request,
      Array.isArray(request?.steps) ? request.steps : [],
      step
    );
    return { ...step, signature };
  }

  // Seat -> audit key, from the signed genesis roster only.
  function genesisAuditKeyForSeat(payload = matchStartPayloadRef.current) {
    return genesisAuditKeyLookup(payload?.players || []);
  }

  async function assertZiffleCeremonyStepsSigned(ceremony, payload = matchStartPayloadRef.current) {
    await verifyZiffleCeremonyStepSignatures(ceremony, genesisAuditKeyForSeat(payload));
  }

  async function answerZiffleShuffleStepRequest(conn, message) {
    // Announce the (first) response so peers can close their wait on this seat.
    conn = servicesRef.current.protocolResponseConn?.(conn, message) || conn;
    try {
      const step = await buildLocalZiffleShuffleStep(message.request || {});
      safeSend(conn, {
        type: "ziffle_shuffle_step_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        step,
      });
    } catch (err) {
      safeSend(conn, {
        type: "ziffle_shuffle_step_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  function recordZiffleShufflePerf(entry) {
    const normalized = {
      at: Date.now(),
      ...entry,
    };
    ziffleShufflePerfRef.current = [
      ...ziffleShufflePerfRef.current,
      normalized,
    ].slice(-32);
    console.debug?.("Ironsmith ziffle shuffle perf", normalized);
  }

  function normalizedZiffleShuffleCeremony(input, index, keys) {
    const deckCount = Number(input?.deckCount);
    if (!Number.isSafeInteger(deckCount) || deckCount <= 1) return null;
    if (!isSupportedZiffleDeckCount(deckCount)) {
      throw new Error(`Unsupported in-game ziffle library size ${deckCount}`);
    }
    const owner = Number(input?.owner);
    const zone = String(input?.zone || "library");
    const context = String(input?.context || "");
    const keyContext = String(input?.keyContext || context);
    return {
      id: String(input?.id || input?.requirementId || `ziffle:${owner}:${zone}:${index}`),
      requirement: input?.requirement || null,
      requirementId: String(input?.requirementId || input?.id || ""),
      owner,
      zone,
      deckCount,
      context,
      keyContext,
      manifest: input?.manifest || null,
      keys: cloneMultiplayerPayload(input?.keys || keys || []),
      ...ziffleInputDeckFields(input),
      ...(!isPrivateZiffleEpoch(input) ? {
        beforeOrder: normalizeShuffleOrder(input?.beforeOrder ?? input?.before_order),
        afterOrder: normalizeShuffleOrder(input?.afterOrder ?? input?.after_order),
      } : {}),
      steps: [],
      timings: {
        stepMs: [],
      },
    };
  }

  function ziffleShuffleRequestForCeremony(ceremony, shuffler) {
    return {
      deckCount: Number(ceremony.deckCount),
      context: String(ceremony.context || ""),
      keyContext: String(ceremony.keyContext || ceremony.context || ""),
      keys: cloneMultiplayerPayload(ceremony.keys || []),
      steps: cloneMultiplayerPayload(ceremony.steps || []),
      ...ziffleInputDeckFields(ceremony),
      owner: Number(ceremony.owner),
      zone: String(ceremony.zone || "library"),
      shuffler: Number(shuffler.index || 0),
    };
  }

  function appendZiffleShuffleStep(ceremony, step) {
    ceremony.steps.push({
      shuffler: Number(step.shuffler),
      deckHex: String(step.deckHex || ""),
      proofHex: String(step.proofHex || ""),
      signature: String(step.signature || ""),
    });
  }

  async function runBatchedZiffleShuffleCeremonies(rawCeremonies, players, options = {}) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
      throw new Error("Ziffle mental-poker backend is not available");
    }
    const orderedPlayers = reindexPlayers(players);
    const keys = cloneMultiplayerPayload(options.keys || zifflePublicKeysForPlayers(orderedPlayers));
    const ceremonies = (rawCeremonies || [])
      .map((ceremony, index) => normalizedZiffleShuffleCeremony(ceremony, index, keys))
      .filter(Boolean);
    if (ceremonies.length === 0) return [];

    const session = multiplayerRef.current;
    const localIndex = resolveLocalPlayerIndex(session);
    const localPeerId = String(session.localPeerId || "");
    const startedAt = nowMonotonicMs();
    const rounds = [];

    for (const shuffler of orderedPlayers) {
      const shufflerIndex = Number(shuffler.index || 0);
      const isLocalShuffler =
        shufflerIndex === Number(localIndex)
        || (localPeerId && String(shuffler.peerId || "") === localPeerId);
      const roundStartedAt = nowMonotonicMs();
      let results;
      if (isLocalShuffler) {
        results = await Promise.all(ceremonies.map(async (ceremony) => {
          const stepStartedAt = nowMonotonicMs();
          const step = await buildLocalZiffleShuffleStep(
            ziffleShuffleRequestForCeremony(ceremony, shuffler)
          );
          ceremony.timings.stepMs.push(nowMonotonicMs() - stepStartedAt);
          return { ceremony, step };
        }));
      } else {
        const routePeerId = routePeerIdForPlayer(shuffler);
        if (!routePeerId) {
          throw new Error(`Missing peer for ziffle shuffle player ${shufflerIndex + 1}`);
        }
        const label = shuffler.name || `Player ${shufflerIndex + 1}`;
        setStatus(
          `Waiting for cryptographic shuffle material from ${label} `
          + `(${ceremonies.length} shuffle${ceremonies.length === 1 ? "" : "s"})`
        );
        results = await Promise.all(ceremonies.map(async (ceremony) => {
          const requestId = makeZiffleRequestId("ziffle-live-shuffle");
          const requestedAtMs = Date.now();
          const waiter = waitForZiffleShuffleStep(requestId, PROTOCOL_RESPONSE_TIMEOUT_MS, {
            peerIndex: shufflerIndex,
            peerName: label,
            description:
              `${label} must provide verifiable shuffle material for ${ceremonies.length} `
              + `shuffle${ceremonies.length === 1 ? "" : "s"} before the game can advance.`,
          });
          const stepStartedAt = nowMonotonicMs();
          const requestPayload = {
            type: "ziffle_shuffle_step_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            request: ziffleShuffleRequestForCeremony(ceremony, shuffler),
          };
          await sendDirectProtocolMessage(routePeerId, requestPayload);
          const step = await waitForProtocolResponse(waiter, {
            basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
            targetPlayerIndex: shufflerIndex,
            targetPeerId: shuffler.peerId,
            requesterIndex: localIndex,
            requestType: requestPayload.type,
            requestId,
            requestPayload,
            responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
            requestedAtMs,
          });
          ceremony.timings.stepMs.push(nowMonotonicMs() - stepStartedAt);
          return { ceremony, step };
        }));
      }
      for (const { ceremony, step } of results) {
        appendZiffleShuffleStep(ceremony, step);
      }
      rounds.push({
        shuffler: shufflerIndex,
        local: Boolean(isLocalShuffler),
        ceremonyCount: ceremonies.length,
        ms: Math.round(nowMonotonicMs() - roundStartedAt),
      });
    }

    const verifyStartedAt = nowMonotonicMs();
    const verifiedCeremonies = await Promise.all(ceremonies.map(async (ceremony) => {
      const verified = await currentGame.ziffleVerifyShuffle({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: String(ceremony.keyContext || ceremony.context || ""),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        ...ziffleInputDeckFields(ceremony),
      });
      return { ceremony, verified };
    }));
    const verifyMs = nowMonotonicMs() - verifyStartedAt;
    await Promise.all(ceremonies.map((ceremony) => assertZiffleCeremonyStepsSigned(
      ceremony,
      options.payload || matchStartPayloadRef.current
    )));
    for (const { ceremony, verified } of verifiedCeremonies) {
      ceremony.verification = verified;
      ceremony.deckHash = String(verified.deckHash || "");
      ceremony.deckHex = String(verified.deckHex || "");
    }

    recordZiffleShufflePerf({
      kind: String(options.kind || "shuffle"),
      ceremonyCount: ceremonies.length,
      playerCount: orderedPlayers.length,
      deckCounts: ceremonies.map((ceremony) => ceremony.deckCount),
      totalMs: Math.round(nowMonotonicMs() - startedAt),
      verifyMs: Math.round(verifyMs),
      rounds,
    });

    return ceremonies;
  }

  async function buildLocalZiffleRevealToken(ceremony, cardPosition) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleBuildRevealToken !== "function") {
      throw new Error("Ziffle reveal-token backend is not available");
    }
    const localIndex = resolveLocalCryptoPlayerIndex();
    const keyContext = ziffleKeyContextForCeremony(ceremony);
    const keyPair = await ensureZiffleIdentity({
      context: keyContext,
      deckCount: ceremony.deckCount,
    });
    return currentGame.ziffleBuildRevealToken({
      deckCount: Number(ceremony.deckCount),
      deckHash: String(ceremony.deckHash || ""),
      context: String(ceremony.context || ""),
      keyContext,
      keys: cloneMultiplayerPayload(ceremony.keys || []),
      steps: cloneMultiplayerPayload(ceremony.steps || []),
      ...ziffleInputDeckFields(ceremony),
      cardPosition: Number(cardPosition),
      publicKeyHex: String(keyPair.publicKeyHex || ""),
      secretKeyHex: String(keyPair.secretKeyHex || ""),
      entropyHex: randomAuditHex(32),
      player: Number(localIndex || 0),
    });
  }

  function zifflePositionsDetail(positions = []) {
    const normalized = (positions || []).map(Number).filter((position) => Number.isFinite(position));
    if (normalized.length === 0) return "";
    const sample = normalized.slice(0, 5).join(", ");
    return normalized.length > 5
      ? `Positions ${sample}, +${normalized.length - 5} more`
      : `Positions ${sample}`;
  }

  async function buildLocalZiffleRevealTokens(ceremony, cardPositions) {
    const positions = normalizeZiffleCardPositions(cardPositions, ceremony, {
      allowEmpty: true,
      label: "Local ziffle reveal-token build",
    });
    if (positions.length === 0) return [];
    const currentGame = gameRef.current;
    if (!currentGame) {
      throw new Error("Ziffle reveal-token backend is not available");
    }
    const waitId = beginPeerWait({
      kind: "local_ziffle_reveal",
      local: true,
      peerIndex: resolveLocalCryptoPlayerIndex(),
      peerName: "You",
      title: "Generating reveal proof",
      description:
        `Your browser is deriving cryptographic opening material for ${positions.length} hidden `
        + `card${positions.length === 1 ? "" : "s"}.`,
      operation: "Generating reveal proof",
      detail: zifflePositionsDetail(positions),
      progressCurrent: 0,
      progressTotal: positions.length,
      responseTimeoutMs: ziffleRevealTokenTimeoutMs(positions.length, ceremony),
    });
    try {
      if (typeof currentGame.ziffleBuildRevealTokens !== "function") {
        const tokens = [];
        for (const [index, position] of positions.entries()) {
          updatePeerWait(waitId, {
            detail: `Position ${Number(position)}`,
            progressCurrent: index,
            progressTotal: positions.length,
          });
          const token = await buildLocalZiffleRevealToken(ceremony, position);
          tokens.push({
            ...token,
            cardPosition: positions[index],
          });
          updatePeerWait(waitId, {
            progressCurrent: index + 1,
            progressTotal: positions.length,
          });
        }
        return tokens;
      }
      const localIndex = resolveLocalCryptoPlayerIndex();
      const keyContext = ziffleKeyContextForCeremony(ceremony);
      const keyPair = await ensureZiffleIdentity({
        context: keyContext,
        deckCount: ceremony.deckCount,
      });
      updatePeerWait(waitId, {
        operation: "Generating batched reveal proof",
        progressCurrent: 0,
        progressTotal: positions.length,
      });
      return await currentGame.ziffleBuildRevealTokens({
        deckCount: Number(ceremony.deckCount),
        deckHash: String(ceremony.deckHash || ""),
        context: String(ceremony.context || ""),
        keyContext,
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        ...ziffleInputDeckFields(ceremony),
        cardPositions: positions,
        publicKeyHex: String(keyPair.publicKeyHex || ""),
        secretKeyHex: String(keyPair.secretKeyHex || ""),
        entropyHex: randomAuditHex(32),
        player: Number(localIndex || 0),
      });
    } finally {
      clearPeerWait(waitId);
    }
  }

  async function waitForZiffleCeremony(owner, options = {}, timeoutMs = 10000) {
    const started = Date.now();
    while (Date.now() - started < timeoutMs) {
      const ceremony = ziffleCeremonyForOwner(owner, options);
      if (ceremony) return ceremony;
      await sleep(50);
    }
    return null;
  }

  // Positions of `owner`'s hidden cards that `requester` may reopen from the
  // visible state alone. The owner may reopen what it may look at; any other
  // requester only cards outside the library the rules let it see (a card it
  // exiled face down from the owner's library with Gonti, Praetor's Grasp...).
  async function authorizedZiffleRevealPositionsForOwner(owner, deckHash, requester = owner) {
    const requesterIsOwner = Number(requester) === Number(owner);
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.exportSyncCheckpoint !== "function") {
      return new Set();
    }
    const checkpoint = await currentGame.exportSyncCheckpoint();
    const positions = new Set();
    const expectedDeckHash = String(deckHash || "");
    const addMetadataPosition = (metadata) => {
      if (!metadata || Number(metadata.owner) !== Number(owner)) return;
      const publicCommitment = String(
        metadata.publicCommitment
        || metadata.public_commitment
        || ""
      );
      const hiddenCommitment = String(metadata.commitment || "");
      const publicSlot = metadata.publicSlot ?? metadata.public_slot ?? null;
      const addCommittedPosition = (position, commitment) => {
        const commitmentDeckHash = ziffleDeckHashFromCommitment(commitment);
        if (!commitmentDeckHash) return;
        if (expectedDeckHash && commitmentDeckHash !== expectedDeckHash) return;
        const committedPosition = position ?? zifflePositionFromCommitment(commitment);
        const normalizedPosition = Number(committedPosition);
        if (Number.isSafeInteger(normalizedPosition) && normalizedPosition >= 0) {
          positions.add(normalizedPosition);
        }
      };
      addCommittedPosition(publicSlot, publicCommitment);
      addCommittedPosition(metadata.slot, hiddenCommitment);
      const origin = ziffleOriginAnchorFromMetadata(metadata);
      if (origin) addCommittedPosition(origin.originPosition, origin.originPositionCommitment);
    };
    const objectsById = new Map((checkpoint?.objects || []).map((object) => [
      Number(object?.id),
      object,
    ]));
    // Ownership is not permission to look: an owner may not see its own card
    // an opponent exiled face down (Gonti, Praetor's Grasp) or a face-down
    // permanent it owns but does not control. Ask the engine's view rules;
    // without that export, fail closed for these zones.
    const requesterMayView = async (objectId) => {
      if (typeof currentGame.hiddenObjectViewableBy !== "function") return false;
      const numeric = Number(objectId);
      if (!Number.isSafeInteger(numeric) || numeric <= 0) return false;
      try {
        return Boolean(await currentGame.hiddenObjectViewableBy(wasmObjectIdArg(numeric), Number(requester)));
      } catch {
        return false;
      }
    };
    const collectZoneObjectIds = (player, zoneKey) => {
      for (const objectId of player?.[zoneKey] || []) {
        const object = objectsById.get(Number(objectId));
        const hidden = object?.hiddenCard || object?.hidden_card || null;
        if (!hidden) continue;
        addMetadataPosition({
          objectId: Number(objectId),
          owner: hidden.owner,
          slot: hidden.slot,
          commitment: hidden.commitment,
          originSlot: hidden.originSlot ?? hidden.origin_slot,
          originCommitment: hidden.originCommitment ?? hidden.origin_commitment,
          publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
          publicCommitment: hidden.publicCommitment ?? hidden.public_commitment ?? "",
        });
      }
    };
    for (const player of checkpoint?.players || []) {
      if (!requesterIsOwner) break;
      if (Number(player?.id ?? player?.index) !== Number(owner)) continue;
      collectZoneObjectIds(player, "hand");
      collectZoneObjectIds(player, "graveyard");
      collectZoneObjectIds(player, "commanders");
      collectZoneObjectIds(player, "sideboard");
    }
    for (const objectId of checkpoint?.exile || []) {
      const object = objectsById.get(Number(objectId));
      const hidden = object?.hiddenCard || object?.hidden_card || null;
      if (!hidden || Number(hidden.owner) !== Number(owner)) continue;
      if (!await requesterMayView(objectId)) continue;
      addMetadataPosition({
        objectId: Number(objectId),
        owner: hidden.owner,
        slot: hidden.slot,
        commitment: hidden.commitment,
        originSlot: hidden.originSlot ?? hidden.origin_slot,
        originCommitment: hidden.originCommitment ?? hidden.origin_commitment,
        publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
        publicCommitment: hidden.publicCommitment ?? hidden.public_commitment ?? "",
      });
    }
    // End-of-match disclosure: once the owner's part of the match is over on
    // this engine it must open every card its disclosure lists, including the
    // library anchors (the durable ziffle ciphertexts of claimed cards that
    // entered its library, see game_state/hidden_hand_choices.rs). The list
    // comes from this engine's own state and the game is over for that seat,
    // so revealing those positions is harmless.
    if (
      requesterIsOwner
      && disclosureDueForPlayer(stateRef.current, owner)
      && typeof currentGame.endOfMatchDisclosureRequirements === "function"
    ) {
      try {
        const requirements = await currentGame.endOfMatchDisclosureRequirements(Number(owner));
        for (const requirement of Array.isArray(requirements) ? requirements : []) {
          addMetadataPosition(requirement);
        }
      } catch {
        // No disclosure positions: the visible-state rules below still apply.
      }
    }
    const blockedZones = new Set(requesterIsOwner
      ? ["library", "outside_game"]
      : ["library", "outside_game", "hand"]);
    for (const object of checkpoint?.objects || []) {
      const hidden = object?.hiddenCard || object?.hidden_card || null;
      if (!hidden || Number(hidden.owner) !== Number(owner)) continue;
      const zone = String(object?.zone || hidden.zone || "");
      if (blockedZones.has(zone)) continue;
      if (!await requesterMayView(object?.id)) continue;
      addMetadataPosition({
        objectId: Number(object?.id),
        owner: hidden.owner,
        slot: hidden.slot,
        commitment: hidden.commitment,
        originSlot: hidden.originSlot ?? hidden.origin_slot,
        originCommitment: hidden.originCommitment ?? hidden.origin_commitment,
        publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
        publicCommitment: hidden.publicCommitment ?? hidden.public_commitment ?? "",
      });
    }
    return positions;
  }

  async function waitForAuthorizedZiffleRevealPositions(
    owner, deckHash, requestedPositions, timeoutMs = 10000, requester = owner
  ) {
    const needed = new Set((requestedPositions || []).map((position) => Number(position)));
    const started = Date.now();
    let allowed = await authorizedZiffleRevealPositionsForOwner(owner, deckHash, requester);
    while ([...needed].some((position) => !allowed.has(position)) && Date.now() - started < timeoutMs) {
      await sleep(50);
      allowed = await authorizedZiffleRevealPositionsForOwner(owner, deckHash, requester);
    }
    return allowed;
  }

  async function ziffleRevealAuthorizedByOutboundCryptoRequest(message, requester, owner, positions, ceremony) {
    const requestId = String(message?.cryptoMaterialRequestId || "");
    if (!requestId) return false;
    const pending = outboundCryptoMaterialRequestsRef.current.get(requestId);
    if (!pending) return false;
    if (String(pending.peerId || "") !== String(message?.requesterPeerId || "")) return false;
    if (!pending.actionIntent) return false;
    if (Number(pending.targetSeat ?? pending.owner) !== Number(requester)) return false;
    // A pending request authorizes its exact openings in this ceremony, not
    // the same numeric positions in another shuffle of the owner's deck.
    return ziffleRequirementsAuthorizeRevealPositions(
      pending.requirements, requester, owner, positions, ceremony, pending.seq
    ) || await ziffleRequirementsAuthorizeRevealPositionsByMetadata(
      pending.requirements, requester, owner, positions, ceremony
    );
  }

  function ziffleRequirementType(requirement) {
    return String(requirement?.type || requirement?.requirement_type || "");
  }

  function ziffleRequirementZone(requirement) {
    return String(requirement?.zone || "");
  }

  function ziffleRequirementViewer(requirement) {
    if (requirement?.viewer === null || requirement?.viewer === undefined) return null;
    const viewer = Number(requirement.viewer);
    return Number.isInteger(viewer) ? viewer : null;
  }

  function zifflePositionFromRequirement(requirement) {
    const publicPosition = zifflePublicPositionFromSources(requirement);
    if (publicPosition && publicPosition.useAsPosition !== false) return publicPosition.position;
    const committedPosition =
      zifflePositionFromCommitment(requirement?.commitment)
      ?? zifflePositionFromCommitment(requirement?.positionCommitment)
      ?? zifflePositionFromCommitment(requirement?.position_commitment);
    if (committedPosition != null) return committedPosition;
    const explicitPosition = Number(requirement?.position);
    if (Number.isSafeInteger(explicitPosition) && explicitPosition >= 0) return explicitPosition;
    const publicSlot = Number(requirement?.publicSlot ?? requirement?.public_slot);
    if (Number.isSafeInteger(publicSlot) && publicSlot >= 0) return publicSlot;
    const slot = Number(requirement?.slot);
    return Number.isSafeInteger(slot) && slot >= 0 ? slot : null;
  }

  function ziffleRevealPositionFromRequirement(requirement) {
    const publicPosition = zifflePublicPositionFromSources(requirement);
    if (publicPosition && publicPosition.useAsPosition !== false) {
      return publicPosition.position;
    }
    const committedPosition =
      zifflePositionFromCommitment(requirement?.commitment)
      ?? zifflePositionFromCommitment(requirement?.positionCommitment)
      ?? zifflePositionFromCommitment(requirement?.position_commitment);
    if (committedPosition != null) return committedPosition;
    const zone = ziffleRequirementZone(requirement).toLowerCase();
    if (zone && zone !== "library") return null;
    const explicitPosition = Number(requirement?.position);
    if (Number.isSafeInteger(explicitPosition) && explicitPosition >= 0) return explicitPosition;
    const publicSlot = Number(requirement?.publicSlot ?? requirement?.public_slot);
    return Number.isSafeInteger(publicSlot) && publicSlot >= 0 ? publicSlot : null;
  }

  function ziffleRequirementPositionCommitments(requirement) {
    return [
      requirement?.publicCommitment, requirement?.public_commitment,
      requirement?.positionCommitment, requirement?.position_commitment,
      requirement?.commitment,
    ].map(value => String(value || "")).filter(value => ziffleDeckHashFromCommitment(value));
  }

  function ziffleRequirementsAuthorizeRevealPositions(requirements, requester, owner, positions, ceremony, sequence = null) {
    const requested = (positions || []).map(Number);
    const deckCount = Number(ceremony?.deckCount);
    const deckHash = String(ceremony?.deckHash || "");
    if (!deckHash || !Number.isSafeInteger(deckCount) || deckCount <= 0
      || requested.length === 0
      || requested.some(position => !Number.isSafeInteger(position) || position < 0 || position >= deckCount)) return false;
    const exactPositions = new Set();
    for (const requirement of requirements || []) {
      if (Number(requirement?.owner) !== Number(owner)) continue;
      const type = ziffleRequirementType(requirement);
      if (type === "private_open" && ziffleRequirementViewer(requirement) !== Number(requester)) continue;
      if (type === "public_open" || type === "private_open") {
        // These requirements are produced by our engine (or retained from its
        // finalized preview), so their immutable anchors identify exact cards.
        const origin = ziffleOriginAnchorFromMetadata(requirement);
        if (origin && ziffleDeckHashFromCommitment(origin.originPositionCommitment) === deckHash) {
          exactPositions.add(origin.originPosition);
        }
        for (const commitment of ziffleRequirementPositionCommitments(requirement)) {
          if (ziffleDeckHashFromCommitment(commitment) !== deckHash) continue;
          const position = zifflePositionFromCommitment(commitment);
          if (position != null) exactPositions.add(position);
        }
      } else if (type === "verifiable_shuffle" && ziffleRequirementZone(requirement) === "library"
        && Number(requester) === Number(owner)) {
        const before = normalizeShuffleOrder(requirement.beforeOrder ?? requirement.before_order);
        const after = normalizeShuffleOrder(requirement.afterOrder ?? requirement.after_order);
        const prefix = Number(requirement.count);
        const context = [currentAuditMatchId(), "action", Number(sequence), "shuffle",
          String(requirement.id || ""), Number(owner), "library"].join(":");
        if (Number.isSafeInteger(Number(sequence)) && Number(sequence) > 0
          && String(ceremony.context || "") === context
          && before.length === deckCount && after.length === deckCount
          && sameShuffleOrder(before, normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order))
          && sameShuffleOrder(after, normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order))
          && Number.isSafeInteger(prefix) && prefix >= 0 && prefix <= deckCount) {
          // The engine puts cards drawn by a shuffle (e.g. a mulligan) after
          // its remaining-library prefix. Only the owner may reopen that tail.
          for (let position = prefix; position < deckCount; position++) exactPositions.add(position);
        }
      }
      // A view-window count never identifies cards. The engine also emits
      // exact opening requirements for the cards in that window.
    }
    return requested.every(position => exactPositions.has(position));
  }

  async function ziffleRequirementsAuthorizeRevealPositionsByMetadata(
    requirements, requester, owner, positions, ceremony, allowUnpinned = false
  ) {
    const requested = (positions || []).map(Number);
    const deckCount = Number(ceremony?.deckCount);
    const deckHash = String(ceremony?.deckHash || "");
    if (!deckHash || !Number.isSafeInteger(deckCount) || deckCount <= 0
      || requested.length === 0
      || requested.some(position => !Number.isSafeInteger(position) || position < 0 || position >= deckCount)) return false;
    const currentGame = gameRef.current;
    let checkpoint = null;
    const allowed = new Set();
    for (const requirement of requirements || []) {
      if (Number(requirement?.owner) !== Number(owner)) continue;
      const type = ziffleRequirementType(requirement);
      if (type !== "public_open" && type !== "private_open") continue;
      if (type === "private_open" && ziffleRequirementViewer(requirement) !== Number(requester)) continue;
      const commitments = ziffleRequirementPositionCommitments(requirement);
      // Historical openings must retain their original ceremony binding; a
      // later shuffle must not turn an old view into knowledge of new order.
      const currentCeremonyBound = commitments.some(value => ziffleDeckHashFromCommitment(value) === deckHash)
        || (allowUnpinned && commitments.length === 0);
      const requiredCommitments = [requirement.commitment, ...commitments].filter(Boolean).map(String);
      const matchesRequirement = metadata => Boolean(metadata)
        && Number(metadata.owner) === Number(owner)
        && requiredCommitments.some(value => value === String(metadata.commitment || "")
          || value === String(metadata.publicCommitment || ""));
      let objectId = requirement.objectId ?? requirement.object_id;
      let metadata = objectId == null ? null : await currentHiddenCardMetadataForObject(Number(objectId));
      if (!matchesRequirement(metadata) && typeof currentGame?.exportSyncCheckpoint === "function") {
        checkpoint ||= await currentGame.exportSyncCheckpoint();
        // Runtime IDs can differ between peers. Resolve the locally trusted
        // requirement's committed identity before consulting its position.
        objectId = hiddenObjectIdForOpeningFromCheckpoint(checkpoint, {
          owner, slot: requirement.slot, commitment: requirement.commitment,
          positionCommitment: commitments.find(value => value !== String(requirement.commitment || ""))
            || commitments[0] || "",
        });
        metadata = objectId == null ? null : hiddenCardMetadataForObjectFromCheckpoint(checkpoint, objectId);
      }
      if (!matchesRequirement(metadata)) continue;
      if (currentCeremonyBound) {
        for (const commitment of [metadata.publicCommitment, metadata.commitment]) {
          if (ziffleDeckHashFromCommitment(commitment) !== deckHash) continue;
          const position = zifflePositionFromCommitment(commitment);
          if (position != null) allowed.add(position);
        }
      }
      // An exact authorized card may reopen its immutable genesis anchor.
      // The anchor comes only from this engine, never attached requirements.
      const origin = ziffleOriginAnchorFromMetadata(metadata);
      if (origin && ziffleDeckHashFromCommitment(origin.originPositionCommitment) === deckHash) {
        allowed.add(origin.originPosition);
      }
    }
    return requested.every(position => allowed.has(position));
  }

  function compactCryptoRequirementForDiagnostics(requirement) {
    if (!requirement || typeof requirement !== "object") return null;
    const beforeOrder = normalizeShuffleOrder(requirement.beforeOrder ?? requirement.before_order);
    const afterOrder = normalizeShuffleOrder(requirement.afterOrder ?? requirement.after_order);
    return {
      type: ziffleRequirementType(requirement),
      owner: Number(requirement.owner),
      viewer: ziffleRequirementViewer(requirement),
      zone: ziffleRequirementZone(requirement),
      count: requirement.count == null ? null : Number(requirement.count),
      slot: requirement.slot == null ? null : Number(requirement.slot),
      position: zifflePositionFromRequirement(requirement),
      beforeOrderLength: beforeOrder.length,
      afterOrderLength: afterOrder.length,
      afterOrderFirst: afterOrder.length > 0 ? afterOrder[0] : null,
      afterOrderLast: afterOrder.length > 0 ? afterOrder[afterOrder.length - 1] : null,
    };
  }

  function compactActionAuthorizationForDiagnostics(auth) {
    if (!auth || typeof auth !== "object") return null;
    return {
      matchId: String(auth.matchId || ""),
      seq: Number(auth.seq || 0),
      requesterIndex: auth.requesterIndex == null ? null : Number(auth.requesterIndex),
      actorIndex: auth.actorIndex == null ? null : Number(auth.actorIndex),
      commandType: String(auth.command?.type || ""),
      actionKind: String(auth.command?.action_ref?.kind || ""),
      requirements: (Array.isArray(auth.requirements) ? auth.requirements : [])
        .map(compactCryptoRequirementForDiagnostics)
        .filter(Boolean),
    };
  }

  function lockVerifiedZiffleActionShuffles(proofs) {
    const matchId = String(currentAuditMatchId());
    const prefix = `${matchId}:action:`;
    const candidates = proofs.filter(proof => String(proof.context || "").startsWith(prefix));
    const locks = ziffleActionRevealLocksRef.current;
    // No await between checking and recording the complete set: overlapping
    // requests cannot authorize different hypothetical hands for one action.
    for (const proof of candidates) {
      const existing = locks.get(proof.context);
      if (existing && existing.deckHash !== String(proof.deckHash)) {
        throw new Error("Action shuffle conflicts with previously authorized reveal material");
      }
    }
    for (const proof of candidates) {
      locks.set(proof.context, { matchId, seq: Number(proof.context.slice(prefix.length).split(":")[0]),
        deckHash: String(proof.deckHash) });
    }
    const oldest = Number(multiplayerRef.current.lastAppliedSequence || 0) - 64;
    for (const [key, lock] of locks) {
      if (lock.matchId !== matchId || lock.seq < oldest) locks.delete(key);
    }
  }

  async function previewZiffleActionRequirements(auth, initialRequirements, options = {}) {
    const shuffles = initialRequirements.filter(requirement => ziffleRequirementType(requirement) === "verifiable_shuffle");
    const proofs = auth.shuffleProofs || auth.actionAudit?.shuffleProofs || [];
    const rngReveals = auth.rngReveals || auth.actionAudit?.rngReveals || [];
    const hasFairRandom = initialRequirements.some(requirement => ziffleRequirementType(requirement) === "fair_random");
    if (shuffles.length === 0 && proofs.length === 0 && !hasFairRandom && rngReveals.length === 0) return initialRequirements;
    const currentGame = gameRef.current;
    if (!Array.isArray(proofs)
      || typeof currentGame?.previewCryptoRequirementsWithMaterial !== "function") {
      throw new Error("Shuffle-dependent reveals require verified action shuffle material");
    }
    const material = { seeds: [], libraryShuffles: [], libraryEpochs: [], libraryEpochOpenings: [] };
    const preceding = [];
    let requirements = initialRequirements;
    const processed = new Set();
    const processedRandom = new Set();
    const previewWithAgreedRandomness = async () => {
      // A committed random result can expose a shuffle or another random event.
      // Authenticate each locally required result before it enters the clone;
      // attached requirements and caller-provided seed strings are not authority.
      for (let attempt = 0; attempt < 256; attempt++) {
        const freshRandom = requirements.filter(entry => ziffleRequirementType(entry) === "fair_random"
          && !processedRandom.has(String(entry.id)));
        if (freshRandom.length) {
          await verifyAuditSatisfiesCryptoRequirements({ requirements: freshRandom,
            audit: { seq: Number(auth.seq), rngReveals } });
          for (const requirement of freshRandom) {
            const reveal = rngReveals.find(entry => String(entry?.requirementId || "") === String(requirement.id || ""));
            material.seeds.push(String(reveal.combinedSeedHex));
            processedRandom.add(String(requirement.id));
          }
        }
        requirements = await currentGame.previewCryptoRequirementsWithMaterial(auth.command, material);
        if (!requirements.some(entry => ziffleRequirementType(entry) === "fair_random"
          && !processedRandom.has(String(entry.id)))) return requirements;
      }
      throw new Error("Action random requirements did not converge");
    };
    if (hasFairRandom || rngReveals.length) await previewWithAgreedRandomness();
    for (let index = 0; index < proofs.length; index++) {
      const requirement = requirements.filter(entry => ziffleRequirementType(entry) === "verifiable_shuffle"
        && !processed.has(String(entry.id)))
        .sort((a, b) => Number(a.randomCountBefore) - Number(b.randomCountBefore))[0];
      if (!requirement) throw new Error("Shuffle disappeared after its preceding epoch");
      const expectedContext = [currentAuditMatchId(), "action", Number(auth.seq), "shuffle",
        String(requirement.id || ""), Number(requirement.owner), String(requirement.zone || "library")].join(":");
      const matching = proofs.filter(proof => String(proof?.context || "") === expectedContext
        && String(proof?.requirementId || "") === String(requirement.id || "")
        && Number(proof?.owner) === Number(requirement.owner));
      if (matching.length !== 1) throw new Error("Action shuffle does not match the locally previewed requirement");
      const proof = matching[0];
      assertZiffleShuffleProofBoundToSignedMatch(proof, requirement);
      await assertZiffleCeremonyStepsSigned(proof);
      const verified = await currentGame.ziffleVerifyShuffle({
        deckCount: Number(proof.deckCount), context: String(proof.context),
        keyContext: String(proof.keyContext || proof.context), keys: cloneMultiplayerPayload(proof.keys || []),
        steps: cloneMultiplayerPayload(proof.steps || []), ...ziffleInputDeckFields(proof),
      });
      if (!verified.deckHash || String(verified.deckHash) !== String(proof.deckHash || "")) {
        throw new Error("Action shuffle verification returned a different deck hash");
      }
      if (isPrivateZiffleEpoch(proof)) {
        const accepted = acceptedEpochsForProof(proof.owner, auth.seq, preceding);
        assertZiffleEpochVerification(proof, verified, accepted);
        material.libraryEpochs.push(ziffleEpochMaterial(proof, requirement, assertZiffleEpochInputs(proof, requirement, accepted)));
      } else {
        if (Number(matchStartPayloadRef.current?.protocolVersion) >= 15) throw new Error("Linked shuffle is not allowed in this match");
        const beforeOrder = normalizeShuffleOrder(requirement.beforeOrder ?? requirement.before_order);
        const afterOrder = normalizeShuffleOrder(requirement.afterOrder ?? requirement.after_order);
        if (!beforeOrder.length || beforeOrder.length !== afterOrder.length || Number(proof.deckCount) !== beforeOrder.length) {
          throw new Error("Action shuffle does not match the locally previewed requirement");
        }
        material.seeds.push(String(verified.deckHash));
        material.libraryShuffles.push({ owner: Number(requirement.owner), beforeOrder, afterOrder });
      }
      preceding.push(proof);
      processed.add(String(requirement.id));
      requirements = await previewWithAgreedRandomness();
      if (isPrivateZiffleEpoch(proof)) {
        rememberLocalZiffleCeremonyForLookup(proof);
        const pendingOpenings = (auth.openings || auth.actionAudit?.openings || []).filter(opening =>
          String(opening.timing || "pre") === "pre"
          && ziffleDeckHashFromCommitment(opening.positionCommitment) === String(proof.deckHash));
        while (pendingOpenings.length) {
          const requiredFor = opening => requirements.find(entry => ziffleRequirementType(entry) === "public_open"
            && Number(entry.owner) === Number(opening.owner)
            && ziffleRequirementPositionCommitments(entry).includes(String(opening.positionCommitment)));
          const next = pendingOpenings.findIndex(opening => requiredFor(opening));
          if (next < 0) throw new Error("Future epoch opening is not a locally authorized public reveal");
          const [opening] = pendingOpenings.splice(next, 1);
          const required = requiredFor(opening);
          assertZiffleOpeningOriginMatchesMetadata(opening, required);
          const manifest = servicesRef.current.publicDeckManifestForOwner(opening.owner);
          if (!await verifyCardOpeningAgainstManifest({ manifest, slot: opening.slot, card: opening.card, salt: opening.salt })) {
            throw new Error("Future epoch opening does not match its committed deck");
          }
          await servicesRef.current.verifyZiffleOpeningCryptographicProof(opening, { shuffleProofs: preceding });
          material.libraryEpochOpenings.push({ owner: Number(opening.owner), deckHash: String(proof.deckHash),
            position: Number(opening.position), originalSlot: Number(opening.slot), cardName: String(opening.card),
            commitment: String(opening.commitment || "") });
          // Revealing this identity can expose a replacement, another reveal,
          // or a shuffle. Derive authorization afresh before the next opening.
          requirements = await previewWithAgreedRandomness();
        }
      }
    }
    if (requirements.some(entry => ziffleRequirementType(entry) === "verifiable_shuffle" && !processed.has(String(entry.id)))) {
      throw new Error("Missing proof for a shuffle discovered after opening its source cards");
    }
    lockVerifiedZiffleActionShuffles(proofs);
    if (options.install) {
      if (material.seeds.length || material.libraryShuffles.length) await currentGame.injectTranscriptRandomSeeds(material);
      for (const epoch of material.libraryEpochs) await currentGame.queueVerifiedHiddenLibraryEpoch(epoch);
      for (const opening of material.libraryEpochOpenings) await currentGame.queueVerifiedHiddenLibraryOpening(opening);
    }
    return requirements;
  }

  async function waitForRevealAuthorizationSequence(sequence, debug = null, timeoutMs = 10000) {
    const target = Number(sequence);
    const started = Date.now();
    let currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    while (
      Number.isSafeInteger(target)
      && target > currentSequence + 1
      && Date.now() - started < timeoutMs
    ) {
      await sleep(50);
      currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    }
    if (debug) {
      debug.currentSequence = currentSequence;
      debug.expectedSeq = currentSequence + 1;
      debug.sequenceWaitMs = Date.now() - started;
    }
    return currentSequence;
  }

  // `disclosure`, when given, receives the in-flight intent whose previewed
  // requirements authorized the positions; the caller must lock it before
  // releasing any token.
  async function ziffleRevealAuthorizedByAction(message, requester, owner, positions, ceremony, debug = null, disclosure = null) {
    const auth = message?.actionAuthorization;
    const reject = (reason) => {
      if (debug) debug.reason = reason;
      return false;
    };
    if (!auth || typeof auth !== "object") return reject("missing_action_authorization");
    if (String(auth.matchId || "") !== String(currentAuditMatchId())) return reject("match_id_mismatch");
    if (Number(auth.requesterIndex) !== Number(requester)) return reject("requester_index_mismatch");
    // The requester may be the deck owner (its own reveals) or the viewer of a
    // non-owner private view (mental-poker flow); requirement checks below
    // only authorize private positions whose viewer is the requester.
    const sequence = Number(auth.seq);
    if (!Number.isSafeInteger(sequence) || sequence <= 0) return reject("invalid_sequence");
    if (!auth.command || typeof auth.command !== "object") return reject("missing_command");
    const currentSequence = await waitForRevealAuthorizationSequence(sequence, debug);
    const expectedSeq = currentSequence + 1;
    if (debug) {
      debug.sequence = sequence;
      debug.currentSequence = currentSequence;
      debug.expectedSeq = expectedSeq;
    }

    if (sequence <= currentSequence) {
      const applied = actionHistoryEntryForSequence(sequence);
      if (!applied) return reject("missing_applied_action");
      if (canonicalMultiplayerPayload(applied.command) !== canonicalMultiplayerPayload(auth.command)) {
        return reject("applied_command_mismatch");
      }
      if (
        auth.actorIndex !== null
        && auth.actorIndex !== undefined
        && Number(auth.actorIndex) !== Number(applied.actorIndex)
      ) {
        return reject("applied_actor_mismatch");
      }
      const storedRequirements = actionCryptoRequirementsForSequence(sequence);
      if (debug) {
        debug.storedRequirements = storedRequirements.map(compactCryptoRequirementForDiagnostics).filter(Boolean);
      }
      const authorized = ziffleRequirementsAuthorizeRevealPositions(
        actionCryptoRequirementsForSequence(sequence),
        requester,
        owner,
        positions,
        ceremony,
        sequence
      );
      if (authorized && debug) debug.reason = "authorized_by_stored_requirements";
      if (authorized) return true;
      const authorizedByMetadata = await ziffleRequirementsAuthorizeRevealPositionsByMetadata(
        storedRequirements,
        requester,
        owner,
        positions,
        ceremony
      );
      if (authorizedByMetadata && debug) debug.reason = "authorized_by_stored_requirement_metadata";
      if (authorizedByMetadata) return true;
      // The visible-state fallback derives positions from what the requester
      // may look at under the rules (see authorizedZiffleRevealPositionsForOwner).
      const authorizedByVisibleState = await waitForAuthorizedZiffleRevealPositions(
        owner,
        String(ceremony?.deckHash || ""),
        positions,
        2000,
        requester
      );
      if ([...positions].every((position) => authorizedByVisibleState.has(Number(position)))) {
        if (debug) debug.reason = "authorized_by_visible_current_hidden_zone_state";
        return true;
      }
      return reject("stored_requirements_do_not_authorize_positions");
    }

    if (sequence !== expectedSeq) return reject("unexpected_sequence");

    const liveState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
    const compatible = isDecisionCommandCompatible(liveState?.decision, auth.command);
    if (debug) {
      debug.liveDecisionKind = String(liveState?.decision?.kind || "");
      debug.liveDecisionPlayer = liveState?.decision?.player == null ? null : Number(liveState.decision.player);
      debug.commandCompatible = compatible;
    }
    if (!compatible) return reject("command_not_compatible_with_live_decision");
    if (
      auth.actorIndex !== null
      && auth.actorIndex !== undefined
      && liveState?.decision?.player !== null
      && liveState?.decision?.player !== undefined
      && Number(auth.actorIndex) !== Number(liveState.decision.player)
    ) {
      return reject("actor_index_mismatch");
    }
    if (auth.actionAudit) {
      try {
        await verifySequencedActionAudit({
          audit: auth.actionAudit,
          seq: sequence,
          actorIndex: auth.actorIndex,
          command: auth.command,
        });
      } catch (err) {
        if (debug) debug.signedActionAuditError = toErrorMessage(err);
        return reject("signed_action_audit_invalid");
      }
    } else {
      const preActionPublicCheckpointHash = await currentPublicAuditCheckpointHash();
      try {
        await verifySignedActionIntent(auth.actionIntent, {
          matchId: currentAuditMatchId(),
          seq: sequence,
          actorIndex: auth.actorIndex,
          // Bound to the local chain head, not a requester-claimed hash: an
          // intent that could never apply must not release anything, and the
          // disclosure lock below is keyed by this hash.
          prevStateHash: auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH,
          preActionPublicCheckpointHash,
          command: auth.command,
        });
      } catch (err) {
        if (debug) debug.signedIntentError = toErrorMessage(err);
        return reject("signed_action_intent_invalid");
      }
    }

    let previewedRequirements = [];
    try {
      previewedRequirements = await previewRequirementsForCommand(auth.command);
      previewedRequirements = await previewZiffleActionRequirements(auth, previewedRequirements);
    } catch (error) {
      // A concealed card may need its owner's already-visible hand opening
      // before this peer can preview the cast. Only the visible-state fallback
      // below can authorize that case; attached requirements remain untrusted.
      if (debug) debug.previewError = toErrorMessage(error);
      previewedRequirements = [];
    }
    if (debug) {
      debug.previewedRequirements = previewedRequirements
        .map(compactCryptoRequirementForDiagnostics)
        .filter(Boolean);
      debug.attachedRequirements = (Array.isArray(auth.requirements) ? auth.requirements : [])
        .map(compactCryptoRequirementForDiagnostics)
        .filter(Boolean);
    }
    // Tokens released on the strength of this not-yet-applied action pin the
    // sequence to its command (see fairRandomRevealLockKey), so the requester
    // cannot peek and then cancel or substitute a different action.
    const disclosureIntent = {
      matchId: currentAuditMatchId(),
      seq: sequence,
      actorIndex: auth.actorIndex ?? liveState?.decision?.player,
      prevStateHash: auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH,
      command: auth.command,
    };
    if (fairRandomRevealLockConflict(disclosureIntent)) {
      return reject("conflicts_with_disclosed_action_intent");
    }
    if (ziffleRequirementsAuthorizeRevealPositions(
      previewedRequirements,
      requester,
      owner,
      positions,
      ceremony,
      sequence
    )) {
      if (debug) debug.reason = "authorized_by_previewed_requirements";
      if (disclosure) disclosure.intent = disclosureIntent;
      return true;
    }
    const authorizedByMetadata = await ziffleRequirementsAuthorizeRevealPositionsByMetadata(
      previewedRequirements, requester, owner, positions, ceremony, true
    );
    if (authorizedByMetadata && debug) debug.reason = "authorized_by_requirement_metadata";
    if (authorizedByMetadata) {
      if (disclosure) disclosure.intent = disclosureIntent;
      return true;
    }

    const authorizedByVisibleState = await waitForAuthorizedZiffleRevealPositions(
      owner,
      String(ceremony?.deckHash || ""),
      positions,
      2000,
      requester
    );
    if ([...positions].every((position) => authorizedByVisibleState.has(Number(position)))) {
      if (debug) debug.reason = "authorized_by_visible_current_hidden_zone_state";
      return true;
    }
    return reject("requirements_do_not_authorize_positions");
  }

  // An attached ceremony is considered only for an in-flight action: it must be
  // byte-identical to one of that action's shuffle proofs, bound to the signed
  // match and roster, signed step-by-step by every shuffler, and verify to its
  // own claimed deck hash from a complete chain.
  async function verifiedProvisionalZiffleCeremony(attached, auth, owner) {
    if (!attached || typeof attached !== "object" || !auth || typeof auth !== "object") return null;
    const seq = Number(auth.seq);
    const deckHash = String(attached.deckHash || "");
    const context = String(attached.context || "");
    const zone = String(attached.zone || "library");
    if (!Number.isSafeInteger(seq) || seq <= 0 || !deckHash) return null;
    if (Number(attached.owner) !== Number(owner)) return null;
    if (!context.startsWith(`${currentAuditMatchId()}:action:${seq}:shuffle:`)
      || !context.endsWith(`:${Number(owner)}:${zone}`)) {
      throw new Error("attached ceremony is not an action shuffle of the authorized sequence");
    }
    const proofs = auth.shuffleProofs || auth.actionAudit?.shuffleProofs || [];
    const fields = (entry) => canonicalMultiplayerPayload({
      owner: Number(entry?.owner),
      zone: String(entry?.zone || "library"),
      deckCount: Number(entry?.deckCount),
      context: String(entry?.context || ""),
      keyContext: String(entry?.keyContext || entry?.context || ""),
      keys: entry?.keys || [],
      steps: entry?.steps || [],
      deckHash: String(entry?.deckHash || ""),
      inputDeck: entry?.inputDeck ?? null,
    });
    const attachedFields = fields(attached);
    if (!Array.isArray(proofs) || !proofs.some((proof) => fields(proof) === attachedFields)) {
      throw new Error("attached ceremony is not one of the authorized action's shuffle proofs");
    }
    assertZiffleShuffleProofBoundToSignedMatch(attached);
    await assertZiffleCeremonyStepsSigned(attached);
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") return null;
    const verified = await currentGame.ziffleVerifyShuffle({
      deckCount: Number(attached.deckCount),
      context,
      keyContext: String(attached.keyContext || context),
      keys: cloneMultiplayerPayload(attached.keys || []),
      steps: cloneMultiplayerPayload(attached.steps || []),
      ...ziffleInputDeckFields(attached),
    });
    if (String(verified?.deckHash || "") !== deckHash) {
      throw new Error("attached ceremony does not verify to its claimed deck hash");
    }
    return cloneMultiplayerPayload(attached);
  }

  // After authorization, a provisional ceremony must have become locally
  // accepted (authorizing the action previews and verifies its shuffle epochs
  // against this engine's own requirements, then remembers them).
  async function locallyAcceptedZiffleCeremonyFor(provisional, auth) {
    const owner = Number(provisional.owner);
    const exact = { deckHash: String(provisional.deckHash || ""), context: String(provisional.context || "") };
    const matches = (entry) => entry
      && String(entry.deckHash || "") === exact.deckHash
      && String(entry.context || "") === exact.context
      && Number(entry.deckCount) === Number(provisional.deckCount);
    let accepted = ziffleCeremonyForOwner(owner, exact);
    if (!matches(accepted) && auth?.command) {
      try {
        const initial = await previewRequirementsForCommand(auth.command);
        await previewZiffleActionRequirements(auth, initial);
      } catch {
        // Fall through: without local acceptance no token is built.
      }
      accepted = ziffleCeremonyForOwner(owner, exact);
    }
    if (!matches(accepted)) {
      throw new Error("Ziffle reveal-token request names a ceremony this peer has not accepted");
    }
    return accepted;
  }

  async function answerZiffleRevealTokenRequest(conn, message) {
    // Announce the (first) response so peers can close their wait on this seat.
    conn = servicesRef.current.protocolResponseConn?.(conn, message) || conn;
    let diagnostics = null;
    try {
      const requester = playerIndexForPeerId(conn?.peer);
      const requestedOwner = Number(message.ceremonyOwner);
      if (requester == null) {
        throw new Error("Ziffle reveal tokens can only be requested by a match player");
      }
      const lookup = {
        deckHash: message.deckHash,
        context: message.ceremonyContext,
      };
      const requestCeremony = message.ceremony;
      const requestCeremonySummary = compactZiffleCeremonyForDiagnostics(requestCeremony);
      let requestCeremonyRejectReason = "";
      const attachedCeremony =
        requestCeremony && typeof requestCeremony === "object"
          ? (() => {
            if (Number(requestCeremony.owner) !== Number(message.ceremonyOwner)) {
              requestCeremonyRejectReason = "attached ceremony owner does not match request owner";
              return null;
            }
            if (
              message.deckHash
              && String(requestCeremony.deckHash || "") !== String(message.deckHash || "")
            ) {
              requestCeremonyRejectReason = "attached ceremony deckHash does not match request deckHash";
              return null;
            }
            if (
              message.ceremonyContext
              && String(requestCeremony.context || "") !== String(message.ceremonyContext || "")
            ) {
              requestCeremonyRejectReason = "attached ceremony context does not match request context";
              return null;
            }
            return cloneMultiplayerPayload(requestCeremony);
          })()
          : null;
      const liveCeremonies = [...liveZiffleCeremoniesRef.current.values()]
        .map(compactZiffleCeremonyForDiagnostics)
        .filter(Boolean);
      const payloadCeremonies = (matchStartPayloadRef.current?.ziffleCeremonies || [])
        .map(compactZiffleCeremonyForDiagnostics)
        .filter(Boolean);
      const currentSession = multiplayerRef.current;
      const actionAuthorizationDebug = {};
      diagnostics = {
        requestId: String(message.requestId || ""),
        requesterPeerId: String(message.requesterPeerId || ""),
        responderPeerId: String(currentSession.localPeerId || ""),
        connectionPeerId: String(conn?.peer || ""),
        responderRole: String(currentSession.role || ""),
        responderMode: String(currentSession.mode || ""),
        responderMatchStarted: Boolean(currentSession.matchStarted),
        responderLocalPlayerIndex:
          currentSession.localPlayerIndex == null ? null : Number(currentSession.localPlayerIndex),
        requestedOwner: Number(message.ceremonyOwner),
        requestedDeckHash: String(message.deckHash || ""),
        requestedContext: String(message.ceremonyContext || ""),
        requestedCardPosition: Number(message.cardPosition),
        actionAuthorizationPresent: Boolean(message.actionAuthorization),
        actionAuthorization: compactActionAuthorizationForDiagnostics(message.actionAuthorization),
        actionAuthorizationDebug,
        attachedCeremonyPresent: Boolean(requestCeremony),
        attachedCeremony: requestCeremonySummary,
        attachedCeremonyRejectReason: requestCeremonyRejectReason,
        matchStartPayloadPresent: Boolean(matchStartPayloadRef.current),
        matchStartAuditMatchId: String(matchStartPayloadRef.current?.auditMatchId || ""),
        liveCeremonies,
        payloadCeremonies,
        sessionPlayers: (currentSession.players || []).map((player) => ({
          index: Number(player.index),
          name: String(player.name || ""),
          peerId: String(player.peerId || ""),
          connected: player.connected !== false,
          hasZiffleKey: Boolean(player.ziffleKey),
        })),
      };
      // Tokens are only ever built against a ceremony this peer accepted itself
      // (genesis-verified, or a verified shuffle from an applied or locally
      // previewed action). The requester chooses the lookup keys and can attach
      // any ceremony, so an attached one is at most provisional: it must be a
      // fully verified, signed shuffle of the in-flight action it claims, and
      // the local copy accepted while authorizing that action replaces it
      // before any token is built.
      let ceremony = ziffleCeremonyForOwner(message.ceremonyOwner, lookup);
      let provisionalCeremony = false;
      if (!ceremony && attachedCeremony && message.actionAuthorization) {
        try {
          ceremony = await verifiedProvisionalZiffleCeremony(
            attachedCeremony,
            message.actionAuthorization,
            requestedOwner
          );
          provisionalCeremony = Boolean(ceremony);
        } catch (err) {
          diagnostics.attachedCeremonyRejectReason = toErrorMessage(err);
          ceremony = null;
        }
      }
      if (!ceremony) ceremony = await waitForZiffleCeremony(message.ceremonyOwner, lookup);
      if (!ceremony) {
        const error = new Error("Unknown ziffle ceremony");
        error.ziffleDiagnostics = diagnostics;
        throw error;
      }
      const rawCardPositions = Array.isArray(message.cardPositions) && message.cardPositions.length > 0
        ? message.cardPositions
        : [message.cardPosition];
      const cardPositions = normalizeZiffleCardPositions(rawCardPositions, ceremony, {
        label: "Ziffle reveal-token request",
      });
      const revealTokenPerf = {
        request_id: String(message.requestId || ""),
        requester,
        owner: requestedOwner,
        positions: cardPositions.length,
        sample_positions: cardPositions.slice(0, 12).map(Number),
        request_bytes: payloadSizeBytes(message),
      };
      recordPeerSyncPerf("ziffle_reveal_token_request:received", revealTokenPerf);
      const authorizedByCryptoRequest = await ziffleRevealAuthorizedByOutboundCryptoRequest(
        message,
        requester,
        requestedOwner,
        cardPositions,
        ceremony
      );
      const actionDisclosure = {};
      const authorizedByAction = authorizedByCryptoRequest
        ? false
        : await ziffleRevealAuthorizedByAction(
          message,
          requester,
          requestedOwner,
          cardPositions,
          ceremony,
          actionAuthorizationDebug,
          actionDisclosure
        );
      // The visible-state fallback only reopens positions the requester is
      // already entitled to look at (the owner's own known cards, or another
      // player's face-down exiled card it may look at); anything else must
      // carry an explicit authorization.
      const allowedPositions =
        (authorizedByCryptoRequest || authorizedByAction)
          ? new Set()
          : await waitForAuthorizedZiffleRevealPositions(
            requestedOwner,
            String(ceremony.deckHash || ""),
            cardPositions,
            10000,
            requester
          );
      for (const position of cardPositions) {
        if (
          !allowedPositions.has(Number(position))
          && !authorizedByCryptoRequest
          && !authorizedByAction
        ) {
          throw new Error("Ziffle reveal-token request is not authorized by the visible hidden-zone state");
        }
      }
      if (provisionalCeremony) {
        ceremony = await locallyAcceptedZiffleCeremonyFor(ceremony, message.actionAuthorization);
        normalizeZiffleCardPositions(cardPositions, ceremony, { label: "Ziffle reveal-token request" });
      }
      if (authorizedByAction && message.actionAuthorization?.actionIntent) {
        const requestPayload = cloneMultiplayerPayload(message);
        await timePeerSyncPhase(
          "ziffle_reveal_token_request:remember_intent",
          revealTokenPerf,
          async () => rememberPendingActionIntent(message.actionAuthorization.actionIntent, {
            requestType: "ziffle_reveal_token_request",
            requestId: String(message.requestId || ""),
            requestPayload,
            requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
            responseTimeoutMs: ziffleRevealTokenTimeoutMs(cardPositions.length, ceremony),
            requestedAtMs: Date.now(),
          })
        );
      }
      if (authorizedByAction && actionDisclosure.intent) {
        // Pin the sequence before any token leaves this peer.
        lockFairRandomRevealIntent(actionDisclosure.intent);
      }
      setStatus(`Generating hidden-card reveal payloads for ${cardPositions.length} card${cardPositions.length === 1 ? "" : "s"}`);
      const tokens = await timePeerSyncPhase(
        "ziffle_reveal_token_request:build_local_tokens",
        revealTokenPerf,
        () => buildLocalZiffleRevealTokens(ceremony, cardPositions)
      );
      recordPeerSyncPerf("ziffle_reveal_token_request:send_response", {
        ...revealTokenPerf,
        tokens: Array.isArray(tokens) ? tokens.length : 0,
        response_bytes: payloadSizeBytes({ tokens }),
      });
      safeSend(conn, {
        type: "ziffle_reveal_token_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        token: tokens[0] || null,
        tokens,
      });
    } catch (err) {
      const ziffleDiagnostics = err?.ziffleDiagnostics || diagnostics;
      if (ziffleDiagnostics) {
        console.warn("Ironsmith ziffle reveal-token request failed", ziffleDiagnostics);
      }
      // Diagnostics (stored/previewed requirement slots and positions, live
      // ceremony summaries, peer list) stay local: they describe this seat's
      // hidden-card bookkeeping and must never go on the wire.
      safeSend(conn, {
        type: "ziffle_reveal_token_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  function ziffleRoutePeerCandidates(peerId) {
    const target = String(peerId || "").trim();
    if (!target) return [];
    const players = routingPlayers();
    const candidates = [target];
    const hostRoute = currentHostRouteInfo();
    for (const player of players || []) {
      const stablePeerId = String(player?.peerId || "").trim();
      const currentPeerId = String(player?.currentPeerId || "").trim();
      if (stablePeerId !== target && currentPeerId !== target) continue;
      if (currentPeerId) candidates.push(currentPeerId);
      if (stablePeerId) candidates.push(stablePeerId);
      if (
        hostRoute.peerId
        && hostRoute.index != null
        && Number(player?.index) === Number(hostRoute.index)
      ) {
        candidates.push(hostRoute.peerId);
      }
    }
    return [...new Set(candidates.filter(Boolean))];
  }

  function openConnectionForPeerCandidates(connections, candidates) {
    for (const candidate of candidates) {
      const conn = connections.get(candidate);
      if (conn && conn.open !== false) return conn;
    }
    for (const conn of connections.values()) {
      if (!conn || conn.open === false) continue;
      if (candidates.includes(String(conn.peer || "").trim())) return conn;
    }
    return null;
  }

  function openZiffleRoute(peerId) {
    const target = String(peerId || "").trim();
    const session = multiplayerRef.current;
    if (!target || target === session.localPeerId) return null;
    const candidates = ziffleRoutePeerCandidates(target);
    if (session.role === "host") {
      return (
        openConnectionForPeerCandidates(clientConnectionsRef.current, candidates)
        || openConnectionForPeerCandidates(peerConnectionsRef.current, candidates)
      );
    }
    const hostRoute = currentHostRouteInfo();
    if (
      candidates.includes(String(session.hostPeerId || "").trim())
      || (hostRoute.peerId && candidates.includes(hostRoute.peerId))
    ) {
      const conn = hostConnectionRef.current;
      if (conn && conn.open !== false) return conn;
      return openConnectionForPeerCandidates(peerConnectionsRef.current, candidates);
    }
    return openConnectionForPeerCandidates(peerConnectionsRef.current, candidates);
  }

  async function waitForZiffleRoute(peerId, timeoutMs = Math.min(PROTOCOL_RESPONSE_TIMEOUT_MS, 30000)) {
    const target = String(peerId || "").trim();
    const started = Date.now();
    const nextDirectAttemptAt = new Map();
    while (Date.now() - started < timeoutMs) {
      const conn = openZiffleRoute(target);
      if (conn) return conn;
      const session = multiplayerRef.current;
      const now = Date.now();
      for (const candidate of ziffleRoutePeerCandidates(target)) {
        if (
          !candidate
          || candidate === String(session.localPeerId || "").trim()
          || Number(nextDirectAttemptAt.get(candidate) || 0) > now
        ) {
          continue;
        }
        nextDirectAttemptAt.set(candidate, now + 3000);
        connectDirectPeer(candidate);
      }
      await sleep(50);
    }
    const session = multiplayerRef.current;
    const error = new Error(`No direct ziffle route to peer ${target || peerId}`);
    error.ziffleDiagnostics = {
      localPeerId: String(session.localPeerId || ""),
      role: String(session.role || ""),
      hostPeerId: String(session.hostPeerId || ""),
      hostRoute: currentHostRouteInfo(),
      hostConnectionPeer: String(hostConnectionRef.current?.peer || ""),
      hostConnectionOpen: Boolean(hostConnectionRef.current && hostConnectionRef.current.open !== false),
      peerConnectionPeers: [...peerConnectionsRef.current.entries()].map(([key, conn]) => ({
        key: String(key || ""),
        peer: String(conn?.peer || ""),
        open: Boolean(conn && conn.open !== false),
      })),
      clientConnectionPeers: [...clientConnectionsRef.current.entries()].map(([key, conn]) => ({
        key: String(key || ""),
        peer: String(conn?.peer || ""),
        open: Boolean(conn && conn.open !== false),
      })),
      candidates: ziffleRoutePeerCandidates(target),
      players: routingPlayers().map((player) => ({
        index: player?.index == null ? null : Number(player.index),
        peerId: String(player?.peerId || ""),
        currentPeerId: String(player?.currentPeerId || ""),
        connected: player?.connected !== false,
      })),
    };
    throw error;
  }

  async function sendDirectProtocolMessage(peerId, payload, options = {}) {
    const target = String(peerId || "").trim();
    if (!target) {
      throw new Error(`Missing peer route for ${String(payload?.type || "protocol message")}`);
    }
    const session = multiplayerRef.current;
    if (target === String(session.localPeerId || "").trim()) {
      throw new Error(`Cannot request ${String(payload?.type || "protocol message")} from the local peer`);
    }
    const attempts = Math.max(1, Number(options.attempts || 3));
    const routeTimeoutMs = Math.max(250, Number(options.routeTimeoutMs || 5000));
    const retryDelayMs = Math.max(50, Number(options.retryDelayMs || 500));
    let lastError = null;
    for (let attempt = 1; attempt <= attempts; attempt += 1) {
      try {
        const conn = await waitForZiffleRoute(target, routeTimeoutMs);
        safeSend(conn, payload);
        return true;
      } catch (err) {
        lastError = err;
      }
      if (sendDirectPeerMessage(target, payload)) {
        return true;
      }
      if (attempt < attempts) {
        await sleep(retryDelayMs);
      }
    }
    if (lastError) throw lastError;
    throw new Error(`No direct ziffle route to peer ${target}`);
  }

  async function collectZiffleRevealTokens(ceremony, cardPosition, options = {}) {
    const tokens = await collectZiffleRevealTokensBatch(ceremony, [cardPosition], options);
    return tokens
      .filter((token) => Number(token.cardPosition ?? cardPosition) === Number(cardPosition))
      .map((token) => ({
        player: token.player,
        publicKeyHex: token.publicKeyHex,
        tokenHex: token.tokenHex,
        proofHex: token.proofHex,
      }));
  }

  async function collectZiffleRevealTokensBatch(ceremony, cardPositions, options = {}) {
    const session = multiplayerRef.current;
    const localIndex = resolveLocalPlayerIndex(session);
    const players = reindexPlayers(
      (session.players || []).length > 0
        ? session.players
        : (
          matchStartPayloadRef.current?.currentPlayers
          || matchStartPayloadRef.current?.players
          || []
        )
    );
    const positions = normalizeZiffleCardPositions(cardPositions, ceremony, {
      allowEmpty: true,
      label: "Ziffle reveal token batch",
    });
    if (positions.length === 0) return [];
    const batchPerf = {
      owner: ceremony?.owner == null ? null : Number(ceremony.owner),
      positions: positions.length,
      sample_positions: positions.slice(0, 12),
      key_players: Array.isArray(ceremony?.keys)
        ? ceremony.keys.map((key) => Number(key?.player)).filter((player) => Number.isFinite(player))
        : [],
      command: summarizePeerCommand(options.command),
      seq: options.seq == null ? null : Number(options.seq),
      actor: options.actorIndex == null ? null : Number(options.actorIndex),
    };
    recordPeerSyncPerf("ziffle_reveal_token_batch:start", batchPerf);
    const actionAuthorization = options.command
      ? {
        matchId: currentAuditMatchId(),
        seq: Number(options.seq ?? Number(session.lastAppliedSequence || 0) + 1),
        requesterIndex: Number(localIndex),
        ...(options.actorIndex !== null && options.actorIndex !== undefined
          ? { actorIndex: Number(options.actorIndex) }
          : {}),
        prevStateHash: String(options.prevStateHash || auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH),
        preActionPublicCheckpointHash: String(
          options.preActionPublicCheckpointHash
          || options.publicCheckpointHash
          || ""
        ),
        command: cloneMultiplayerPayload(options.command),
        // Derive reveal authority on the receiving peer. Local requirements
        // can contain privately opened names and original manifest slots.
        ...(options.shuffleProofs ? { shuffleProofs: cloneMultiplayerPayload(options.shuffleProofs) } : {}),
        ...(options.rngReveals ? { rngReveals: cloneMultiplayerPayload(options.rngReveals) } : {}),
        ...(options.openings ? { openings: cloneMultiplayerPayload(options.openings) } : {}),
        ...(options.actionIntent ? { actionIntent: cloneMultiplayerPayload(options.actionIntent) } : {}),
        ...(options.actionAudit ? { actionAudit: cloneMultiplayerPayload(options.actionAudit) } : {}),
      }
      : null;
    const ceremonyKeys = Array.isArray(ceremony.keys) ? ceremony.keys : [];
    const tokenGroups = await collectZiffleRevealTokenGroups({
      keys: ceremonyKeys,
      positions,
      readCachedTokens: (key, requestedPositions) =>
        cachedZiffleRevealTokens(ceremony, Number(key.player), requestedPositions),
      rememberTokens: (tokens, requestedPositions) =>
        rememberZiffleRevealTokens(ceremony, tokens, requestedPositions),
      onCacheHit: (key, tokens) => recordPeerSyncPerf("ziffle_reveal_token_batch:cache_hit", {
        ...batchPerf,
        token_player: Number(key.player),
        tokens: tokens.length,
      }),
      // Launch local proof generation and peer requests together. Only missing
      // positions go over the wire; every request keeps its full authorization.
      collectTokens: async (key, positions) => {
        const tokenPlayer = Number(key.player);
        const signerPerf = {
          ...batchPerf,
          positions: positions.length,
          sample_positions: positions.slice(0, 12),
        };
        if (tokenPlayer === Number(localIndex)) {
          return timePeerSyncPhase(
            "ziffle_reveal_token_batch:build_local_tokens",
            { ...signerPerf, token_player: tokenPlayer },
            () => buildLocalZiffleRevealTokens(ceremony, positions)
          );
        }
        const peer = players.find((player) => Number(player.index) === tokenPlayer) || {
          index: tokenPlayer,
        };
        const routePeerId = routePeerIdForPlayer(peer);
        if (!routePeerId) {
          throw new Error(`Missing peer for ziffle reveal token player ${key.player}`);
        }
        const peerLabel = peer.name || `Player ${Number(key.player) + 1}`;
        setStatus(`Waiting for ${peerLabel} to generate hidden-card reveal payloads`);
        const requestId = makeZiffleRequestId("ziffle-reveal");
        const revealTokenTimeoutMs = ziffleRevealTokenTimeoutMs(positions.length, ceremony);
        const responseTimeoutMs = actionAuthorization
          ? revealTokenTimeoutMs
          : Math.max(revealTokenTimeoutMs, PROTOCOL_RESPONSE_TIMEOUT_MS);
        const requestDiagnostics = {
          requestId,
          localPeerId: String(session.localPeerId || ""),
          localRole: String(session.role || ""),
          localMode: String(session.mode || ""),
          targetPeerId: routePeerId,
          targetStablePeerId: String(peer.peerId || ""),
          targetPlayerIndex: tokenPlayer,
          ceremony: compactZiffleCeremonyForDiagnostics(ceremony),
          cardPosition: positions.length === 1 ? positions[0] : null,
          cardPositions: positions,
          matchStartPayloadPresent: Boolean(matchStartPayloadRef.current),
          matchStartAuditMatchId: String(matchStartPayloadRef.current?.auditMatchId || ""),
          actionIntentKey: actionAuthorization?.actionIntent
            ? actionIntentKey(actionAuthorization.actionIntent)
            : "",
        };
        const requestedAtMs = Date.now();
        const waiter = waitForZiffleRevealToken(requestId, responseTimeoutMs, requestDiagnostics, {
          peerIndex: tokenPlayer,
          peerName: peerLabel,
          description:
            `${peerLabel} is generating reveal proof material for ${positions.length} hidden `
            + `card${positions.length === 1 ? "" : "s"}.`,
          operation: "Waiting for reveal proof",
          detail: zifflePositionsDetail(positions),
          progressCurrent: 0,
          progressTotal: positions.length,
          responseTimeoutMs,
        });
        const includeCeremonyInRevealRequest =
          Boolean(options.includeCeremonyInRevealRequest)
          || Boolean(actionAuthorization);
        const requestPayload = {
          type: "ziffle_reveal_token_request",
          protocolVersion: PROTOCOL_VERSION,
          requestId,
          ceremonyOwner: Number(ceremony.owner),
          deckHash: String(ceremony.deckHash || ""),
          ceremonyContext: String(ceremony.context || ""),
          ...(includeCeremonyInRevealRequest ? { ceremony: cloneMultiplayerPayload(ceremony) } : {}),
          cardPosition: positions[0],
          cardPositions: positions,
          requesterPeerId: session.localPeerId || "",
          requesterIndex: localIndex,
          cryptoMaterialRequestId: options.cryptoMaterialRequestId || "",
          ...(actionAuthorization ? { actionAuthorization } : {}),
        };
        const requestPerf = {
          ...signerPerf,
          token_player: tokenPlayer,
          request_id: requestId,
          peer_id: routePeerId,
          request_bytes: payloadSizeBytes(requestPayload),
        };
        recordPeerSyncPerf("ziffle_reveal_token_batch:send_request", requestPerf);
        await sendDirectProtocolMessage(routePeerId, requestPayload);
        const remoteTokens = await timePeerSyncPhase(
          "ziffle_reveal_token_batch:wait_response",
          requestPerf,
          () => waitForProtocolResponse(waiter, {
            basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
            targetPlayerIndex: tokenPlayer,
            targetPeerId: routePeerId,
            requesterIndex: localIndex,
            requestType: requestPayload.type,
            requestId,
            requestPayload,
            responseTimeoutMs,
            requestedAtMs,
          })
        );
        recordPeerSyncPerf("ziffle_reveal_token_batch:received_response", {
          ...requestPerf,
          tokens: Array.isArray(remoteTokens) ? remoteTokens.length : 0,
          response_bytes: payloadSizeBytes(remoteTokens),
        });
        return remoteTokens;
      },
    });
    const tokens = tokenGroups
      .filter(Boolean)
      .flatMap((group) => Array.isArray(group) ? group : [group]);
    recordPeerSyncPerf("ziffle_reveal_token_batch:done", {
      ...batchPerf,
      tokens: tokens.length,
      bytes: payloadSizeBytes(tokens),
    });
    return tokens;
  }

  function acceptedEpochsForProof(owner, seq, precedingProofs = []) {
    return acceptedZiffleEpochs(matchStartPayloadRef.current,
      (actionHistoryRef.current || []).filter(entry => Number(entry.seq) < Number(seq)), owner, precedingProofs);
  }

  async function buildLiveZiffleShuffleProofs(requirements, seq, options = {}) {
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    const keys = zifflePublicKeysForPlayers(players);
    const keyContext = currentAuditMatchId();
    const proofs = [];
    const libraryEpochs = [];
    const ordered = [...requirements].sort((a, b) =>
      Number(a.randomCountBefore ?? a.random_count_before) - Number(b.randomCountBefore ?? b.random_count_before));
    for (const initial of ordered) {
      let requirement = initial;
      // A second shuffle in one resolution must consume the first shuffle's
      // ciphertexts. Re-preview under the earlier epochs before selecting it.
      if (libraryEpochs.length) {
        if (!options.command) throw new Error("Repeated shuffle needs its replay command");
        const refreshed = await gameRef.current.previewCryptoRequirementsWithMaterial(options.command, { libraryEpochs });
        requirement = refreshed.find(entry => String(entry.id) === String(initial.id));
        if (!requirement) throw new Error("Shuffle requirement changed after installing the preceding epoch");
      }
      const inputs = requirement.inputCommitments ?? requirement.input_commitments;
      if (!Array.isArray(inputs)) throw new Error("Engine did not expose authenticated shuffle inputs");
      if (inputs.length <= 1) continue;
      const owner = Number(requirement.owner);
      const accepted = acceptedEpochsForProof(owner, seq, [...(options.precedingProofs || []), ...proofs]);
      const [ceremony] = await runBatchedZiffleShuffleCeremonies([{
        id: String(requirement.id || ""), requirement, requirementId: String(requirement.id || ""),
        owner, zone: String(requirement.zone || "library"), deckCount: inputs.length, keyContext,
        context: [keyContext, "action", Number(seq), "shuffle", String(requirement.id || ""), owner,
          String(requirement.zone || "library")].join(":"),
        keys, inputDeck: buildZiffleInputDeck(accepted, inputs),
      }], players, { keys, kind: "action" });
      const proof = {
        type: "ziffle_shuffle", requirementId: ceremony.requirementId,
        owner, zone: ceremony.zone, epoch: Number(seq), deckCount: ceremony.deckCount,
        context: ceremony.context, keyContext, keys: cloneMultiplayerPayload(ceremony.keys),
        steps: cloneMultiplayerPayload(ceremony.steps), deckHash: ceremony.deckHash,
        ...ziffleInputDeckFields(ceremony),
      };
      assertZiffleEpochVerification(proof, ceremony.verification, accepted);
      libraryEpochs.push(ziffleEpochMaterial(proof, requirement, assertZiffleEpochInputs(proof, requirement, accepted)));
      verifiedShuffleProofsRef.current.add(proof);
      rememberLocalZiffleCeremonyForLookup(proof);
      proofs.push(proof);
    }
    return proofs;
  }

  async function buildLocalShuffleProofsForRequirements(cryptoRequirements = [], seq, options = {}) {
    const requirements = (cryptoRequirements || []).filter(
      requirement => ziffleRequirementType(requirement) === "verifiable_shuffle"
    );
    return buildLiveZiffleShuffleProofs(requirements, seq, options);
  }

  const assertZiffleShuffleProofBoundToSignedMatch = useCallback((proof, requirement = null, options = {}) => {
    const matchPayload = options.matchPayload || matchStartPayloadRef.current;
    const players = reindexPlayers(matchPayload?.players || multiplayerRef.current.players || []);
    const playerSeats = new Set(players.map((player) => Number(player.index)));
    const owner = normalizePlayerIndex(proof?.owner);
    if (owner === null || !playerSeats.has(owner)) {
      throw new Error("Ziffle shuffle proof references an unknown player");
    }
    if (requirement && owner !== normalizePlayerIndex(requirement.owner)) {
      throw new Error(`Ziffle shuffle proof owner mismatch for player ${Number(requirement.owner) + 1}`);
    }
    const zone = String(proof?.zone || "library");
    const expectedZone = String(requirement?.zone || "library");
    if (zone !== expectedZone) {
      throw new Error(`Ziffle shuffle proof zone mismatch for player ${owner + 1}`);
    }
    const expectedKeys = signedZiffleKeysForPayload(matchPayload);
    if (!expectedKeys.length || canonicalMultiplayerPayload(proof?.keys || []) !== canonicalMultiplayerPayload(expectedKeys)) {
      throw new Error("Ziffle shuffle proof is not bound to the signed ziffle key roster");
    }
    const expectedMatchId = String(
      matchPayload?.auditMatchId
        || matchPayload?.matchId
        || currentAuditMatchId()
        || ""
    );
    const proofContext = String(proof?.context || "");
    const proofKeyContext = String(proof?.keyContext || proof?.context || "");
    if (
      !expectedMatchId
      || (proofContext !== expectedMatchId && !proofContext.startsWith(`${expectedMatchId}:`))
    ) {
      throw new Error("Ziffle shuffle proof is bound to a different match");
    }
    if (proofKeyContext !== expectedMatchId) {
      throw new Error("Ziffle shuffle proof uses a mismatched ziffle key context");
    }
  }, [currentAuditMatchId, signedZiffleKeysForPayload]);

  async function verifyShuffleProofsForRequirements(requirements = [], shuffleProofs = [], options = {}) {
    const currentGame = gameRef.current;
    const allowAfterOrderMismatch = Boolean(options.allowAfterOrderMismatch);
    let verifiedCount = 0;
    let skippedCount = 0;
    let verifyMs = 0;
    const pendingVerifications = [];
    const proofsToLock = [];
    for (const requirement of requirements || []) {
      if (ziffleRequirementType(requirement) !== "verifiable_shuffle") continue;
      const proof = (shuffleProofs || []).find((entry) =>
        shuffleProofMatchesRequirement(entry, requirement)
      );
      if (!proof) {
        throw new Error(`Missing verifiable shuffle proof for player ${Number(requirement.owner) + 1}`);
      }
      const expectedContext = [currentAuditMatchId(), "action", Number(options.seq), "shuffle",
        String(requirement.id || ""), Number(requirement.owner), String(requirement.zone || "library")].join(":");
      if (!Number.isSafeInteger(Number(options.seq)) || Number(options.seq) <= 0
        || String(proof.context || "") !== expectedContext) {
        throw new Error("Verifiable shuffle is not bound to this action sequence");
      }
      proofsToLock.push(proof);
      assertZiffleShuffleProofBoundToSignedMatch(proof, requirement);
      if (isPrivateZiffleEpoch(proof)) {
        const accepted = acceptedEpochsForProof(proof.owner, options.seq, shuffleProofs.slice(0, shuffleProofs.indexOf(proof)));
        assertZiffleEpochInputs(proof, requirement, accepted);
        if (verifiedShuffleProofsRef.current.has(proof)) skippedCount += 1;
        else pendingVerifications.push({ proof, accepted });
        continue;
      }
      if (Number(matchStartPayloadRef.current?.protocolVersion) >= 15) {
        throw new Error("Verified match requires an authenticated private shuffle epoch");
      }
      const requirementBefore = normalizeShuffleOrder(requirement.beforeOrder ?? requirement.before_order);
      const requirementAfter = normalizeShuffleOrder(requirement.afterOrder ?? requirement.after_order);
      const proofBefore = normalizeShuffleOrder(proof.beforeOrder ?? proof.before_order);
      const proofAfter = normalizeShuffleOrder(proof.afterOrder ?? proof.after_order);
      const beforeMap = shuffleOrderIdMap(proofBefore, requirementBefore);
      const afterMap = shuffleOrderIdMap(proofAfter, requirementAfter);
      const localizedProofBefore = beforeMap ? localizeShuffleOrder(proofBefore, beforeMap) : proofBefore;
      const localizedProofAfter = afterMap ? localizeShuffleOrder(proofAfter, afterMap) : proofAfter;
      if (
        requirementBefore.length > 0
        && !sameShuffleOrder(localizedProofBefore, requirementBefore)
      ) {
        throw new Error(`Verifiable shuffle before-order mismatch for player ${Number(requirement.owner) + 1}`);
      }
      if (
        requirementAfter.length > 0
        && !sameShuffleOrder(localizedProofAfter, requirementAfter)
      ) {
        if (!allowAfterOrderMismatch) {
          throw new Error(`Verifiable shuffle after-order mismatch for player ${Number(requirement.owner) + 1}`);
        }
      }
      if (verifiedShuffleProofsRef.current.has(proof)) {
        skippedCount += 1;
        continue;
      }
      if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
        throw new Error("Ziffle mental-poker backend is not available");
      }
      assertZiffleShuffleProofBoundToSignedMatch(proof, requirement);
      pendingVerifications.push({ proof });
    }
    if (pendingVerifications.length > 0) {
      const verifyStartedAt = nowMonotonicMs();
      const verifiedProofs = await Promise.all(pendingVerifications.map(async ({ proof, accepted }) => {
        await assertZiffleCeremonyStepsSigned(proof);
        const verified = await currentGame.ziffleVerifyShuffle({
          deckCount: Number(proof.deckCount),
          context: String(proof.context || ""),
          keyContext: String(proof.keyContext || proof.context || ""),
          keys: cloneMultiplayerPayload(proof.keys || []),
          steps: cloneMultiplayerPayload(proof.steps || []),
          ...ziffleInputDeckFields(proof),
        });
        if (String(verified.deckHash || "") !== String(proof.deckHash || "")) {
          throw new Error(`Ziffle shuffle proof mismatch for player ${Number(proof.owner) + 1}`);
        }
        if (accepted) assertZiffleEpochVerification(proof, verified, accepted);
        return proof;
      }));
      verifyMs = nowMonotonicMs() - verifyStartedAt;
      for (const proof of verifiedProofs) {
        verifiedShuffleProofsRef.current.add(proof);
      }
      verifiedCount = verifiedProofs.length;
    }
    lockVerifiedZiffleActionShuffles(proofsToLock);
    if (verifiedCount > 0 || skippedCount > 0) {
      recordZiffleShufflePerf({
        kind: "verify",
        verifiedCount,
        skippedCount,
        verifyMs: Math.round(verifyMs),
      });
    }
  }

  // Applies verified ziffle shuffles to the local engine, in the order the
  // shuffles happened within the action (alignShuffleProofsWithRequirements
  // sorts them chronologically and matches proofs by requirement id only).
  //
  // Each proof's afterOrder is the full verified deck: index p is the object at
  // ziffle position p. It is passed to the engine unprojected so that
  //  - cards that left the library after the shuffle in the same action (draws,
  //    mulligans, Mind's Desire) are re-pinned to their post-shuffle position,
  //  - library cards that were excluded from the shuffle and reinserted
  //    (Mystical/Vampiric/Enlightened/Worldly Tutor, Imperial Seal: the engine
  //    journals the shuffle over N-1 cards plus a hidden_order_update over N)
  //    keep their existing verified position, and
  //  - the lookup ceremony keeps every position, so zifflePositionForObjectId
  //    still resolves drawn/exiled cards.
  // The engine only re-sequences the library to a proof's order for the last
  // shuffle of that owner, and only when no deterministic reorder
  // (hidden_order_update) in the action moved cards around it.
  async function applyVerifiedShuffleProofs(shuffleProofs = [], options = {}) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.applyVerifiedHiddenLibraryShuffle !== "function") return;
    const libraryProofs = (shuffleProofs || []).filter((proof) =>
      proof && String(proof?.zone || "library") === "library"
    );
    if (libraryProofs.length === 0) return;
    const lastProofIndexByOwner = new Map();
    libraryProofs.forEach((proof, index) => {
      lastProofIndexByOwner.set(Number(proof.owner), index);
    });
    const ownersWithOrderUpdates = new Set(
      (options.requirements || [])
        .filter((requirement) =>
          ziffleRequirementType(requirement) === "hidden_order_update"
          && String(requirement?.zone || "library") === "library"
        )
        .map((requirement) => Number(requirement.owner))
    );
    for (const [index, proof] of libraryProofs.entries()) {
      const owner = Number(proof.owner);
      if (isPrivateZiffleEpoch(proof)) {
        // Tokens are bound to exact ciphertexts. Withdrawn cards retain their
        // old ciphertext; new outputs use different hashes and cannot reuse them.
        clearOwnerZiffleOpeningCache(owner, undefined, { preserveCiphertextTokens: true });
        liveZiffleCeremoniesRef.current.set(owner, cloneMultiplayerPayload(proof));
        rememberLocalZiffleCeremonyForLookup(proof);
        continue;
      }
      const afterOrder = normalizeShuffleOrder(proof.afterOrder ?? proof.after_order);
      const enforceLibraryOrder =
        lastProofIndexByOwner.get(owner) === index
        && !ownersWithOrderUpdates.has(owner);
      try {
        await currentGame.applyVerifiedHiddenLibraryShuffle({
          owner,
          deckHash: String(proof.deckHash || ""),
          afterOrder,
          enforceLibraryOrder,
        });
      } catch (err) {
        throw new Error(
          `${String(err?.message || err || "failed to apply verified shuffle")}; `
          + `shuffle proof owner ${owner} requirement ${String(proof.requirementId || "")} `
          + `afterOrderLen ${afterOrder.length} firstAfterOrder ${afterOrder.slice(0, 12).join(",")}`
        );
      }
      clearOwnerZiffleOpeningCache(owner);
      const localProof = {
        ...cloneMultiplayerPayload(proof),
        afterOrder,
        after_order: afterOrder,
        authenticatedOrder: true,
      };
      liveZiffleCeremoniesRef.current.set(owner, localProof);
      rememberLocalZiffleCeremonyForLookup(localProof);
    }
  }

  async function rngCommitmentForNonce(nonceHex) {
    return sha256Hex(canonicalJson({
      domain: "ironsmith-rng-commit-v1",
      nonceHex: String(nonceHex || ""),
    }));
  }

  async function signRngCommitmentEntry({
    matchId,
    seq,
    requirementId,
    requestId,
    requester,
    player,
    commitmentHex,
  }) {
    const { keyPair } = await ensureAuditIdentity();
    const payload = rngCommitmentPayload({
      matchId,
      seq,
      requirementId,
      requestId,
      requester,
      player,
      commitmentHex,
    });
    return {
      player: Number(player),
      requester: Number(requester),
      requestId: String(requestId || ""),
      commitmentHex: String(commitmentHex || ""),
      signature: await signAuditPayload(keyPair, payload),
    };
  }

  async function signRngRevealEntry({
    matchId,
    seq,
    requirementId,
    requestId,
    commitRequestId,
    requester,
    player,
    nonceHex,
    commitmentHex,
  }) {
    const { keyPair } = await ensureAuditIdentity();
    const payload = rngRevealPayload({
      matchId,
      seq,
      requirementId,
      requestId,
      commitRequestId,
      requester,
      player,
      nonceHex,
      commitmentHex,
    });
    return {
      player: Number(player),
      requester: Number(requester),
      requestId: String(requestId || ""),
      commitRequestId: String(commitRequestId || ""),
      nonceHex: String(nonceHex || ""),
      commitmentHex: String(commitmentHex || ""),
      signature: await signAuditPayload(keyPair, payload),
    };
  }

  async function verifyRngCommitmentEntry(entry, {
    matchId,
    seq,
    requirementId,
    requester,
    player,
  }) {
    if (!entry?.signature) {
      throw new Error("Random commitment response is missing its signature");
    }
    const signer = Number(player ?? entry.player);
    const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(signer));
    const valid = await verifyAuditPayload(publicKey, rngCommitmentPayload({
      matchId,
      seq,
      requirementId,
      requestId: entry.requestId,
      requester: requester ?? entry.requester,
      player: signer,
      commitmentHex: entry.commitmentHex,
    }), entry.signature || "");
    if (!valid) {
      throw new Error("Random commitment response signature is invalid");
    }
  }

  async function verifyRngRevealEntry(entry, {
    matchId,
    seq,
    requirementId,
    requester,
    player,
  }) {
    if (!entry?.signature) {
      throw new Error("Random reveal response is missing its signature");
    }
    const signer = Number(player ?? entry.player);
    const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(signer));
    const valid = await verifyAuditPayload(publicKey, rngRevealPayload({
      matchId,
      seq,
      requirementId,
      requestId: entry.requestId,
      commitRequestId: entry.commitRequestId,
      requester: requester ?? entry.requester,
      player: signer,
      nonceHex: entry.nonceHex,
      commitmentHex: entry.commitmentHex,
    }), entry.signature || "");
    if (!valid) {
      throw new Error("Random reveal response signature is invalid");
    }
  }

  function currentHostRouteInfo() {
    const session = multiplayerRef.current;
    const payload = matchStartPayloadRef.current || {};
    const hostPeerId = firstOnlinePeerId(
      payload.currentHostPeerId,
      payload.hostPeerId,
      session.hostPeerId
    );
    const players = [
      ...(Array.isArray(session.players) ? session.players : []),
      ...(Array.isArray(payload.currentPlayers) ? payload.currentPlayers : []),
      ...(Array.isArray(payload.players) ? payload.players : []),
    ];
    const matchedHost = hostPeerId
      ? players.find((entry) =>
        String(entry?.peerId || "").trim() === hostPeerId
        || String(entry?.currentPeerId || "").trim() === hostPeerId
      )
      : null;
    const sessionHostIndex = (
      session.role === "host"
      && hostPeerId
      && String(session.localPeerId || "").trim() === hostPeerId
    )
      ? normalizePlayerIndex(session.localPlayerIndex)
      : null;
    const hostIndex =
      sessionHostIndex
      ?? normalizePlayerIndex(payload.currentHostPlayerIndex)
      ?? normalizePlayerIndex(matchedHost?.index)
      ?? normalizePlayerIndex(payload.genesis?.hostSeat)
      ?? (hostPeerId ? 0 : null);
    return { peerId: hostPeerId, index: hostIndex };
  }

  function routingPlayers() {
    const session = multiplayerRef.current;
    const payload = matchStartPayloadRef.current || {};
    return [
      ...(Array.isArray(session.players) ? session.players : []),
      ...(Array.isArray(payload.currentPlayers) ? payload.currentPlayers : []),
      ...(Array.isArray(payload.players) ? payload.players : []),
    ];
  }

  function playerIndexForPeerId(peerId) {
    const peer = String(peerId || "");
    const players = routingPlayers();
    const player = players.find((entry) =>
      String(entry?.peerId || "") === peer
      || String(entry?.currentPeerId || "") === peer
    );
    if (player?.index != null) return Number(player.index);
    const hostRoute = currentHostRouteInfo();
    if (peer && hostRoute.peerId && peer === hostRoute.peerId) {
      return hostRoute.index == null ? null : Number(hostRoute.index);
    }
    return null;
  }

  function routePeerIdForPlayer(player) {
    const index = normalizePlayerIndex(player?.index);
    const session = multiplayerRef.current;
    const livePlayers = routingPlayers();
    const indexedPlayers = index == null
      ? []
      : livePlayers.filter((entry) => Number(entry?.index) === index);
    const hostPeerId = String(session.hostPeerId || "").trim();
    const liveCurrentPeerId = firstOnlinePeerId(
      ...indexedPlayers.map((entry) => entry?.currentPeerId)
    );
    const liveStablePeerId = firstOnlinePeerId(
      ...indexedPlayers.map((entry) => entry?.peerId)
    );
    const stablePeerId = String(player?.peerId || "").trim();
    const currentPeerId = String(player?.currentPeerId || "").trim();
    const hostRoute = currentHostRouteInfo();
    if (
      session.role === "client"
      && hostRoute.peerId
      && hostRoute.index != null
      && index === hostRoute.index
    ) {
      return hostRoute.peerId;
    }
    const matchesHostPeerId = Boolean(
      hostPeerId
      && (
        liveStablePeerId === hostPeerId
        || liveCurrentPeerId === hostPeerId
        || stablePeerId === hostPeerId
        || currentPeerId === hostPeerId
      )
    );
    if (
      matchesHostPeerId
      && (
        hostRoute.index == null
        || index == null
        || Number(index) === Number(hostRoute.index)
      )
    ) {
      return hostPeerId;
    }
    return firstOnlinePeerId(
      liveCurrentPeerId,
      liveStablePeerId,
      currentPeerId,
      stablePeerId
    );
  }

  // Once this peer discloses anything for an in-flight action at (match, seq,
  // actor, pre-state) -- its fair-random nonce, or ziffle reveal tokens the
  // action's previewed requirements authorized -- the actor has learned
  // something it cannot un-learn. The action at that sequence is then pinned
  // to the command of the disclosed intent: a cancel does not release it, a
  // different command cannot re-roll with fresh nonces, and a different
  // (cheap) action cannot be substituted after peeking. The only permitted
  // outcomes are completing that command or a forfeit. Cancels before any
  // disclosure stay free, since commitments alone disclose nothing.
  function fairRandomRevealLockKey({ matchId, seq, actorIndex, prevStateHash } = {}) {
    return [
      "fair-random-intent",
      String(matchId || ""),
      Number(seq || 0),
      Number(actorIndex),
      String(prevStateHash || ""),
    ].join(":");
  }

  function fairRandomRevealLockConflict(intent = {}) {
    const locked = rngRevealCommitSetLocksRef.current.get(fairRandomRevealLockKey(intent));
    if (!locked) return false;
    if (isForfeitCommandForDisclosureLock(intent.command, intent.actorIndex)) return false;
    return locked !== canonicalMultiplayerPayload(intent.command || {});
  }

  function isForfeitCommandForDisclosureLock(command, actorIndex) {
    if (!command || typeof command !== "object") return false;
    return isSelfForfeitCommand(command, actorIndex)
      || isActionTimeoutForfeitCommand(command)
      || isDisconnectTimeoutForfeitCommand(command)
      || isProtocolResponseTimeoutForfeitCommand(command)
      || isWitnessForfeitCommand(command);
  }

  function lockFairRandomRevealIntent(intent = {}) {
    if (fairRandomRevealLockConflict(intent)) {
      throw new Error("Random reveal request conflicts with an action intent already revealed for this sequence");
    }
    rngRevealCommitSetLocksRef.current.set(
      fairRandomRevealLockKey(intent),
      canonicalMultiplayerPayload(intent.command || {})
    );
  }

  function fairRandomRequirementId(requirement) {
    return String(
      requirement?.id
      || requirement?.requirementId
      || requirement?.requirement_id
      || ""
    );
  }

  async function fairRandomRequestContextKey({
    matchId,
    seq,
    requirement,
    requester,
    actorIndex,
    prevStateHash,
    publicCheckpointHash,
    command,
  }) {
    return sha256Hex(canonicalMultiplayerPayload({
      domain: "ironsmith-rng-request-context-v1",
      matchId: String(matchId || ""),
      seq: Number(seq),
      requester: Number(requester),
      actorIndex: Number(actorIndex),
      prevStateHash: String(prevStateHash || ""),
      publicCheckpointHash: String(publicCheckpointHash || ""),
      command: cloneMultiplayerPayload(command),
      requirement: cloneMultiplayerPayload(requirement),
    }));
  }

  async function fairRandomCommitSetHash(contextKey, commits = []) {
    return sha256Hex(canonicalMultiplayerPayload({
      domain: "ironsmith-rng-commit-set-lock-v1",
      contextKey: String(contextKey || ""),
      commits: (Array.isArray(commits) ? commits : [])
        .map((entry) => ({
          player: Number(entry?.player),
          commitmentHex: String(entry?.commitmentHex || ""),
        }))
        .sort((left, right) => Number(left.player) - Number(right.player)),
    }));
  }

  async function validateCompleteRngCommitSet(commits, request) {
    const entries = Array.isArray(commits) ? commits : [];
    const expectedPlayers = reindexPlayers(
      matchStartPayloadRef.current?.players || multiplayerRef.current.players || []
    ).map((player) => Number(player.index)).sort((left, right) => left - right);
    if (entries.length !== expectedPlayers.length) {
      throw new Error("Random reveal request must include every player commitment");
    }
    for (let index = 0; index < expectedPlayers.length; index += 1) {
      if (Number(entries[index]?.player) !== expectedPlayers[index]) {
        throw new Error("Random reveal request commitments must be sorted and complete");
      }
    }
    for (const entry of entries) {
      const player = Number(entry?.player);
      if (Number(entry?.requester) !== Number(request.requesterPlayer)) {
        throw new Error("Random commitment set requester mismatch");
      }
      if (!String(entry?.commitmentHex || "")) {
        throw new Error("Random commitment set is missing a commitment");
      }
      await verifyRngCommitmentEntry(entry, {
        matchId: request.matchId,
        seq: request.seq,
        requirementId: request.requirementId,
        requester: request.requesterPlayer,
        player,
      });
    }
    return cloneMultiplayerPayload(entries);
  }

  async function validateIncomingRngRequest(conn, message, label) {
    let session = multiplayerRef.current;
    assertMatchNotDisputed(session, label);
    if (!session.matchStarted) {
      throw new Error(`${label} received before match start`);
    }
    if (String(message?.matchId || "") !== currentAuditMatchId()) {
      throw new Error(`${label} belongs to a different match`);
    }
    const seq = Number(message?.seq);
    if (!String(message?.requirementId || "")) {
      throw new Error(`${label} is missing a requirement id`);
    }
    const requester = playerIndexForPeerId(conn?.peer);
    if (requester == null) {
      throw new Error(`${label} requester is not a match player`);
    }
    if (normalizePlayerIndex(message?.requesterIndex) !== requester) {
      throw new Error(`${label} requester index does not match the peer`);
    }
    const actorIndex = normalizePlayerIndex(message?.actorIndex);
    if (actorIndex == null || actorIndex !== requester) {
      throw new Error(`${label} requester is not the acting player`);
    }
    await servicesRef.current.waitForProtocolActionHead?.(message, label);
    session = multiplayerRef.current;
    assertMatchNotDisputed(session, label);
    const expectedSeq = Number(session.lastAppliedSequence || 0) + 1;
    if (!Number.isSafeInteger(seq) || seq !== expectedSeq) {
      throw new Error(`${label} has an invalid action sequence`);
    }
    const prevStateHash = String(message?.prevStateHash || "");
    if (prevStateHash !== String(auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH)) {
      throw new Error(`${label} is not based on the local transcript head`);
    }
    const publicCheckpointHash = String(message?.publicCheckpointHash || "");
    if (!publicCheckpointHash) {
      throw new Error(`${label} is missing the public checkpoint hash`);
    }
    const command = message?.command;
    if (!command || typeof command !== "object") {
      throw new Error(`${label} is missing the signed command preview`);
    }
    const requestedRequirement = message?.requirement;
    if (
      !requestedRequirement
      || typeof requestedRequirement !== "object"
      || String(requestedRequirement?.type || requestedRequirement?.requirement_type || "") !== "fair_random"
    ) {
      throw new Error(`${label} is missing the fair-random requirement`);
    }
    const decision = gameRef.current
      ? (await gameRef.current.uiState())?.decision
      : stateRef.current?.decision;
    if (
      decision?.player == null
      || Number(decision.player) !== Number(requester)
    ) {
      throw new Error(`${label} was not sent by the active decision player`);
    }
    if (!isDecisionCommandCompatible(decision, command)) {
      throw new Error(`${label} command is not available locally`);
    }
    await verifyCurrentPublicCheckpointHash(
      publicCheckpointHash,
      `${label} public checkpoint does not match local state`
    );
    const actionIntent = await verifySignedActionIntent(message.actionIntent, {
      matchId: currentAuditMatchId(),
      seq,
      actorIndex,
      prevStateHash,
      preActionPublicCheckpointHash: publicCheckpointHash,
      command,
    });
    const liveState = gameRef.current && typeof gameRef.current.uiState === "function"
      ? await gameRef.current.uiState()
      : stateRef.current;
    const previewedRequirements = filterCryptoRequirementsForCommand(
      command,
      liveState,
      freshCryptoRequirementsForSequence(
        seq,
        await previewRequirementsForCommand(command)
      )
    );
    const requestedRequirementKey = cryptoRequirementReplayKey(requestedRequirement);
    const requirement = previewedRequirements.find((entry) =>
      String(entry?.type || entry?.requirement_type || "") === "fair_random"
      && fairRandomRequirementId(entry) === String(message.requirementId || "")
      && cryptoRequirementReplayKey(entry) === requestedRequirementKey
    );
    if (!requirement) {
      throw new Error(`${label} asks for unauthorized fair-random material`);
    }
    const contextKey = await fairRandomRequestContextKey({
      matchId: currentAuditMatchId(),
      seq,
      requirement,
      requester,
      actorIndex,
      prevStateHash,
      publicCheckpointHash,
      command,
    });
    if (fairRandomRevealLockConflict({
      matchId: currentAuditMatchId(),
      seq,
      actorIndex,
      prevStateHash,
      command,
    })) {
      throw new Error(`${label} conflicts with an action intent whose randomness was already revealed`);
    }
    return {
      matchId: currentAuditMatchId(),
      seq,
      requirementId: String(message.requirementId || ""),
      requirement,
      contextKey,
      command,
      actorIndex,
      prevStateHash,
      publicCheckpointHash,
      actionIntent,
      requesterPeerId: String(conn?.peer || ""),
      requesterPlayer: Number(requester),
    };
  }

  async function answerRngCommitRequest(conn, message) {
    // Announce the (first) response so peers can close their wait on this seat.
    conn = servicesRef.current.protocolResponseConn?.(conn, message) || conn;
    try {
      const request = await validateIncomingRngRequest(conn, message, "Random commitment request");
      const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
      if (localPlayer == null) {
        throw new Error("Local player seat is not assigned");
      }
      const contributionKey = `${request.contextKey}:${Number(localPlayer)}`;
      let contribution = signedRngCommitmentsRef.current.get(contributionKey);
      if (!contribution) {
        const nonceHex = randomAuditHex(32);
        contribution = {
          nonceHex,
          commitmentHex: await rngCommitmentForNonce(nonceHex),
        };
        signedRngCommitmentsRef.current.set(contributionKey, contribution);
      }
      rngCommitNoncesRef.current.set(String(message.requestId || ""), {
        ...request,
        contributionKey,
        localPlayer,
        nonceHex: contribution.nonceHex,
        commitmentHex: contribution.commitmentHex,
      });
      safeSend(conn, {
        type: "rng_commit_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        commitment: await signRngCommitmentEntry({
          matchId: request.matchId,
          seq: request.seq,
          requirementId: request.requirementId,
          requestId: message.requestId,
          requester: request.requesterPlayer,
          player: localPlayer,
          commitmentHex: contribution.commitmentHex,
        }),
      });
    } catch (err) {
      safeSend(conn, {
        type: "rng_commit_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  async function answerRngRevealRequest(conn, message) {
    // Announce the (first) response so peers can close their wait on this seat.
    conn = servicesRef.current.protocolResponseConn?.(conn, message) || conn;
    try {
      const request = await validateIncomingRngRequest(conn, message, "Random reveal request");
      const stored = rngCommitNoncesRef.current.get(String(message.commitRequestId || ""));
      if (!stored) {
        throw new Error("Unknown random commitment request");
      }
      if (
        request.matchId !== stored.matchId
        || Number(request.seq) !== Number(stored.seq)
        || request.requirementId !== stored.requirementId
        || request.requesterPeerId !== stored.requesterPeerId
        || request.contextKey !== stored.contextKey
      ) {
        throw new Error("Random reveal request does not match the commitment request");
      }
      const localPlayer = resolveLocalPlayerIndex(multiplayerRef.current);
      if (localPlayer == null) {
        throw new Error("Local player seat is not assigned");
      }
      const commits = await validateCompleteRngCommitSet(message.commits, request);
      const localCommit = commits.find((entry) => Number(entry?.player) === Number(localPlayer));
      if (!localCommit || String(localCommit.commitmentHex || "") !== String(stored.commitmentHex || "")) {
        throw new Error("Random reveal request does not include the locked local commitment");
      }
      const commitSetHash = await fairRandomCommitSetHash(request.contextKey, commits);
      const lockKey = `${request.contextKey}:${Number(localPlayer)}`;
      const existingLock = rngRevealCommitSetLocksRef.current.get(lockKey);
      if (existingLock && existingLock !== commitSetHash) {
        throw new Error("Random reveal request conflicts with the locked commitment set");
      }
      rngRevealCommitSetLocksRef.current.set(lockKey, commitSetHash);
      lockFairRandomRevealIntent(request);
      const requestPayload = cloneMultiplayerPayload(message);
      await rememberPendingActionIntent(request.actionIntent, {
        requestType: "rng_reveal_request",
        requestId: String(message.requestId || ""),
        requestPayload,
        requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
        responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
        requestedAtMs: Date.now(),
      });
      safeSend(conn, {
        type: "rng_reveal_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        reveal: await signRngRevealEntry({
          matchId: request.matchId,
          seq: request.seq,
          requirementId: request.requirementId,
          requestId: message.requestId,
          commitRequestId: message.commitRequestId,
          requester: request.requesterPlayer,
          player: localPlayer,
          nonceHex: stored.nonceHex,
          commitmentHex: stored.commitmentHex,
        }),
      });
    } catch (err) {
      safeSend(conn, {
        type: "rng_reveal_response",
        protocolVersion: PROTOCOL_VERSION,
        requestId: message.requestId,
        error: toErrorMessage(err),
      });
    }
  }

  async function collectFairRandomReveal(requirement, seq, options = {}) {
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    const localIndex = resolveLocalCryptoPlayerIndex();
    if (localIndex == null) {
      throw new Error("Local player seat is not assigned");
    }
    const matchId = currentAuditMatchId();
    const requirementId = fairRandomRequirementId(requirement);
    if (!requirementId) {
      throw new Error("Fair-random requirement is missing an id");
    }
    const command = options.command || null;
    if (!command || typeof command !== "object") {
      throw new Error("Fair-random request requires a command preview");
    }
    const actorIndex = normalizePlayerIndex(options.actorIndex ?? localIndex);
    if (actorIndex == null || actorIndex !== Number(localIndex)) {
      throw new Error("Fair-random request actor does not match the local player");
    }
    const prevStateHash = String(
      options.prevStateHash ?? auditStateHashRef.current ?? INITIAL_AUDIT_STATE_HASH
    );
    const publicCheckpointHash = String(
      options.publicCheckpointHash || await currentPublicAuditCheckpointHash()
    );
    const actionIntent = options.actionIntent || await signActionIntentForCommand({
      seq,
      actorIndex,
      command,
      prevStateHash,
      preActionPublicCheckpointHash: publicCheckpointHash,
    });
    const requestContext = {
      matchId,
      seq: Number(seq),
      requirementId,
      requesterIndex: Number(localIndex),
      actorIndex,
      prevStateHash,
      publicCheckpointHash,
      command: cloneMultiplayerPayload(command),
      requirement: cloneMultiplayerPayload(requirement),
      actionIntent: cloneMultiplayerPayload(actionIntent),
    };
    const contextKey = await fairRandomRequestContextKey({
      matchId,
      seq,
      requirement,
      requester: localIndex,
      actorIndex,
      prevStateHash,
      publicCheckpointHash,
      command,
    });
    const commits = [];
    const revealRequests = [];
    for (const player of players) {
      if (Number(player.index) === Number(localIndex)) {
        const contributionKey = `${contextKey}:${Number(player.index)}`;
        let contribution = signedRngCommitmentsRef.current.get(contributionKey);
        if (!contribution) {
          const nonceHex = randomAuditHex(32);
          contribution = {
            nonceHex,
            commitmentHex: await rngCommitmentForNonce(nonceHex),
          };
          signedRngCommitmentsRef.current.set(contributionKey, contribution);
        }
        const commitRequestId = makeZiffleRequestId("rng-local");
        commits.push(await signRngCommitmentEntry({
          matchId,
          seq: Number(seq),
          requirementId,
          requestId: commitRequestId,
          requester: localIndex,
          player: Number(player.index),
          commitmentHex: contribution.commitmentHex,
        }));
        revealRequests.push({ player, commitRequestId, local: contribution });
      } else {
        const routePeerId = routePeerIdForPlayer(player);
        const conn = await waitForZiffleRoute(routePeerId);
        const requestId = makeZiffleRequestId("rng-commit");
        const requestedAtMs = Date.now();
        const playerLabel = player.name || `Player ${Number(player.index) + 1}`;
        const waiter = waitForRngCommit(requestId, PROTOCOL_RESPONSE_TIMEOUT_MS, {
          peerIndex: Number(player.index),
          peerName: playerLabel,
          description:
            `${playerLabel} must sign a random commitment before the shared random value can be generated.`,
        });
        setStatus(`Waiting for random commitment from ${playerLabel}`);
        const requestPayload = {
          type: "rng_commit_request",
          protocolVersion: PROTOCOL_VERSION,
          requestId,
          ...requestContext,
        };
        safeSend(conn, requestPayload);
        const commitment = await waitForProtocolResponse(waiter, {
          basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
          targetPlayerIndex: Number(player.index),
          targetPeerId: routePeerId,
          requesterIndex: localIndex,
          requestType: requestPayload.type,
          requestId,
          requestPayload,
          responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
          requestedAtMs,
        });
        if (Number(commitment?.player) !== Number(player.index)) {
          throw new Error("Random commitment response came from the wrong player");
        }
        if (!String(commitment?.commitmentHex || "")) {
          throw new Error("Random commitment response is missing its commitment");
        }
        await verifyRngCommitmentEntry(commitment, {
          matchId,
          seq: Number(seq),
          requirementId,
          requester: localIndex,
          player: Number(player.index),
        });
        commits.push({
          player: Number(commitment.player),
          requester: Number(commitment.requester),
          requestId: String(commitment.requestId || requestId),
          commitmentHex: String(commitment.commitmentHex || ""),
          signature: String(commitment.signature || ""),
        });
        revealRequests.push({ player, commitRequestId: requestId });
      }
    }
    commits.sort((a, b) => Number(a.player) - Number(b.player));
    const commitSetHash = await fairRandomCommitSetHash(contextKey, commits);
    rngRevealCommitSetLocksRef.current.set(`${contextKey}:${Number(localIndex)}`, commitSetHash);
    const reveals = [];
    for (const request of revealRequests) {
      if (request.local) {
        reveals.push(await signRngRevealEntry({
          matchId,
          seq: Number(seq),
          requirementId,
          requestId: makeZiffleRequestId("rng-local-reveal"),
          commitRequestId: request.commitRequestId,
          requester: localIndex,
          player: Number(request.player.index),
          nonceHex: request.local.nonceHex,
          commitmentHex: request.local.commitmentHex,
        }));
      } else {
        const routePeerId = routePeerIdForPlayer(request.player);
        const conn = await waitForZiffleRoute(routePeerId);
        const requestId = makeZiffleRequestId("rng-reveal");
        const requestedAtMs = Date.now();
        const playerLabel = request.player.name || `Player ${Number(request.player.index) + 1}`;
        const waiter = waitForRngReveal(requestId, PROTOCOL_RESPONSE_TIMEOUT_MS, {
          peerIndex: Number(request.player.index),
          peerName: playerLabel,
          description:
            `${playerLabel} must reveal their committed random contribution before the game can use it.`,
        });
        setStatus(`Waiting for random reveal from ${playerLabel}`);
        const requestPayload = {
          type: "rng_reveal_request",
          protocolVersion: PROTOCOL_VERSION,
          requestId,
          commitRequestId: request.commitRequestId,
          ...requestContext,
          commits,
        };
        safeSend(conn, requestPayload);
        const reveal = await waitForProtocolResponse(waiter, {
          basisSequence: Number(multiplayerRef.current.lastAppliedSequence || 0),
          targetPlayerIndex: Number(request.player.index),
          targetPeerId: routePeerId,
          requesterIndex: localIndex,
          requestType: requestPayload.type,
          requestId,
          requestPayload,
          responseTimeoutMs: PROTOCOL_RESPONSE_TIMEOUT_MS,
          requestedAtMs,
        });
        if (Number(reveal?.player) !== Number(request.player.index)) {
          throw new Error("Random reveal response came from the wrong player");
        }
        await verifyRngRevealEntry(reveal, {
          matchId,
          seq: Number(seq),
          requirementId,
          requester: localIndex,
          player: Number(request.player.index),
        });
        reveals.push({
          player: Number(reveal.player),
          requester: Number(reveal.requester),
          requestId: String(reveal.requestId || requestId),
          commitRequestId: String(reveal.commitRequestId || request.commitRequestId),
          nonceHex: String(reveal.nonceHex || ""),
          commitmentHex: String(reveal.commitmentHex || ""),
          signature: String(reveal.signature || ""),
        });
      }
    }
    reveals.sort((a, b) => Number(a.player) - Number(b.player));
    for (const reveal of reveals) {
      const expected = commits.find((commit) => Number(commit.player) === Number(reveal.player));
      const actual = await rngCommitmentForNonce(reveal.nonceHex);
      if (!expected || actual !== expected.commitmentHex || actual !== reveal.commitmentHex) {
        throw new Error("Random reveal does not match its commitment");
      }
    }
    const combinedSeedHex = await fairRandomCombinedSeedHex({
      matchId,
      seq: Number(seq),
      requirementId,
      commits,
      reveals,
    });
    return {
      type: "commit_reveal_random",
      requirementId,
      count: Number(requirement.count || 1),
      commits,
      reveals,
      combinedSeedHex,
    };
  }

  const buildLocalRngRevealsForRequirements = useCallback(async (cryptoRequirements = [], seq = 0, options = {}) => {
    const fairRandomRequirements = (cryptoRequirements || []).filter(
      (requirement) => String(requirement?.type || requirement?.requirement_type || "") === "fair_random"
    );
    const reveals = [];
    for (const requirement of fairRandomRequirements) {
      reveals.push(await collectFairRandomReveal(requirement, seq, options));
    }
    return reveals;
  }, [
    currentAuditMatchId,
    currentPublicAuditCheckpointHash,
    makeZiffleRequestId,
    resolveLocalCryptoPlayerIndex,
    setStatus,
    waitForRngCommit,
    waitForRngReveal,
  ]);

  function handIdsForRevealKey(player) {
    if (!player || typeof player !== "object") return null;
    if (Array.isArray(player.hand)) {
      return player.hand.map((id) => Number(id)).filter(Number.isFinite);
    }
    if (Array.isArray(player.hand_cards)) {
      return player.hand_cards
        .map((card) => Number(card?.id ?? card?.object_id ?? card?.objectId))
        .filter(Number.isFinite);
    }
    return null;
  }

	  function localZiffleHandRevealKey(stateLike, localIndex, matchId) {
	    const players = Array.isArray(stateLike?.players) ? stateLike.players : [];
	    const player = players.find((entry) => Number(entry?.id) === Number(localIndex));
	    const ids = handIdsForRevealKey(player);
	    if (!ids) return "";
	    const objects = new Map(
	      (Array.isArray(stateLike?.objects) ? stateLike.objects : [])
	        .map((object) => [Number(object?.id), object])
	    );
	    const identities = ids.map((id) => {
	      const object = objects.get(Number(id));
	      const hidden = object?.hiddenCard || object?.hidden_card || null;
	      if (!hidden) return String(Number(id));
	      return [
	        Number(id),
	        hidden.owner == null ? "" : Number(hidden.owner),
	        hidden.slot == null ? "" : Number(hidden.slot),
	        String(hidden.commitment || ""),
	        hidden.publicSlot == null && hidden.public_slot == null
	          ? ""
	          : Number(hidden.publicSlot ?? hidden.public_slot),
	        String(hidden.publicCommitment || hidden.public_commitment || ""),
	      ].join(":");
	    });
	    identities.sort();
	    return [
	      String(matchId || currentAuditMatchId() || ""),
	      Number(localIndex),
	      identities.join(","),
	    ].join("|");
	  }

	  function localZiffleHandRevealObjectIdsKey(stateLike, localIndex, matchId) {
	    const players = Array.isArray(stateLike?.players) ? stateLike.players : [];
	    const player = players.find((entry) => Number(entry?.id) === Number(localIndex));
	    const ids = handIdsForRevealKey(player);
	    if (!ids) return "";
	    ids.sort((left, right) => left - right);
	    return [
	      String(matchId || currentAuditMatchId() || ""),
	      Number(localIndex),
	      ids.join(","),
	    ].join("|");
	  }

  function isInspectorOnlyViewedCards(viewedCards) {
    return Boolean(viewedCards?.inspector_only || viewedCards?.inspectorOnly);
  }

  function viewedCardsStateHint(...states) {
    for (const candidate of states) {
      if (candidate?.viewed_cards && !isInspectorOnlyViewedCards(candidate.viewed_cards)) {
        return candidate;
      }
    }
    return states.find(Boolean) || null;
  }

  function isHiddenViewedCardName(name) {
    return String(name || "").trim().toLowerCase() === "hidden card";
  }

  async function hydrateViewedCardsFromLiveObjects(viewedCards, currentGame) {
    if (
      !viewedCards
      || !Array.isArray(viewedCards.cards)
      || typeof currentGame?.objectDetails !== "function"
    ) {
      return viewedCards;
    }

    let changed = false;
    const cards = [];
    for (const card of viewedCards.cards) {
      const objectId = Number(card?.id);
      if (!Number.isSafeInteger(objectId) || objectId < 0) {
        cards.push(card);
        continue;
      }

      let details = null;
      try {
        details = await currentGame.objectDetails(wasmObjectIdArg(objectId));
      } catch {
        cards.push(card);
        continue;
      }

      const detailName = String(details?.name || "").trim();
      const cardName = String(card?.name || "").trim();
      let nextCard = card;
      if (
        detailName
        && !isHiddenViewedCardName(detailName)
        && (isHiddenViewedCardName(cardName) || cardName !== detailName)
      ) {
        nextCard = { ...nextCard, name: detailName };
        changed = true;
      }
      if (details?.oracle_text && details.oracle_text !== nextCard?.oracle_text) {
        nextCard = { ...nextCard, oracle_text: details.oracle_text };
        changed = true;
      }
      if (
        details?.stable_id != null
        && Number(details.stable_id) !== Number(nextCard?.stable_id)
      ) {
        nextCard = { ...nextCard, stable_id: details.stable_id };
        changed = true;
      }
      cards.push(nextCard);
    }

    return changed ? { ...viewedCards, cards } : viewedCards;
  }

  async function preserveViewedCardsFromHint(nextState, stateHint = null, currentGame = null) {
    if (!nextState) {
      return nextState;
    }
    if (nextState.viewed_cards) {
      const viewedCards = await hydrateViewedCardsFromLiveObjects(
        nextState.viewed_cards,
        currentGame
      );
      return viewedCards === nextState.viewed_cards
        ? nextState
        : { ...nextState, viewed_cards: viewedCards };
    }
    if (!stateHint?.viewed_cards || isInspectorOnlyViewedCards(stateHint.viewed_cards)) {
      return nextState;
    }
    const viewedCards = await hydrateViewedCardsFromLiveObjects(
      stateHint.viewed_cards,
      currentGame
    );
    return {
      ...nextState,
      viewed_cards: viewedCards,
    };
  }

  async function ziffleRevealTokenOptionsForLocalHandReveal({
    ceremony,
    positions,
    options = {},
    localIndex,
  } = {}) {
    const sanitized = { includeCeremonyInRevealRequest: true };
    const normalizedLocalIndex = Number(localIndex);
    if (
      !options?.command
      || !Number.isSafeInteger(normalizedLocalIndex)
      || Number(options.actorIndex) !== normalizedLocalIndex
    ) {
      return sanitized;
    }
    const requirements = Array.isArray(options.requirements) ? options.requirements : [];
    if (requirements.length === 0) return sanitized;
    const requester = normalizedLocalIndex;
    const owner = normalizedLocalIndex;
    if (
      ziffleRequirementsAuthorizeRevealPositions(
        requirements,
        requester,
        owner,
        positions,
        ceremony,
        options.seq
      )
    ) {
      return {
        ...options,
        includeCeremonyInRevealRequest: true,
      };
    }
    try {
      const authorizedByMetadata = await ziffleRequirementsAuthorizeRevealPositionsByMetadata(
        requirements,
        requester,
        owner,
        positions,
        ceremony
      );
      if (authorizedByMetadata) {
        return {
          ...options,
          includeCeremonyInRevealRequest: true,
        };
      }
    } catch {
      // Fall through to visible-state authorization.
    }
    return sanitized;
  }

  async function revealLocalZiffleHand(payload = matchStartPayloadRef.current, options = {}) {
    const previousReveal = localZiffleRevealInFlightRef.current;
    const revealPromise = (async () => {
      if (previousReveal) {
        await previousReveal;
      }
      return revealLocalZiffleHandInner(payload, options);
    })();
    localZiffleRevealInFlightRef.current = revealPromise;
    try {
      return await revealPromise;
    } finally {
      if (localZiffleRevealInFlightRef.current === revealPromise) {
        localZiffleRevealInFlightRef.current = null;
      }
    }
  }

  // After a foreign checkpoint import: another player's hidden cards this seat
  // may look at (a card it exiled face down with Gonti or Praetor's Grasp, a
  // face-down permanent it controls) lost their identities with the redacted
  // checkpoint. Reopen each from its immutable ziffle anchor with everyone's
  // reveal tokens; the responders authorize them from their own view rules
  // (authorizedZiffleRevealPositionsForOwner with this seat as requester).
  // Best effort: a card that cannot be reopened stays closed.
  async function revealViewableForeignHiddenCards(payload, checkpoint, localIndex) {
    const currentGame = gameRef.current;
    if (typeof currentGame?.hiddenObjectViewableBy !== "function") return;
    const groups = new Map();
    for (const object of checkpoint?.objects || []) {
      const hidden = object?.hiddenCard || object?.hidden_card || null;
      const zone = String(object?.zone || hidden?.zone || "");
      const objectId = Number(object?.id);
      if (!hidden || Number(hidden.owner) === Number(localIndex)) continue;
      if (["library", "outside_game", "hand"].includes(zone)) continue;
      if (!Number.isSafeInteger(objectId) || objectId <= 0) continue;
      if (!isHiddenViewedCardName(object?.name)) continue;
      try {
        if (!await currentGame.hiddenObjectViewableBy(wasmObjectIdArg(objectId), Number(localIndex))) continue;
      } catch {
        continue;
      }
      const owner = Number(hidden.owner);
      const origin = ziffleOriginAnchorFromMetadata(hidden);
      const publicCommitment = String(hidden.publicCommitment ?? hidden.public_commitment ?? "");
      const hiddenCommitment = String(hidden.commitment || "");
      const anchor = origin
        ? { position: origin.originPosition, positionCommitment: origin.originPositionCommitment }
        : ziffleDeckHashFromCommitment(publicCommitment)
          ? {
            position: hidden.publicSlot ?? hidden.public_slot ?? zifflePositionFromCommitment(publicCommitment),
            positionCommitment: publicCommitment,
          }
          : ziffleDeckHashFromCommitment(hiddenCommitment)
            ? { position: hidden.slot, positionCommitment: hiddenCommitment }
            : null;
      const position = Number(anchor?.position);
      if (!anchor || !Number.isSafeInteger(position) || position < 0) continue;
      const ceremony = ziffleCeremonyForOwner(owner, { payload, commitment: anchor.positionCommitment });
      if (!ceremony) continue;
      const groupKey = [owner, String(ceremony.context || ""), String(ceremony.deckHash || "")].join(":");
      if (!groups.has(groupKey)) groups.set(groupKey, { owner, ceremony, entries: [] });
      groups.get(groupKey).entries.push({ objectId, position, positionCommitment: anchor.positionCommitment });
    }
    for (const { owner, ceremony, entries } of groups.values()) {
      const manifest = privateDeckManifestForOwner(owner, payload?.auditMatchId);
      if (!manifest) continue;
      try {
        const positions = [...new Set(entries.map((entry) => entry.position))];
        const revealOptions = { includeCeremonyInRevealRequest: true };
        const tokens = await collectZiffleRevealTokensBatch(ceremony, positions, revealOptions);
        const reveals = await currentGame.ziffleRevealCards({
          deckCount: Number(ceremony.deckCount),
          context: String(ceremony.context || ""),
          keyContext: ziffleKeyContextForCeremony(ceremony),
          keys: cloneMultiplayerPayload(ceremony.keys || []),
          steps: cloneMultiplayerPayload(ceremony.steps || []),
          ...ziffleInputDeckFields(ceremony),
          cardPositions: positions,
          tokens,
        });
        const revealByPosition = new Map((Array.isArray(reveals) ? reveals : []).map((reveal) => [
          Number(reveal.cardPosition),
          Number(reveal.originalSlot),
        ]));
        for (const entry of entries) {
          const resolved = await resolveCommittedZiffleRevealSlot({
            owner,
            ceremony,
            shuffleOriginalSlot: revealByPosition.get(entry.position),
            shuffleOriginalSlotIsVerified: true,
            position: entry.position,
            objectId: entry.objectId,
            manifest,
            payload,
            options: revealOptions,
          });
          if (!resolved) continue;
          const originalSlot = Number(resolved.slot);
          const secret = (manifest.slotSecrets || []).find((candidate) => Number(candidate.slot) === originalSlot);
          if (!secret) continue;
          await currentGame.revealHiddenPosition({
            owner,
            objectId: entry.objectId,
            position: entry.position,
            originalSlot,
            cardName: String(resolved.card || secret.card || ""),
            positionCommitment: entry.positionCommitment,
            commitment: secret.commitment,
          });
        }
      } catch (err) {
        console.warn("[ironsmith] could not reopen a viewable opponent card after resync", err);
      }
    }
  }

	  async function revealLocalZiffleHandInner(payload = matchStartPayloadRef.current, options = {}) {
    if (!payload?.ziffleCeremonies?.length && liveZiffleCeremoniesRef.current.size === 0) return;
    const currentGame = gameRef.current;
    if (
      !currentGame
      || typeof currentGame.exportSyncCheckpoint !== "function"
      || typeof currentGame.exportHiddenCardOpening !== "function"
      || typeof currentGame.ziffleRevealCard !== "function"
      || typeof currentGame.ziffleRevealCards !== "function"
      || typeof currentGame.revealHiddenPosition !== "function"
    ) {
      return;
    }
    const localIndex = resolveLocalPlayerIndex(multiplayerRef.current);
    if (localIndex == null) return;
    const manifest = privateDeckManifestForOwner(localIndex, payload.auditMatchId);
	    if (!manifest) {
	      throw new Error("Missing private deck manifest for local ziffle hand reveal");
	    }

	    if (
	      options.skipIfHandUnchanged
	      && (options.requirements || []).length === 0
	      && options.command?.type === "priority_action"
	      && ["pass_priority", "test_priority_action"].includes(
	        String(options.command?.action_ref?.kind || options.command?.actionRef?.kind || "")
	      )
	    ) {
	      return;
	    }

	    if (options.skipIfHandUnchanged) {
	      const quickKey = localZiffleHandRevealObjectIdsKey(
	        options.stateHint || stateRef.current,
	        localIndex,
	        payload.auditMatchId,
	      );
	      if (quickKey && quickKey === ziffleHandRevealQuickKeyRef.current) {
	        return;
	      }
	    }

	    const checkpoint = await currentGame.exportSyncCheckpoint();
	    const localPlayer = (checkpoint.players || []).find(
	      (player) => Number(player.id) === Number(localIndex)
	    );
	    const checkpointKey = localZiffleHandRevealKey(checkpoint, localIndex, payload.auditMatchId);
	    const checkpointQuickKey = localZiffleHandRevealObjectIdsKey(
	      checkpoint,
	      localIndex,
	      payload.auditMatchId
	    );
	    if (
	      options.skipIfHandUnchanged
	      && checkpointKey
	      && checkpointKey === ziffleHandRevealKeyRef.current
	    ) {
	      if (checkpointQuickKey) ziffleHandRevealQuickKeyRef.current = checkpointQuickKey;
	      return;
	    }
    const handIds = new Set((localPlayer?.hand || []).map((id) => Number(id)));
    if (options.includeOwnerViewableHidden && typeof currentGame.hiddenObjectViewableBy === "function") {
      // After a foreign checkpoint import: this seat's own hidden cards outside
      // the hand (face-down permanents, face-down exile it may look at) whose
      // identities the exporter's redacted checkpoint could not carry. Library
      // cards stay closed; the responders' owner visible-state rule gates the rest.
      handIds.clear();
      for (const object of checkpoint.objects || []) {
        const hidden = object?.hiddenCard || object?.hidden_card || null;
        const zone = String(object?.zone || hidden?.zone || "");
        const objectId = Number(object?.id);
        if (!hidden || Number(hidden.owner) !== Number(localIndex)) continue;
        if (["library", "outside_game", "hand"].includes(zone)) continue;
        if (!Number.isSafeInteger(objectId) || objectId <= 0) continue;
        try {
          if (await currentGame.hiddenObjectViewableBy(wasmObjectIdArg(objectId), Number(localIndex))) {
            handIds.add(objectId);
          }
        } catch {
          // Not viewable by this seat: leave it closed.
        }
      }
      await revealViewableForeignHiddenCards(payload, checkpoint, localIndex);
      if (handIds.size === 0) return;
    }
	    if (handIds.size === 0) {
	      if (checkpointKey) ziffleHandRevealKeyRef.current = checkpointKey;
	      if (checkpointQuickKey) ziffleHandRevealQuickKeyRef.current = checkpointQuickKey;
	      return;
	    }
    const objects = new Map((checkpoint.objects || []).map((object) => [Number(object.id), object]));
    let changed = false;
    const ziffleGroups = new Map();
    for (const objectId of handIds) {
      const object = objects.get(objectId);
      const hidden = object?.hiddenCard;
      let exported = null;
      try {
        exported = await currentGame.exportHiddenCardOpening(wasmObjectIdArg(objectId));
      } catch {
        exported = null;
      }
      if (exported && Number(exported.owner) !== Number(localIndex)) {
        exported = null;
      }
      if (!hidden && !exported) continue;
      const exportedCommitment = String(exported?.commitment || "");
      const exportedZiffleDeckHash = ziffleDeckHashFromCommitment(exportedCommitment);
      const hiddenCommitment = String(hidden?.commitment || "");
      const hiddenZiffleDeckHash = ziffleDeckHashFromCommitment(hiddenCommitment);
	      const publicSlot =
	        hidden?.publicSlot
	        ?? hidden?.public_slot
	        ?? exported?.publicSlot
	        ?? exported?.public_slot
	        ?? null;
	      const publicCommitment = String(
	        hidden?.publicCommitment
	        || hidden?.public_commitment
	        || exported?.publicCommitment
	        || exported?.public_commitment
	        || ""
	      );
	      const publicZiffleDeckHash = ziffleDeckHashFromCommitment(publicCommitment);
	      const publicPositionFromCommitment = zifflePositionFromCommitment(publicCommitment);
		      const orderedPosition =
		        zifflePositionForObjectId(localIndex, objectId, { payload })
		        || zifflePositionForOriginalSlot(localIndex, exported?.slot, { payload });
		      const ziffleContext = orderedPosition?.ziffleContext || "";
		      const identityPosition = ziffleIdentityPositionFromSources(hidden, exported);
		      const knownPosition =
		        (publicZiffleDeckHash
		          ? (publicSlot != null ? Number(publicSlot) : publicPositionFromCommitment)
		          : hiddenZiffleDeckHash
		            ? Number(hidden?.slot)
		            : exportedZiffleDeckHash
		              ? Number(exported?.slot)
		              : orderedPosition?.position ?? null)
		        ?? identityPosition?.position;
		      const knownPositionCommitment =
		        (publicZiffleDeckHash
		          ? publicCommitment
		          : hiddenZiffleDeckHash
		            ? hiddenCommitment
		            : exportedZiffleDeckHash
		              ? exportedCommitment
		              : orderedPosition?.positionCommitment || "")
		        || identityPosition?.positionCommitment;
	      if (exported && !exportedZiffleDeckHash && !knownPositionCommitment) {
        let opening = localRevealedOpeningForExport(exported);
        if (opening && !knownPositionCommitment && !openingHasZifflePosition(opening)) {
          opening = cloneMultiplayerPayload(opening);
          delete opening.position;
          delete opening.positionCommitment;
          delete opening.ziffleReveal;
          delete opening.ziffleProof;
          delete opening.positionOpeningProof;
        }
        if (!opening) {
          const built = await buildDeckSlotOpeningForExport({
            manifest,
            preferredSlot: exported.slot,
            card: exported.card,
            exportedCommitment,
            label: "Local hidden card opening",
          });
          opening = built.opening;
        }
        rememberLocalRevealedOpening(opening, {
          objectId,
          position: opening.position ?? knownPosition,
          positionCommitment: opening.positionCommitment || knownPositionCommitment,
          matchId: payload.auditMatchId,
        });
        if (knownPosition != null) {
          rememberZiffleOpeningPosition(localIndex, opening.slot, knownPosition);
        }
        if (typeof currentGame.revealHiddenObject === "function") {
          try {
            await currentGame.revealHiddenObject({
              objectId,
              slot: Number(opening.slot),
              cardName: String(opening.card || exported.card || ""),
              commitment: String(opening.commitment || exportedCommitment || "") || undefined,
              recomputeDecision: false,
            });
            changed = true;
          } catch (err) {
            const message = String(err?.message || err || "");
            if (!message.includes("not present") && !message.includes("not a hidden")) {
              throw err;
            }
          }
        }
        continue;
      }
      if (!knownPositionCommitment) continue;
      const position = Number(knownPosition);
      const positionCommitment = knownPositionCommitment;
      const ceremony = ziffleCeremonyForOwner(localIndex, {
        payload,
        commitment: positionCommitment,
        context: ziffleContext,
      });
      if (!ceremony) continue;
      const groupKey = [
        Number(localIndex),
        String(ceremony.context || ""),
        String(ceremony.deckHash || ""),
      ].join(":");
      if (!ziffleGroups.has(groupKey)) {
        ziffleGroups.set(groupKey, {
          ceremony,
          entries: [],
        });
      }
      ziffleGroups.get(groupKey).entries.push({
        objectId,
        position,
        positionCommitment,
        exported,
      });
	    }

	    for (const { ceremony, entries } of ziffleGroups.values()) {
	      if (ziffleCeremonyHasObjectOrder(ceremony)) {
	        for (const entry of entries) {
	          const position = Number(entry.position);
	          const positionCommitment =
	            entry.positionCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
	          const revealTokenOptions = await ziffleRevealTokenOptionsForLocalHandReveal({
	            ceremony,
	            positions: [position],
	            options,
	            localIndex,
	          });
	          const {
	            resolvedRevealSlot,
	            shuffleOriginalSlot,
	          } = await resolveCommittedSlotForZifflePosition({
	            owner: localIndex,
	            ceremony,
	            position,
	            card: entry.exported?.card || "",
	            objectId: entry.objectId,
	            manifest,
	            payload,
	            options: revealTokenOptions,
	          });
	          if (!resolvedRevealSlot) {
	            throw new Error(
	              `Ziffle hand reveal could not resolve committed slot `
	              + `(owner ${Number(localIndex) + 1}, position ${position}, `
	              + `shuffle slot ${shuffleOriginalSlot ?? "none"}, card ${String(entry.exported?.card || "")})`
	            );
	          }
	          const {
	            opening,
	            openingWithPosition,
	            originalSlot,
	          } = await buildOpeningFromResolvedCommittedSlot({
	            manifest,
	            resolvedRevealSlot,
	            fallbackObjectId: entry.objectId,
	            position,
	            positionCommitment,
	            ceremony,
	          });
	          const revealObjectId = Number(entry.objectId);
	          if (!Number.isSafeInteger(revealObjectId) || revealObjectId < 0) {
	            throw new Error("Local ziffle hand reveal is missing the current hand object id");
	          }
		          await currentGame.revealHiddenPosition({
		            owner: Number(localIndex),
		            objectId: revealObjectId,
		            position,
	            originalSlot,
	            cardName: opening.card,
	            positionCommitment,
		            commitment: opening.commitment,
		          }).catch((err) => {
		            if (!entry.exported) throw err;
		          });
		          const sanitizedOpeningWithPosition = await sanitizeObjectBoundOpening({
		            ...openingWithPosition,
		            objectId: revealObjectId,
		          });
		          rememberLocalRevealedOpening(
		            sanitizedOpeningWithPosition,
		            {
		              objectId: Number(revealObjectId),
		              position,
	              positionCommitment,
	              ziffleContext: sanitizedOpeningWithPosition.ziffleContext,
	              matchId: payload.auditMatchId,
	            }
	          );
	          rememberZiffleOpeningPosition(localIndex, originalSlot, position);
	          changed = true;
	        }
	        continue;
	      }
	      const positions = [...new Set(entries.map((entry) => Number(entry.position)))];
	      const revealTokenOptions = await ziffleRevealTokenOptionsForLocalHandReveal({
	        ceremony,
	        positions,
	        options,
	        localIndex,
	      });
	      const tokens = await collectZiffleRevealTokensBatch(ceremony, positions, revealTokenOptions);
      const reveals = await currentGame.ziffleRevealCards({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(ceremony),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        ...ziffleInputDeckFields(ceremony),
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
	        const position = Number(entry.position);
	        const shuffleOriginalSlot = revealByPosition.get(position);
	        if (!Number.isSafeInteger(shuffleOriginalSlot) || shuffleOriginalSlot < 0) {
	          throw new Error(`Missing ziffle reveal for position ${position}`);
	        }
	        const resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	          owner: localIndex,
	          ceremony,
	          shuffleOriginalSlot,
	          shuffleOriginalSlotIsVerified: true,
	          position,
		          card: "",
		          objectId: entry.objectId,
		          manifest,
		          payload,
              options: revealTokenOptions,
		        });
		        if (!resolvedRevealSlot) {
		          let resolveDebug = null;
		          try {
		            const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
		            const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
		            const ids = [
		              entry.objectId,
		              beforeOrder[Number(shuffleOriginalSlot)],
		              afterOrder[Number(position)],
		            ].map((id) => Number(id)).filter((id, index, list) =>
		              Number.isSafeInteger(id) && id >= 0 && list.indexOf(id) === index
		            );
		            const checkpoint = await currentGame.exportSyncCheckpoint?.();
		            const objectsById = new Map((checkpoint?.objects || []).map((object) => [
		              Number(object.id),
		              object,
		            ]));
		            resolveDebug = {
		              beforeObjectId: beforeOrder[Number(shuffleOriginalSlot)] ?? null,
		              afterObjectId: afterOrder[Number(position)] ?? null,
		              entryObjectId: entry.objectId,
		              exported: entry.exported
		                ? {
		                  slot: entry.exported.slot,
		                  card: entry.exported.card,
		                  commitment: String(entry.exported.commitment || "").slice(0, 32),
		                  publicSlot: entry.exported.publicSlot ?? entry.exported.public_slot ?? null,
		                  publicCommitment: String(
		                    entry.exported.publicCommitment || entry.exported.public_commitment || ""
		                  ).slice(0, 48),
		                }
		                : null,
		              candidates: ids.map((id) => {
		                const object = objectsById.get(id) || {};
		                const hidden = object.hiddenCard || object.hidden_card || {};
		                return {
		                  id,
		                  name: object.name || null,
		                  zone: object.zone || null,
		                  hiddenSlot: hidden.slot ?? null,
		                  hiddenCommitment: String(hidden.commitment || "").slice(0, 48),
		                  publicSlot: hidden.publicSlot ?? hidden.public_slot ?? null,
		                  publicCommitment: String(
		                    hidden.publicCommitment || hidden.public_commitment || ""
		                  ).slice(0, 48),
		                };
		              }),
		            };
		          } catch {
		            resolveDebug = null;
		          }
		          throw new Error(
		            `Ziffle hand reveal could not resolve committed slot `
		            + `(owner ${Number(localIndex) + 1}, position ${position}, `
		            + `shuffle slot ${shuffleOriginalSlot}, card ${String(entry.exported?.card || "")}`
		            + `${resolveDebug ? `, debug ${JSON.stringify(resolveDebug)}` : ""})`
		          );
		        }
	        const originalSlot = Number(resolvedRevealSlot.slot);
	        const secret = (manifest.slotSecrets || []).find(
	          (candidate) => Number(candidate.slot) === originalSlot
	        );
	        if (!secret) {
	          throw new Error(`Missing private deck opening for ziffle slot ${originalSlot}`);
        }
        const opening = await buildDeckSlotOpening({
	          manifest,
	          slot: originalSlot,
	          card: resolvedRevealSlot?.card || secret.card,
	        });
        const positionCommitment =
          entry.positionCommitment || ziffleRuntimeCommitment(ceremony.deckHash, position);
        const revealObjectId = Number(entry.objectId);
        if (!Number.isSafeInteger(revealObjectId) || revealObjectId < 0) {
          throw new Error("Local ziffle hand reveal is missing the current hand object id");
        }
        await currentGame.revealHiddenPosition({
          owner: Number(localIndex),
          objectId: revealObjectId,
          position,
          originalSlot,
          cardName: opening.card,
          positionCommitment,
          commitment: opening.commitment,
        }).catch((err) => {
          if (!entry.exported) throw err;
        });
		        const openingWithPosition = {
		          ...opening,
		          ...(revealObjectId != null ? { objectId: Number(revealObjectId) } : {}),
		          ...(resolvedRevealSlot?.shuffleObjectId != null || resolvedRevealSlot?.objectId != null
		            ? { shuffleObjectId: Number(resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId) }
		            : {}),
			          position,
			          positionCommitment,
			          ziffleContext: ziffleContextFromCeremony(ceremony),
		        };
        if (!ziffleCeremonyHasObjectOrder(ceremony)) {
          openingWithPosition.ziffleReveal = buildZiffleOpeningProof({
	            opening: {
	              ...opening,
	              position,
	              positionCommitment,
	              ziffleContext: ziffleContextFromCeremony(ceremony),
	            },
            ceremony,
            position,
            originalSlot,
            shuffleOriginalSlot,
            positionCommitment,
            tokens: ziffleTokensForPosition(tokens, position),
            compact: true,
          });
        }
	        const sanitizedOpeningWithPosition = await sanitizeObjectBoundOpening(openingWithPosition);
	        rememberLocalRevealedOpening(
	          sanitizedOpeningWithPosition,
	          {
	            objectId: revealObjectId,
	            position,
	            positionCommitment,
	            ziffleContext: sanitizedOpeningWithPosition.ziffleContext,
	            matchId: payload.auditMatchId,
	          }
        );
        rememberZiffleOpeningPosition(localIndex, originalSlot, position);
        changed = true;
      }
    }
    if (changed) {
      await currentGame.setPerspective(localIndex);
      if (options.updateState !== false) {
        const nextState = await preserveViewedCardsFromHint(
          await currentGame.uiState(),
          options.stateHint,
          currentGame,
        );
        stateRef.current = nextState;
        setState(nextState);
      }
    }
	    if (checkpointKey) {
	      ziffleHandRevealKeyRef.current = checkpointKey;
	    }
	    if (checkpointQuickKey) {
	      ziffleHandRevealQuickKeyRef.current = checkpointQuickKey;
	    }
	  }

  async function buildZiffleCeremoniesForPayload(payload, players) {
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
      throw new Error("Ziffle mental-poker backend is not available");
    }
    const keys = zifflePublicKeysForPlayers(players);
    const ceremonies = [];
    const runtimeHiddenDeckManifests = [];
    const orderedPlayers = reindexPlayers(players);
    for (const ownerPlayer of orderedPlayers) {
      const manifest = publicDeckManifest(ownerPlayer.deckAuditManifest);
      const deckCount = Number(manifest?.deckCount || ownerPlayer.deckCount || 0);
      const context = String(payload.auditMatchId || payload.lobbyId || payload.hostPeerId || "");
      const keyContext = context;
      const steps = [];
      for (const shuffler of orderedPlayers) {
        const request = {
          deckCount,
          context,
          keyContext,
          keys,
          steps: cloneMultiplayerPayload(steps),
          owner: Number(ownerPlayer.index || 0),
          zone: "library",
          shuffler: Number(shuffler.index || 0),
        };
        let step;
        if (shuffler.peerId === multiplayerRef.current.localPeerId) {
          step = await buildLocalZiffleShuffleStep(request);
        } else {
          const conn = clientConnectionsRef.current.get(shuffler.peerId);
          if (!conn || conn.open === false) {
            throw new Error(`Cannot request ziffle shuffle step from ${shuffler.name}`);
          }
          const requestId = makeZiffleRequestId("ziffle-shuffle");
          const shufflerLabel = shuffler.name || `Player ${Number(shuffler.index) + 1}`;
          const waiter = waitForZiffleShuffleStep(requestId, 60000, {
            peerIndex: Number(shuffler.index),
            peerName: shufflerLabel,
            description:
              `${shufflerLabel} must provide verifiable shuffle material before the match can start.`,
          });
          safeSend(conn, {
            type: "ziffle_shuffle_step_request",
            protocolVersion: PROTOCOL_VERSION,
            requestId,
            request,
          });
          step = await waiter;
        }
        steps.push({
          shuffler: Number(step.shuffler ?? shuffler.index ?? 0),
          deckHex: String(step.deckHex || ""),
          proofHex: String(step.proofHex || ""),
          signature: String(step.signature || ""),
        });
      }
      const verified = await currentGame.ziffleVerifyShuffle({
        deckCount,
        context,
        keyContext,
        keys,
        steps,
      });
      const ceremony = {
        owner: Number(ownerPlayer.index || 0),
        deckCount,
        context,
        keyContext,
        keys,
        steps,
        deckHash: String(verified.deckHash || ""),
      };
      ceremonies.push(ceremony);
      runtimeHiddenDeckManifests.push(runtimeManifestForZiffleCeremony(manifest, ceremony));
    }
    payload.ziffleKeys = keys;
    payload.ziffleCeremonies = ceremonies;
    payload.runtimeHiddenDeckManifests = runtimeHiddenDeckManifests;
    payload.decks = orderedPlayers.map(() => []);
    return payload;
  }

  async function verifyZiffleCeremoniesForPayload(payload) {
    const ceremonies = Array.isArray(payload?.ziffleCeremonies) ? payload.ziffleCeremonies : [];
    const playerCount = Array.isArray(payload?.players) ? payload.players.length : 0;
    if (!isCurrentAuditPlayerCount(playerCount)) {
      throw new Error("Current protocol requires 2, 3, or 4 players");
    }
    if (ceremonies.length !== playerCount) {
      throw new Error("Current protocol requires one ziffle ceremony per player deck");
    }
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
      throw new Error("Ziffle mental-poker backend is not available");
    }
    for (const ceremony of ceremonies) {
      await assertZiffleCeremonyStepsSigned(ceremony, payload);
      const verified = await currentGame.ziffleVerifyShuffle({
        deckCount: Number(ceremony.deckCount),
        context: String(ceremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(ceremony),
        keys: cloneMultiplayerPayload(ceremony.keys || []),
        steps: cloneMultiplayerPayload(ceremony.steps || []),
        ...ziffleInputDeckFields(ceremony),
      });
      if (String(verified.deckHash || "") !== String(ceremony.deckHash || "")) {
        throw new Error(
          `Ziffle shuffle proof mismatch for player ${Number(ceremony.owner) + 1}`
        );
      }
    }
  }

	  const applyMatchStart = useCallback(
	    async (payload, options = {}) => {
      assertRuntimeVersion(payload);
	      let currentGame = gameRef.current;
	      if (!currentGame || typeof currentGame.startMatch !== "function") {
	        throw new Error("Game engine is not ready for multiplayer");
	      }
	      const securityMode = matchPayloadSecurityMode(payload, sessionSecurityMode(multiplayerRef.current));
	      const verifiedMode = isVerifiedMultiplayerSecurityMode(securityMode);

	      const currentSession = multiplayerRef.current;
	      const localAuditPublicKey = auditPublicKeyRef.current || "";
      const localEncryptionPublicKey = auditEncryptionPublicKeyRef.current || "";
      const localEntry = findLocalMatchPlayer(
        payload.players,
        currentSession,
        localAuditPublicKey,
        localEncryptionPublicKey
      );

	      if (!localEntry) {
	        throw new Error("Local player is missing from the match payload");
	      }
	      // The lobby's advertised mode binds the match: a host cannot start a
	      // Verified lobby (or tournament) as a Trusted match.
	      // The pin is the mode this seat consented to when it joined (see
	      // pinnedSessionSecurityMode); host lobby_state cannot move it.
	      const consentedSecurityMode = pinnedSessionSecurityMode(currentSession, matchStartPayloadRef.current)
	        || (currentSession.securityMode === MULTIPLAYER_SECURITY_VERIFIED ? MULTIPLAYER_SECURITY_VERIFIED : "");
	      if (consentedSecurityMode && consentedSecurityMode !== securityMode) {
	        throw new Error(
	          isVerifiedMultiplayerSecurityMode(consentedSecurityMode)
	            ? "Host tried to start this Verified lobby as a Trusted match"
	            : `Host tried to start this ${consentedSecurityMode} lobby as a ${securityMode} match`
	        );
	      }
	      payload.securityMode = securityMode;
	      const resyncReplayOfAcceptedGenesis = Boolean(options.verifiedResyncReplay)
	        && (!matchStartPayloadRef.current
	          || String(matchStartPayloadRef.current?.genesis?.payloadHash || "")
	            === String(payload?.genesis?.payloadHash || ""));
	      if (verifiedMode && !options.skipGenesisVerification && !resyncReplayOfAcceptedGenesis) {
	        // A resent match_start must never rewind (or swap) a running match.
	        assertVerifiedMatchStartIsFresh(payload, {
	          session: currentSession,
	          currentPayload: matchStartPayloadRef.current,
	          gameOver: Boolean(stateRef.current?.game_over),
	        });
	        await verifySignedMatchGenesis(payload);
	        assertGenesisSeedRevealBinding(payload, localEntry);
	        await servicesRef.current.verifyMatchWitness(payload, currentSession);
	      }
	      if (verifiedMode) {
	        await verifyZiffleCeremoniesForPayload(payload);
	      }
      await servicesRef.current.resetOptimisticState?.("Match initialization");
      servicesRef.current.resetProtocolActionOrder?.("Match initialization");
      currentGame = gameRef.current;
	      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
	      ensureDirectPeerConnections(payload.players || []);
	      if (
	        verifiedMode
	        && localAuditPublicKey
	        && String(localEntry.auditPublicKey || "") !== String(localAuditPublicKey)
	      ) {
	        throw new Error("Match genesis does not bind the local audit key");
	      }
	      if (
	        verifiedMode
	        && localEncryptionPublicKey
	        && String(localEntry.auditEncryptionPublicKey || "") !== String(localEncryptionPublicKey)
	      ) {
	        throw new Error("Match genesis does not bind the local private-view encryption key");
	      }

	      if (isRelayId(payload.lobbyId)) {
        await loadFormatCatalog();
        if (verifiedMode && !currentSession.tournament?.tournamentId) throw new Error('Public relay lobbies require Trusted mode');
        assertFormatMatch({ ...payload, decks: validationDecksForMatchPayload(payload),
          sideboards: validationSideboardsForMatchPayload(payload), commanders: validationCommandersForMatchPayload(payload) });
      }
      const startDecks = verifiedMode
	        ? payload.players.map(() => [])
	        : validationDecksForMatchPayload(payload);
	      const startSideboards = validationSideboardsForMatchPayload(payload);
	      const startCommanders = validationCommandersForMatchPayload(payload);
	      const startPlanarDecks = validationPlanarDecksForMatchPayload(payload);

	      await currentGame.startMatch({
	        playerNames: payload.players.map((player) => player.name),
	        startingLife: payload.startingLife,
	        seed: payload.seed,
	        format: PUBLIC_FORMATS[payload.format]?.engineFormat || payload.format,
	        decks: startDecks,
	        sideboards: startSideboards,
	        commanders: startCommanders,
	        planarDecks: startPlanarDecks,
	        hiddenDeckManifests: verifiedMode ? payload.runtimeHiddenDeckManifests : undefined,
	        // Identical on every peer: decides which seats get a Miracle draw
	        // reveal window in hidden-deck matches.
	        publicDecklists: verifiedMode ? publicDecklistsForMatchPayload(payload) : undefined,
	        openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
	      });
	      await currentGame.setPerspective(localEntry.index);
	      liveZiffleCeremoniesRef.current.clear();
	      localZiffleCeremonyLookupRef.current.clear();
	      ziffleOpeningPositionsRef.current.clear();
	      ziffleRevealTokenCacheRef.current.clear();
	      verifiedAuditOpeningsRef.current.clear();
	      verifiedShuffleProofsRef.current.clear();
	      localRevealedOpeningsRef.current.clear();
	      privateViewDisclosuresRef.current.clear();
	      localZiffleRevealInFlightRef.current = null;
	      if (verifiedMode) {
	        clearStoredRevealedOpeningsForMatch(payload.auditMatchId || currentAuditMatchId());
	      }
	      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
	      ensureDirectPeerConnections(payload.players || []);
	      await currentGame.setPerspective(localEntry.index);

	      const nextState = await currentGame.uiState();
	      setState(nextState);
	      const matchClock = resetMatchClockForMatch(payload, nextState);
	      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
	      actionHistoryRef.current = [];
	      actionCryptoRequirementsRef.current.clear();
	      relayedActionIdsRef.current.clear();
	      applyingSequencedActionsRef.current.clear();
	      pendingSequencedActionsRef.current.clear();
	      drainingPendingSequencedActionsRef.current = false;
	      localZiffleRevealInFlightRef.current = null;
	      // Clients start the chain from the signed genesis; the host re-derives
	      // it once its final genesis (with the checkpoint hash) is signed.
	      const initialAuditStateHash = verifiedMode && !options.skipGenesisVerification
	        ? await initialAuditStateHashForMatch(payload)
	        : INITIAL_AUDIT_STATE_HASH;
	      auditStateHashRef.current = initialAuditStateHash;
	      const initialPublicCheckpointHash = await publicCheckpointHash(
	        await currentGame.exportPublicAuditCheckpoint()
	      );
	      if (
	        verifiedMode
	        && payload.initialPublicCheckpointHash
	        && String(payload.initialPublicCheckpointHash) !== initialPublicCheckpointHash
	      ) {
	        throw new Error("Initial public checkpoint does not match signed match genesis");
	      }
	      initialPublicCheckpointHashRef.current = initialPublicCheckpointHash;
	      payload.initialPublicCheckpointHash = initialPublicCheckpointHash;
	      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
	      liveAuditTranscriptRef.current = {
	        version: 1,
	        kind: verifiedMode
	          ? "ironsmith-live-browser-audit-v1"
	          : "ironsmith-live-browser-trusted-log-v1",
	        securityMode,
	        match: cloneMultiplayerPayload(payload),
	        matchId: payload.auditMatchId || currentAuditMatchId(),
	        lobbyId: payload.lobbyId || payload.hostPeerId || "",
	        protocolVersion: PROTOCOL_VERSION,
	        signatureAlgorithm: verifiedMode ? "ecdsa-p256-sha256" : "none",
	        genesis: verifiedMode ? cloneMultiplayerPayload(payload.genesis) : null,
	        initialStateHash: initialAuditStateHash,
	        initialPublicCheckpointHash,
	        actions: [],
	      };
	      if (verifiedMode && !options.skipGenesisVerification) {
	        rememberStartedGenesis(payload);
	        rotateGenesisSeedNonce(payload.auditMatchId, localEntry.index);
	      }
	      writeStoredPlayerIndex(payload.lobbyId || payload.hostPeerId, localEntry.index);

	      updateMultiplayer((prev) => ({
	        ...prev,
	        role: payload.hostPeerId === prev.localPeerId ? "host" : prev.role,
	        mode: "in_match",
	        lobbyId: payload.lobbyId || prev.lobbyId,
	        hostPeerId: payload.hostPeerId || prev.hostPeerId,
	        localPlayerIndex: localEntry.index,
	        desiredPlayers: payload.players.length,
	        startingLife: payload.startingLife,
	        format: normalizeMatchFormat(payload.format),
	        securityMode,
	        lockedSecurityMode: securityMode,
	        localDeckCount:
	          payload.deckAuditManifests?.[localEntry.index]?.deckCount
	          ?? payload.players?.[localEntry.index]?.deckCount
	          ?? payload.decks?.[localEntry.index]?.length
	          ?? prev.localDeckCount,
	        localCommanderCount:
	          payload.commanders?.[localEntry.index]?.length
	          ?? payload.players?.[localEntry.index]?.commanderCount
	          ?? prev.localCommanderCount,
	        players: payload.players,
	        rematch: null,
	        matchStarted: true,
	        matchDisputed: null,
	        lastAppliedSequence: 0,
	        submittingAction: false,
	        matchClock,
	        actionTimer: actionTimerSnapshotFromMatchClock(matchClock),
	      }));

	      if (verifiedMode && !options.deferLocalZiffleReveal) {
	        await revealLocalZiffleHand(payload);
	        await currentGame.setPerspective(localEntry.index);
	        setState(await currentGame.uiState());
	      }

	      setStatus(
	        verifiedMode
	          ? `Verified multiplayer match started as ${localEntry.name}`
	          : `Trusted multiplayer match started as ${localEntry.name}`
	      );
	    },
    [currentAuditMatchId, setState, setStatus, updateMultiplayer]
  );


  return { fairRandomRevealLockConflict, answerRngCommitRequest, answerRngRevealRequest, answerZiffleRevealTokenRequest, answerZiffleShuffleStepRequest, appendZiffleShuffleStep, applyMatchStart, applySequencedActionMessage, applySequencedActionMessageInner, applyVerifiedShuffleProofs, assertZiffleShuffleProofBoundToSignedMatch, authorizedZiffleRevealPositionsForOwner, bufferFutureSequencedAction, bufferedSequencedActionOptions, buildLiveZiffleShuffleProofs, buildLocalRngRevealsForRequirements, buildLocalShuffleProofsForRequirements, buildLocalZiffleRevealToken, buildLocalZiffleRevealTokens, buildLocalZiffleShuffleStep, buildZiffleCeremoniesForPayload, collectFairRandomReveal, collectZiffleRevealTokens, collectZiffleRevealTokensBatch, compactActionAuthorizationForDiagnostics, compactCryptoRequirementForDiagnostics, currentHostRouteInfo, drainPendingSequencedActions, fairRandomCommitSetHash, fairRandomRequestContextKey, fairRandomRequirementId, handIdsForRevealKey, hydrateViewedCardsFromLiveObjects, isHiddenViewedCardName, isInspectorOnlyViewedCards, localZiffleHandRevealKey, localZiffleHandRevealObjectIdsKey, normalizedZiffleShuffleCeremony, openConnectionForPeerCandidates, openZiffleRoute, playerIndexForPeerId, preserveViewedCardsFromHint, previewZiffleActionRequirements, recordZiffleShufflePerf, restoreSequencedActionValidationSnapshotIfCurrent, revealLocalZiffleHand, revealLocalZiffleHandInner, rngCommitmentForNonce, routePeerIdForPlayer, routingPlayers, runBatchedZiffleShuffleCeremonies, sendDirectProtocolMessage, sequencedActionValidationSnapshotStillCurrent, signRngCommitmentEntry, signRngRevealEntry, validateCompleteRngCommitSet, validateIncomingRngRequest, verifyRngCommitmentEntry, verifyRngRevealEntry, verifyShuffleProofsForRequirements, verifyZiffleCeremoniesForPayload, viewedCardsStateHint, waitForAuthorizedZiffleRevealPositions, waitForRevealAuthorizationSequence, waitForZiffleCeremony, waitForZiffleRoute, zifflePositionFromRequirement, zifflePositionsDetail, ziffleRequirementType, ziffleRequirementViewer, ziffleRequirementZone, ziffleRequirementsAuthorizeRevealPositions, ziffleRequirementsAuthorizeRevealPositionsByMetadata, ziffleRevealAuthorizedByAction, ziffleRevealAuthorizedByOutboundCryptoRequest, ziffleRevealPositionFromRequirement, ziffleRevealTokenOptionsForLocalHandReveal, ziffleRoutePeerCandidates, ziffleShuffleRequestForCeremony };
}
