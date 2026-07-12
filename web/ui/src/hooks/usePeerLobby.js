import {
  DISCONNECT_AUTO_FORFEIT_MS,
  DISCONNECT_FORFEIT_REASON,
  INITIAL_AUDIT_STATE_HASH,
  INITIAL_MATCH_CLOCK_HASH,
  MATCH_CLOCK_CLAIM_SKEW_MS,
  MATCH_CLOCK_TICK_MS,
  MULTIPLAYER_SECURITY_TRUSTED,
  MULTIPLAYER_SECURITY_VERIFIED,
  PROTOCOL_RESPONSE_TIMEOUT_MS,
  PROTOCOL_RESPONSE_TIMEOUT_REASON,
  PROTOCOL_VERSION,
  ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD,
  actionIntentKey,
  actionOpeningPreviewFromOpening,
  actionRefObjectId,
  actionTimerSnapshotFromMatchClock,
  alignShuffleProofsWithRequirements,
  buildExportedMatchOutcome,
  buildMatchClockConfig,
  buildPeerHeartbeatConfig,
  buildPeerOptions,
  canHostedMatchStart,
  cloneMultiplayerPayload,
  commandMayProducePostApplyOpenings,
  createEmptyState,
  createMatchClockSnapshot,
  cryptoRequirementsFromState,
  deckSlotOpeningsForManifest,
  describePeerServer,
  disconnectCertificateFromCommand,
  expectedLocalPublicOpeningPreviewCount,
  filterCryptoRequirementsForCommand,
  isActionTimeoutForfeitCommand,
  isDecisionCommandCompatible,
  isDisconnectTimeoutForfeitCommand,
  isForfeitCommand,
  isProtocolResponseTimeoutForfeitCommand,
  isSelfForfeitCommand,
  isSorcerySpeedForfeitState,
  isTrustedMultiplayerSecurityMode,
  mergeAuditOpenings,
  mergePrivateViewProofs,
  mergeShuffleProofs,
  missingRemotePublicOpenRequirements,
  missingShuffleRequirements,
  normalizePlayerIndex,
  openDecklistPlayerFields,
  parseDeckSubmission,
  payloadSizeBytes,
  playerNameForIndex,
  protocolResponseTimeoutClaimFromError,
  protocolResponseTimeoutVoteThreshold,
  publicCheckpointHash,
  publicDeckManifest,
  recordPeerSyncPerf,
  reindexPlayers,
  rememberDefaultLobbyDeck,
  resolveLocalPlayerIndex,
  safeSend,
  selectObjectSyncMetadataForCommand,
  sessionSecurityMode,
  shouldRequestRemoteCryptoPreview,
  summarizeCryptoMaterialForPerf,
  summarizeCryptoRequirementsForPerf,
  summarizePeerCommand,
  summarizeSequencedActionForPerf,
  timePeerSyncPhase,
  timeoutCertificateFromCommand,
  toErrorMessage,
  useCallback,
  useEffect,
  useRef,
  useState,
  withDeckState,
} from "./peer-lobby/shared.js";

import { usePeerLobbyConnections } from "./peer-lobby/connections.js";
import { usePeerLobbyAuditMaterial } from "./peer-lobby/audit-material.js";
import { usePeerLobbyCryptoResync } from "./peer-lobby/crypto-resync.js";
import { usePeerLobbyValidation } from "./peer-lobby/validation.js";
import { usePeerLobbyMessaging } from "./peer-lobby/messaging.js";

export function usePeerLobby({
  game,
  state,
  setState,
  setStatus,
  applySyncedCommand,
}) {
  const initialPeerOptions = buildPeerOptions();
  const initialHeartbeatConfig = buildPeerHeartbeatConfig();
  const initialMatchClockConfig = buildMatchClockConfig();
  const [multiplayer, setMultiplayer] = useState(() => createEmptyState());
  const peerRef = useRef(null);
  const hostConnectionRef = useRef(null);
  const clientConnectionsRef = useRef(new Map());
  const peerConnectionsRef = useRef(new Map());
  const connectionHeartbeatsRef = useRef(new Map());
  const matchStartPayloadRef = useRef(null);
  const actionHistoryRef = useRef([]);
  const gameRef = useRef(game);
  const stateRef = useRef(state);
  const multiplayerRef = useRef(multiplayer);
  const peerOptionsRef = useRef(initialPeerOptions);
  const peerHeartbeatConfigRef = useRef(initialHeartbeatConfig);
  const matchClockConfigRef = useRef(initialMatchClockConfig);
  const matchClockRef = useRef({
    policy: initialMatchClockConfig,
    playerCount: 0,
    baseRemainingMsByPlayer: [],
    activePlayerIndex: null,
    epochStartedAtMs: null,
    clockHash: INITIAL_MATCH_CLOCK_HASH,
    lastSequence: 0,
  });
  const timeoutClaimInFlightRef = useRef("");
  const disconnectForfeitInFlightRef = useRef(new Set());
  const localDisconnectObservationsRef = useRef(new Map());
  const peerServerLabelRef = useRef(describePeerServer(initialPeerOptions));
  const hostMessageQueueRef = useRef(Promise.resolve());
  const clientMessageQueueRef = useRef(Promise.resolve());
  const peerMessageQueueRef = useRef(Promise.resolve());
  const resyncingPeerIdsRef = useRef(new Set());
  const resyncWaitersRef = useRef([]);
  const submissionIdleWaitersRef = useRef([]);
  const actionSubmissionStartedAtMsRef = useRef(0);
  const awaitingStateResyncRef = useRef(false);
  const auditKeyPairRef = useRef(null);
  const auditEncryptionKeyPairRef = useRef(null);
  const auditPublicKeyRef = useRef("");
  const auditEncryptionPublicKeyRef = useRef("");
  const auditStateHashRef = useRef(INITIAL_AUDIT_STATE_HASH);
  const initialPublicCheckpointHashRef = useRef("");
  const auditVerifyKeyCacheRef = useRef(new Map());
  const liveAuditTranscriptRef = useRef(null);
  const privateDeckManifestsRef = useRef(new Map());
  const localRevealedOpeningsRef = useRef(new Map());
  const ziffleKeyPairsRef = useRef(new Map());
  const ziffleShuffleWaitersRef = useRef(new Map());
  const ziffleRevealWaitersRef = useRef(new Map());
  const ziffleRevealTokenCacheRef = useRef(new Map());
  const verifiedAuditOpeningsRef = useRef(new Set());
  const rngCommitWaitersRef = useRef(new Map());
  const rngRevealWaitersRef = useRef(new Map());
  const rngCommitNoncesRef = useRef(new Map());
  const signedRngCommitmentsRef = useRef(new Map());
  const rngRevealCommitSetLocksRef = useRef(new Map());
  const timeoutVoteWaitersRef = useRef(new Map());
  const actionQuorumVoteWaitersRef = useRef(new Map());
  const signedActionQuorumVotesRef = useRef(new Map());
  const cryptoMaterialWaitersRef = useRef(new Map());
  const outboundCryptoMaterialRequestsRef = useRef(new Map());
  const pendingActionIntentsRef = useRef(new Map());
  const pendingActionIntentTimeoutsRef = useRef(new Map());
  const ignoredActionIntentKeysRef = useRef(new Map());
  const actionIntentOpeningPreviewKeysRef = useRef(new Map());
  const reconnectChallengesRef = useRef(new Map());
  const privateViewDisclosuresRef = useRef(new Map());
  const liveZiffleCeremoniesRef = useRef(new Map());
  const localZiffleCeremonyLookupRef = useRef(new Map());
  const ziffleOpeningPositionsRef = useRef(new Map());
  const ziffleHandRevealKeyRef = useRef("");
  const ziffleHandRevealQuickKeyRef = useRef("");
  const localZiffleRevealInFlightRef = useRef(null);
  const verifiedShuffleProofsRef = useRef(new Set());
  const ziffleShufflePerfRef = useRef([]);
  const relayedActionIdsRef = useRef(new Set());
  const applyingSequencedActionsRef = useRef(new Map());
  const pendingSequencedActionsRef = useRef(new Map());
  const drainingPendingSequencedActionsRef = useRef(false);
  const actionCryptoRequirementsRef = useRef(new Map());
  const matchClockObservationExemptSequenceRef = useRef(0);
  const ensureDirectPeerConnectionsRef = useRef(() => {});

  useEffect(() => {
    gameRef.current = game;
  }, [game]);

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  // updateMultiplayer owns multiplayerRef so async message handlers can observe
  // state changes before React commits the next render. Mirroring rendered state
  // back into the ref here can regress the ref to an older render during fast
  // peer message sequences.

  const servicesRef = useRef({});
  const peerLobbyBase = {
    game, state, setState, setStatus, applySyncedCommand, multiplayer, setMultiplayer,
    peerRef, hostConnectionRef, clientConnectionsRef, peerConnectionsRef, connectionHeartbeatsRef,
    matchStartPayloadRef, actionHistoryRef, gameRef, stateRef, multiplayerRef, peerOptionsRef,
    peerHeartbeatConfigRef, matchClockConfigRef, matchClockRef, timeoutClaimInFlightRef,
    disconnectForfeitInFlightRef, localDisconnectObservationsRef, peerServerLabelRef,
    hostMessageQueueRef, clientMessageQueueRef, peerMessageQueueRef, resyncingPeerIdsRef,
    resyncWaitersRef, submissionIdleWaitersRef, actionSubmissionStartedAtMsRef, awaitingStateResyncRef,
    auditKeyPairRef, auditEncryptionKeyPairRef, auditPublicKeyRef, auditEncryptionPublicKeyRef,
    auditStateHashRef, initialPublicCheckpointHashRef, auditVerifyKeyCacheRef, liveAuditTranscriptRef,
    privateDeckManifestsRef, localRevealedOpeningsRef, ziffleKeyPairsRef, ziffleShuffleWaitersRef,
    ziffleRevealWaitersRef, ziffleRevealTokenCacheRef, verifiedAuditOpeningsRef, rngCommitWaitersRef,
    rngRevealWaitersRef, rngCommitNoncesRef, signedRngCommitmentsRef, rngRevealCommitSetLocksRef,
    timeoutVoteWaitersRef, actionQuorumVoteWaitersRef, signedActionQuorumVotesRef,
    cryptoMaterialWaitersRef, outboundCryptoMaterialRequestsRef, pendingActionIntentsRef,
    pendingActionIntentTimeoutsRef, ignoredActionIntentKeysRef, actionIntentOpeningPreviewKeysRef,
    reconnectChallengesRef, privateViewDisclosuresRef, liveZiffleCeremoniesRef,
    localZiffleCeremonyLookupRef, ziffleOpeningPositionsRef, ziffleHandRevealKeyRef,
    ziffleHandRevealQuickKeyRef, localZiffleRevealInFlightRef, verifiedShuffleProofsRef,
    ziffleShufflePerfRef, relayedActionIdsRef, applyingSequencedActionsRef, pendingSequencedActionsRef,
    drainingPendingSequencedActionsRef, actionCryptoRequirementsRef,
    matchClockObservationExemptSequenceRef, ensureDirectPeerConnectionsRef,
  };

  const connections = usePeerLobbyConnections(peerLobbyBase, servicesRef);
  const { actionBroadcastResponseTimeoutMs, actionIntentKeyFromProtocolClaim, beginPeerWait, broadcastActionIntentCancel, broadcastActionIntentProgress, clearPeerWait, clearPendingActionIntent, currentAuditMatchId, ensureAuditIdentity, ensureZiffleIdentity, publicZiffleKey, rememberIgnoredActionIntentKey, signActionIntentForCommand, signPlayerGenesis, startActionIntentProgressBroadcast, updateMultiplayer, updatePeerWait, waitForPendingActionIntentBeforeLocalSubmit, waitForSubmissionIdle } = connections;
  Object.assign(servicesRef.current, connections);

  const auditMaterial = usePeerLobbyAuditMaterial(peerLobbyBase, servicesRef);
  const { buildLocalDeckAuditManifest, buildLocalOpeningsForCommand, buildLocalRequirementOpeningsForRequirements, buildSequencedActionAudit, currentKnownPublicAuditCheckpointHash, currentPublicAuditCheckpointHash, previewAuditOpeningInInspector, previewRequirementsForCommand, revealAuditOpenings, verifyAuditSatisfiesCryptoRequirements, verifySequencedActionAudit } = auditMaterial;
  Object.assign(servicesRef.current, auditMaterial);

  const cryptoResync = usePeerLobbyCryptoResync(peerLobbyBase, servicesRef);
  const { appendAppliedSequencedAction, buildLocalPrivateViewProofsForRequirements, buildMatchClockAuditForCommand, collectActionQuorumCertificate, collectDisconnectForfeitCertificateForCommand, collectProtocolResponseTimeoutCertificateForCommand, collectRemoteCryptoMaterialForRequirements, collectTimeoutCertificateForCommand, commitMatchClockAudit, createSequencedActionValidationSnapshot, currentHiddenRefForObjectId, currentStableIdForObjectId, filterOpeningsForCommandHiddenRefs, forfeitedPlayersForQuorum, freshCryptoRequirementsForSequence, injectCryptoMaterialForRequirements, leaveLobby, markMatchDisputed, protocolResponseTimeoutRoster, publishCurrentRuntimeState, relaySequencedAction, rememberActionCryptoRequirements, restoreMatchClockRuntime, revealPrivateAuditProofsForLocalViewer, stageLocalMatchClockAudit, teardownPeer, updateMatchClockForState, validateDisconnectForfeitCommand, validateProtocolResponseTimeoutCommand, validateTimeoutForfeitCommand, validateTrustedSequencedAction, verifyActionQuorumForMessage, verifyMatchClockAuditForAction, waitForPeerResyncs } = cryptoResync;
  Object.assign(servicesRef.current, cryptoResync);

  const validation = usePeerLobbyValidation(peerLobbyBase, servicesRef);
  const { applyVerifiedShuffleProofs, buildLocalRngRevealsForRequirements, buildLocalShuffleProofsForRequirements, drainPendingSequencedActions, restoreSequencedActionValidationSnapshotIfCurrent, revealLocalZiffleHand, routePeerIdForPlayer, verifyShuffleProofsForRequirements, viewedCardsStateHint } = validation;
  Object.assign(servicesRef.current, validation);

  const messaging = usePeerLobbyMessaging(peerLobbyBase, servicesRef);
  const { broadcastLobbyState, createLobby, joinLobby, readyForRematch, startHostedMatch, startRematchSideboarding, updateRematchDecks } = messaging;
  Object.assign(servicesRef.current, messaging);

  const updateLobbyDeck = useCallback(
    async (updates) => {
      const currentSession = multiplayerRef.current;
      if (currentSession.matchStarted || currentSession.mode === "starting") {
        return;
      }

      const nextDeckText =
        typeof updates === "string"
          ? String(updates)
          : Object.prototype.hasOwnProperty.call(updates || {}, "deckText")
            ? String(updates.deckText || "")
            : currentSession.localDeckText;
      const nextCommanderText =
        typeof updates === "string"
          ? currentSession.localCommanderText
          : Object.prototype.hasOwnProperty.call(updates || {}, "commanderText")
            ? String(updates.commanderText || "")
            : currentSession.localCommanderText;

	    const deckSubmission = parseDeckSubmission(
	        currentSession.format,
	        nextDeckText,
	        nextCommanderText
	      );
      rememberDefaultLobbyDeck(nextDeckText, nextCommanderText);
	      if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(currentSession))) {
	        const nextSession = updateMultiplayer((prev) => ({
	          ...prev,
	          localDeckText: nextDeckText,
	          localCommanderText: nextCommanderText,
	          localDeckCount: deckSubmission.deckCount,
	          localCommanderCount: deckSubmission.commanderCount,
	          players:
	            prev.role === "host" && prev.localPeerId
	              ? reindexPlayers(
	                  prev.players.map((player) =>
	                    player.peerId === prev.localPeerId
	                      ? withDeckState(
	                          {
	                            ...player,
	                            deckAuditManifest: null,
	                            deckSlotOpenings: [],
	                            ziffleKey: null,
	                            playerGenesisSignature: null,
	                          },
	                          prev.format,
	                          deckSubmission.deck,
	                          deckSubmission.commanders,
	                          deckSubmission.sideboard
	                        )
	                      : player
	                  )
	                )
	              : prev.players,
	        }));
	        if (nextSession.role === "host") {
	          broadcastLobbyState();
	          return;
	        }
	        const conn = hostConnectionRef.current;
	        if (nextSession.role === "client" && conn && conn.open !== false) {
	          safeSend(conn, {
	            type: "deck_update",
	            protocolVersion: PROTOCOL_VERSION,
	            securityMode: MULTIPLAYER_SECURITY_TRUSTED,
	            deckAuditManifest: null,
	            deck: deckSubmission.deck,
	            sideboard: deckSubmission.sideboard,
	            deckSlotOpenings: [],
	            ziffleKey: null,
	            deckCount: deckSubmission.deckCount,
	            sideboardCount: deckSubmission.sideboard.length,
	            commanders: deckSubmission.commanders,
	            commanderCount: deckSubmission.commanderCount,
	            ready: deckSubmission.ready,
	          });
	        }
	        return;
	      }
	      const {
	        publicKey: auditPublicKey,
        encryptionPublicKey: auditEncryptionPublicKey,
      } = await ensureAuditIdentity();
      const resolvedLocalPlayerIndex = resolveLocalPlayerIndex(currentSession);
      const localPlayerIndex = resolvedLocalPlayerIndex ?? 0;
      const deckAuditManifest = await buildLocalDeckAuditManifest({
        matchId: currentSession.lobbyId || currentSession.hostPeerId || "pending",
        owner: localPlayerIndex,
        deck: deckSubmission.deck,
        sideboard: deckSubmission.sideboard,
        commanders: deckSubmission.commanders,
        // Without an assigned seat the owner is a guess; persisting it could
        // shadow the real seat-0 deck in private manifest lookups.
        persist: resolvedLocalPlayerIndex != null,
      });
      const deckSlotOpenings = deckSlotOpeningsForManifest(deckAuditManifest);
      const ziffleKeyPair = await ensureZiffleIdentity({
        context: currentSession.lobbyId || currentSession.hostPeerId || "pending",
        deckCount: deckSubmission.deckCount || 60,
      });
      const localZiffleKey = publicZiffleKey(ziffleKeyPair, localPlayerIndex);
      const localGenesisPlayer = {
        peerId: currentSession.localPeerId,
        name: currentSession.localName,
        index: localPlayerIndex,
        auditPublicKey,
        auditEncryptionPublicKey,
        deckAuditManifest: publicDeckManifest(deckAuditManifest),
        ziffleKey: localZiffleKey,
        ...openDecklistPlayerFields({
          deck: deckSubmission.deck,
          sideboard: deckSubmission.sideboard,
          commanders: deckSubmission.commanders,
          deckSlotOpenings,
        }),
        deckCount: deckSubmission.deckCount,
        sideboardCount: deckSubmission.sideboard.length,
        commanderCount: deckSubmission.commanderCount,
      };
      const playerGenesisSignature = await signPlayerGenesis({
        matchId: currentSession.lobbyId || currentSession.hostPeerId || "pending",
        player: localGenesisPlayer,
      });
      const nextSession = updateMultiplayer((prev) => ({
        ...prev,
        localDeckText: nextDeckText,
        localCommanderText: nextCommanderText,
        localDeckCount: deckSubmission.deckCount,
        localCommanderCount: deckSubmission.commanderCount,
        players:
          prev.role === "host" && prev.localPeerId
            ? reindexPlayers(
                prev.players.map((player) =>
                  player.peerId === prev.localPeerId
                    ? withDeckState(
                        {
	                          ...player,
	                          auditPublicKey,
		                          auditEncryptionPublicKey,
		                          deckAuditManifest: publicDeckManifest(deckAuditManifest),
                          deckSlotOpenings,
	                          ziffleKey: localZiffleKey,
	                          playerGenesisSignature,
                        },
                        prev.format,
                        deckSubmission.deck,
                        deckSubmission.commanders,
                        deckSubmission.sideboard
                      )
                    : player
                )
              )
            : prev.players,
      }));

      if (nextSession.role === "host") {
        broadcastLobbyState();
        return;
      }

      const conn = hostConnectionRef.current;
      if (nextSession.role === "client" && conn && conn.open !== false) {
        safeSend(conn, {
          type: "deck_update",
	          protocolVersion: PROTOCOL_VERSION,
	          auditPublicKey,
	          auditEncryptionPublicKey,
		          playerGenesisSignature,
          deckAuditManifest: publicDeckManifest(deckAuditManifest),
          deck: deckSubmission.deck,
          sideboard: deckSubmission.sideboard,
          deckSlotOpenings,
          ziffleKey: localZiffleKey,
          deckCount: deckSubmission.deckCount,
          sideboardCount: deckSubmission.sideboard.length,
          commanders: deckSubmission.commanders,
          commanderCount: deckSubmission.commanderCount,
	          ready: deckSubmission.ready,
        });
      }
    },
    [
      broadcastLobbyState,
      buildLocalDeckAuditManifest,
      ensureZiffleIdentity,
      ensureAuditIdentity,
      publicZiffleKey,
      signPlayerGenesis,
      updateMultiplayer,
    ]
  );

  async function submitProtocolResponseTimeoutClaim(claim) {
    if (!claim || typeof claim !== "object") return false;
    const targetPlayerIndex = normalizePlayerIndex(claim.targetPlayerIndex);
    if (targetPlayerIndex == null) return false;
    const timedOutActionIntentKey = actionIntentKeyFromProtocolClaim(claim);
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    const roster = protocolResponseTimeoutRoster(targetPlayerIndex);
    const threshold = protocolResponseTimeoutVoteThreshold(roster.length);
    const target = players.find((player) => Number(player.index) === Number(targetPlayerIndex));
    const targetName = String(claim.targetName || target?.name || `Player ${targetPlayerIndex + 1}`);
    if (timedOutActionIntentKey) {
      rememberIgnoredActionIntentKey(timedOutActionIntentKey, "protocol_response_timeout");
      clearPendingActionIntent(timedOutActionIntentKey);
    }
    if (threshold <= 0) {
      markMatchDisputed(
        `Protocol response timeout from ${targetName}; two-player matches require external arbitration.`,
        {
          type: "protocol_response_timeout",
          accusedPlayers: [targetPlayerIndex],
          claim: cloneMultiplayerPayload(claim),
        }
      );
      return true;
    }
    const responseTimeoutMs = Math.max(
      1,
      Math.floor(Number(claim.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS))
    );
    const requestedAtMs = Math.max(1, Math.floor(Number(claim.requestedAtMs || Date.now() - responseTimeoutMs)));
    const command = {
      type: "forfeit_player",
      player: targetPlayerIndex,
      reason: PROTOCOL_RESPONSE_TIMEOUT_REASON,
      timed_out_peer_id: String(claim.targetPeerId || target?.peerId || ""),
      request_type: String(claim.requestType || ""),
      request_id: String(claim.requestId || ""),
      request_payload_hash: String(claim.requestPayloadHash || ""),
      response_timeout_ms: responseTimeoutMs,
      requested_at_ms: requestedAtMs,
      eligible_at_ms: requestedAtMs + responseTimeoutMs,
      claimed_at_ms: Date.now(),
      basis_sequence: Number(claim.basisSequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
    };
    try {
      await submitMultiplayerCommand(
        command,
        `${targetName} forfeited by protocol response timeout`
      );
    } catch (err) {
      const failureReason = toErrorMessage(err);
      if (
        failureReason.includes("Protocol response timeout certificate")
        || failureReason.includes("Protocol-timeout vote")
        || failureReason.includes("protocol-timeout voter")
      ) {
        markMatchDisputed(
          `Protocol response timeout from ${targetName}; certificate quorum could not be completed.`,
          {
            type: "protocol_response_timeout_certificate_failed",
            accusedPlayers: [targetPlayerIndex],
            claim: cloneMultiplayerPayload(claim),
            command: cloneMultiplayerPayload(command),
            error: failureReason,
          }
        );
        return true;
      }
      throw err;
    }
    return true;
  }

  const submitMultiplayerCommand = useCallback(
    async (command, label = "") => {
      let session = multiplayerRef.current;
      if (!session.matchStarted) {
        setStatus("Match has not started yet", true);
        return;
      }
      if (session.submittingAction) {
        setStatus("Waiting for the previous action to sync");
        await waitForSubmissionIdle();
        session = multiplayerRef.current;
        if (!session.matchStarted) {
          setStatus("Match has not started yet", true);
          return;
        }
        if (session.submittingAction) {
          setStatus("Waiting for the previous action to sync");
          return;
        }
      }
      if (session.localPlayerIndex == null) {
        setStatus("Local player seat is not assigned", true);
        return;
      }
      if (awaitingStateResyncRef.current) {
        setStatus("Waiting for resync to finish");
        return;
      }
      const trustedMode = isTrustedMultiplayerSecurityMode(sessionSecurityMode(session));

      let stagedMatchClockRuntime = null;
      let stopActionIntentProgress = null;
      let localActionWaitId = null;
      let localSubmissionSnapshot = null;
      let localSubmissionCommitted = false;
      let signedActionIntent = null;
      const clearLocalActionWait = () => {
        if (localActionWaitId) {
          clearPeerWait(localActionWaitId);
          localActionWaitId = null;
        }
      };
      const stopLocalActionIntentProgress = () => {
        if (stopActionIntentProgress) {
          stopActionIntentProgress();
          stopActionIntentProgress = null;
        }
      };
      const cancelLocalActionIntent = (reason) => {
        stopLocalActionIntentProgress();
        if (signedActionIntent && !localSubmissionCommitted) {
          rememberIgnoredActionIntentKey(actionIntentKey(signedActionIntent), reason || "local_action_cancelled");
          broadcastActionIntentCancel(signedActionIntent, reason);
        }
      };
      updateMultiplayer((prev) => ({ ...prev, submittingAction: true }));
      try {
        if (localZiffleRevealInFlightRef.current) {
          setStatus("Waiting for hidden-card reveal payloads to finish");
          await localZiffleRevealInFlightRef.current;
        }
        if (resyncingPeerIdsRef.current.size > 0) {
          const waitId = beginPeerWait({
            kind: "peer_resync",
            title: "Waiting for peer resync",
            description:
              "One or more peers are importing the latest checkpoint before another action can be submitted.",
          });
          setStatus("Waiting for peers to finish resyncing");
          try {
            await waitForPeerResyncs();
          } finally {
            clearPeerWait(waitId);
          }
        }
        let preSubmitState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
        if (!isDecisionCommandCompatible(preSubmitState?.decision, command)) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("That action is no longer available");
          return;
        }
        if (command?.type === "priority_action" && command.action_ref) {
          const objectId = Number(command.object_id ?? command.objectId ?? actionRefObjectId(command.action_ref));
          const hasStableId = command.object_stable_id != null || command.objectStableId != null;
          let priorityObjectMetadata = {};
          if (Number.isSafeInteger(objectId) && objectId > 0 && !hasStableId) {
            const stableId = await currentStableIdForObjectId(objectId);
            if (stableId != null) {
              priorityObjectMetadata.object_stable_id = stableId;
            }
          }
          const hasHiddenRef = command.object_hidden_ref != null || command.objectHiddenRef != null;
          if (Number.isSafeInteger(objectId) && objectId > 0 && !hasHiddenRef) {
            const hiddenRef = await currentHiddenRefForObjectId(objectId);
            if (hiddenRef) {
              priorityObjectMetadata.object_hidden_ref = hiddenRef;
            }
          }
          if (Number.isSafeInteger(objectId) && objectId > 0) {
            priorityObjectMetadata.object_id = objectId;
          }
          if (Object.keys(priorityObjectMetadata).length > 0) {
            command = {
              ...command,
              ...priorityObjectMetadata,
            };
          }
        }
        if (command?.type === "select_objects" && Array.isArray(command.object_ids)) {
          const objectIds = command.object_ids.map((objectId) => Number(objectId));
          const { stableIds, hiddenRefs } = selectObjectSyncMetadataForCommand(
            { ...command, object_ids: objectIds },
            preSubmitState
          );
          command = {
            ...command,
            object_ids: objectIds,
          };
          if (
            !Array.isArray(command.object_stable_ids)
            && !Array.isArray(command.objectStableIds)
            && stableIds.some((stableId) => stableId != null)
          ) {
            command.object_stable_ids = stableIds;
          }
          if (
            !Array.isArray(command.object_hidden_refs)
            && !Array.isArray(command.objectHiddenRefs)
            && hiddenRefs.some((hiddenRef) => hiddenRef != null)
          ) {
            command.object_hidden_refs = hiddenRefs;
          }
        }
        const expectedActor = preSubmitState?.decision?.player;
        const isTimeoutForfeit = isActionTimeoutForfeitCommand(command);
        const isDisconnectForfeit = isDisconnectTimeoutForfeitCommand(command);
        const isProtocolTimeoutForfeit = isProtocolResponseTimeoutForfeitCommand(command);
        const isSelfForfeit = isSelfForfeitCommand(command, session.localPlayerIndex);
        if (isSelfForfeit && !isSorcerySpeedForfeitState(preSubmitState, session.localPlayerIndex)) {
          throw new Error("Surrender is only available at sorcery speed");
        }
        if (
          isForfeitCommand(command)
          && !isTimeoutForfeit
          && !isDisconnectForfeit
          && !isProtocolTimeoutForfeit
        ) {
          if (Number(command.player) !== Number(session.localPlayerIndex)) {
            throw new Error("A player can only forfeit themselves");
          }
        }
        if (isTimeoutForfeit) {
          const liveState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
          updateMatchClockForState(liveState);
          if (!trustedMode && !timeoutCertificateFromCommand(command)) {
            const certificate = await collectTimeoutCertificateForCommand(command);
            if (certificate) {
              command = {
                ...command,
                timeout_certificate: certificate,
              };
            }
          }
          await validateTimeoutForfeitCommand(command, liveState, {
            skewMs: MATCH_CLOCK_CLAIM_SKEW_MS,
            skipCertificate: trustedMode,
          });
        } else if (isDisconnectForfeit) {
          if (!trustedMode && !disconnectCertificateFromCommand(command)) {
            const certificate = await collectDisconnectForfeitCertificateForCommand(command);
            if (certificate) {
              command = {
                ...command,
                disconnected_peer_id: command.disconnected_peer_id || certificate.forfeitedPeerId,
                disconnect_timeout_ms: certificate.disconnectTimeoutMs,
                disconnect_certificate: certificate,
              };
            }
          }
          await validateDisconnectForfeitCommand(command, {
            actorIndex: session.localPlayerIndex,
            skipCertificate: trustedMode,
          });
        } else if (isProtocolTimeoutForfeit) {
          if (
            !trustedMode
            && !command.protocol_timeout_certificate
            && !command.protocolTimeoutCertificate
          ) {
            const certificate = await collectProtocolResponseTimeoutCertificateForCommand(command);
            if (certificate) {
              command = {
                ...command,
                timed_out_peer_id: command.timed_out_peer_id || certificate.forfeitedPeerId,
                response_timeout_ms: certificate.responseTimeoutMs,
                requested_at_ms: certificate.requestedAtMs,
                protocol_timeout_certificate: certificate,
              };
            }
          }
          await validateProtocolResponseTimeoutCommand(command, {
            actorIndex: session.localPlayerIndex,
            skipCertificate: trustedMode,
          });
        } else if (
          expectedActor !== null
          && expectedActor !== undefined
          && Number(expectedActor) !== Number(session.localPlayerIndex)
        ) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("It is not your turn to act");
          return;
        }
        const nextSequence = Number(multiplayerRef.current.lastAppliedSequence || 0) + 1;
        if (trustedMode) {
          preSubmitState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
          const expectedPreviousSequence = nextSequence - 1;
          const latestAppliedSequenceBeforeApply = Number(multiplayerRef.current.lastAppliedSequence || 0);
          const latestHistorySequenceBeforeApply = Number(
            actionHistoryRef.current.at(-1)?.seq ?? latestAppliedSequenceBeforeApply
          );
          if (
            awaitingStateResyncRef.current
            || latestAppliedSequenceBeforeApply !== expectedPreviousSequence
            || latestHistorySequenceBeforeApply !== expectedPreviousSequence
          ) {
            updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
            setStatus("Another action was broadcast first");
            return;
          }
          const clock = await buildMatchClockAuditForCommand({
            command,
            seq: nextSequence,
            actorIndex: session.localPlayerIndex,
            uiState: preSubmitState,
          });
          await validateTrustedSequencedAction({
            command,
            actorIndex: session.localPlayerIndex,
            seq: nextSequence,
            clock,
            uiState: preSubmitState,
            enforceMatchClockObservationBounds: false,
          });
          localSubmissionSnapshot = await createSequencedActionValidationSnapshot();
          stagedMatchClockRuntime = stageLocalMatchClockAudit(clock);
          const appliedState = await applySyncedCommand(command, label || "", {
            actorIndex: session.localPlayerIndex,
            sequence: nextSequence,
            publishState: false,
          });
          const message = {
            type: "apply_action",
            protocolVersion: PROTOCOL_VERSION,
            securityMode: MULTIPLAYER_SECURITY_TRUSTED,
            seq: nextSequence,
            actorIndex: session.localPlayerIndex,
            command,
            label: label || "",
            clock,
          };
          commitMatchClockAudit(clock, appliedState);
          await appendAppliedSequencedAction(message);
          localSubmissionCommitted = true;
          relaySequencedAction(message);
          await publishCurrentRuntimeState(appliedState);
          await drainPendingSequencedActions();
          setStatus("Action broadcast to trusted peers");
          return;
        }
	        const showLocalActionWait = (patch = {}) => {
          const requestId = `local-action:${currentAuditMatchId()}:${nextSequence}`;
          const localName = playerNameForIndex(
            multiplayerRef.current.players,
            multiplayerRef.current.localPlayerIndex
          );
          const wait = {
            kind: "local_action",
            requestId,
            local: true,
            peerIndex: multiplayerRef.current.localPlayerIndex,
            peerName: localName || "You",
            title: "Working on your action",
            description:
              "Your browser is applying the action locally, producing hidden-card proofs, "
              + "and assembling the signed payload for peers.",
            operation: "Preparing action",
            ...patch,
          };
          if (localActionWaitId === requestId && multiplayerRef.current.peerWait?.requestId === requestId) {
            updatePeerWait(requestId, wait);
          } else {
            localActionWaitId = beginPeerWait(wait);
	          }
        };
        let localOpeningPreviewCount = 0;
        let localOpeningPreviewTotal = 0;
        const localOpeningPreviewKeys = new Set();
        const broadcastLocalActionProgress = (
          phase = "payload_generation",
          responseTimeoutMs = ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD,
          extraPayload = {}
        ) => {
          if (!signedActionIntent) return;
          if (typeof stopActionIntentProgress?.update === "function") {
            stopActionIntentProgress.update(phase, responseTimeoutMs, extraPayload);
            return;
          }
          broadcastActionIntentProgress(
            signedActionIntent,
            phase,
            responseTimeoutMs,
            extraPayload
          );
        };
        const updateLocalActionProgress = (
          waitPatch = {},
          phase = "payload_generation",
          responseTimeoutMs = ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD,
          extraPayload = null
        ) => {
          showLocalActionWait(waitPatch);
          broadcastLocalActionProgress(
            phase,
            responseTimeoutMs,
            extraPayload || {
              operation: waitPatch.operation,
              detail: waitPatch.detail,
              cardName: waitPatch.cardName,
              zone: waitPatch.zone,
              progressCurrent: waitPatch.progressCurrent,
              progressTotal: waitPatch.progressTotal,
              title: waitPatch.title,
              description: waitPatch.description,
            }
          );
        };
        const previewBuiltLocalOpening = (opening, metadata = {}) => {
          const preview = actionOpeningPreviewFromOpening(opening, {
            zone: metadata.zone || "exile",
          });
          if (!preview) return;
          const previewKey = [
            preview.owner,
            opening?.slot ?? "",
            preview.objectId ?? "",
            preview.stableId ?? "",
            preview.position ?? "",
            preview.zone || "",
            preview.card || "",
          ].join(":");
          if (localOpeningPreviewKeys.has(previewKey)) return;
          localOpeningPreviewKeys.add(previewKey);
          localOpeningPreviewCount += 1;
          const metadataTotal = Number(metadata.total);
          localOpeningPreviewTotal = Math.max(
            localOpeningPreviewTotal,
            Number.isFinite(metadataTotal) && metadataTotal > 0 ? metadataTotal : 0,
            localOpeningPreviewCount
          );
          const progress = {
            progressCurrent: localOpeningPreviewCount,
            progressTotal: localOpeningPreviewTotal,
          };
          previewAuditOpeningInInspector(preview, stateRef.current, {
            previewIndex: localOpeningPreviewCount - 1,
            previewTotal: localOpeningPreviewTotal,
            previewZone: preview.zone,
          });
          const progressPayload = {
            operation: "Opening revealed card",
            cardName: preview.card,
            zone: preview.zone,
            openingPreview: preview,
            ...progress,
          };
          updateLocalActionProgress(
            {
              kind: "local_ziffle_reveal",
              title: "Opening revealed card",
              operation: "Opening revealed card",
              cardName: preview.card,
              zone: preview.zone,
              ...progress,
            },
            "opening_preview",
            ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD,
            progressPayload
          );
        };
        const submitPerf = {
          seq: nextSequence,
          actor: session.localPlayerIndex == null ? null : Number(session.localPlayerIndex),
          command: summarizePeerCommand(command),
          decision_kind: String(preSubmitState?.decision?.kind || ""),
          decision_player: preSubmitState?.decision?.player == null
            ? null
            : Number(preSubmitState.decision.player),
          stack_size: Number(preSubmitState?.stack_size || 0),
        };
        recordPeerSyncPerf("submit_action:start", submitPerf);
        const preActionStateHash = String(auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH);
        let preActionPublicCheckpointHash = "";
        const ensurePreActionPublicCheckpointHash = async () => {
          if (!preActionPublicCheckpointHash) {
            preActionPublicCheckpointHash =
              currentKnownPublicAuditCheckpointHash()
              || await currentPublicAuditCheckpointHash();
          }
          return preActionPublicCheckpointHash;
        };
        const ensureSignedActionIntent = async () => {
          if (!signedActionIntent) {
            signedActionIntent = await signActionIntentForCommand({
              seq: nextSequence,
              actorIndex: session.localPlayerIndex,
              command,
              prevStateHash: preActionStateHash,
              preActionPublicCheckpointHash: await ensurePreActionPublicCheckpointHash(),
            });
          }
          return signedActionIntent;
        };
        const pendingIntentCleared = await waitForPendingActionIntentBeforeLocalSubmit(nextSequence);
        if (!pendingIntentCleared) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("Another action was signed first");
          return;
        }
        preSubmitState = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
        if (Number(multiplayerRef.current.lastAppliedSequence || 0) !== nextSequence - 1) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("Another action was signed first");
          return;
        }
        if (!isDecisionCommandCompatible(preSubmitState?.decision, command)) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("That action is no longer available");
          return;
        }
        const refreshedExpectedActor = preSubmitState?.decision?.player;
        if (
          !isForfeitCommand(command)
          && refreshedExpectedActor !== null
          && refreshedExpectedActor !== undefined
          && Number(refreshedExpectedActor) !== Number(session.localPlayerIndex)
        ) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          setStatus("It is not your turn to act");
          return;
        }
        const clock = await buildMatchClockAuditForCommand({
          command,
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          uiState: preSubmitState,
        });
	        await verifyMatchClockAuditForAction({
	          clock,
	          command,
	          seq: nextSequence,
	          actorIndex: session.localPlayerIndex,
	          uiState: preSubmitState,
	          skewMs: 0,
	          enforceObservationBounds: false,
	        });
        localSubmissionSnapshot = await createSequencedActionValidationSnapshot();
        stagedMatchClockRuntime = stageLocalMatchClockAudit(clock);
        let cryptoRequirements = await timePeerSyncPhase(
          "submit_action:preview_requirements",
          submitPerf,
          async () => filterCryptoRequirementsForCommand(
            command,
            preSubmitState,
            freshCryptoRequirementsForSequence(
              nextSequence,
              await previewRequirementsForCommand(command)
            )
          )
        );
        recordPeerSyncPerf("submit_action:preview_requirements:summary", {
          ...submitPerf,
          requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
        });
        rememberActionCryptoRequirements(nextSequence, cryptoRequirements);
        let requestRemoteCryptoPreview = shouldRequestRemoteCryptoPreview(
          command,
          preSubmitState,
          cryptoRequirements
        );
        const actionMayNeedCryptoMaterial =
          cryptoRequirements.length > 0
          || commandMayProducePostApplyOpenings(command, preSubmitState, cryptoRequirements);
        await ensureSignedActionIntent();
        const initialActionProgress = {
          kind: "local_payload",
          title: "Preparing action payload",
          operation: actionMayNeedCryptoMaterial
            ? "Preparing hidden-card material"
            : "Preparing signed action",
          detail:
            cryptoRequirements.length > 0
              ? `${cryptoRequirements.length} crypto requirement${cryptoRequirements.length === 1 ? "" : "s"}`
              : "No hidden-card material required yet",
          actionIntentKey: actionIntentKey(signedActionIntent),
          progressCurrent: 0,
          progressTotal: Math.max(1, cryptoRequirements.length || 1),
        };
        showLocalActionWait(initialActionProgress);
        stopActionIntentProgress = startActionIntentProgressBroadcast(
          signedActionIntent,
          "payload_generation",
          actionMayNeedCryptoMaterial
            ? ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD
            : PROTOCOL_RESPONSE_TIMEOUT_MS,
          {
            operation: initialActionProgress.operation,
            detail: initialActionProgress.detail,
            progressCurrent: initialActionProgress.progressCurrent,
            progressTotal: initialActionProgress.progressTotal,
            title: initialActionProgress.title,
          }
        );
        if (cryptoRequirements.length > 0) {
          setStatus("Preparing cryptographic material for action");
        }
        updateLocalActionProgress({
          kind: "local_payload",
          operation: "Building local shuffle proofs",
          detail: `${cryptoRequirements.length} requirement${cryptoRequirements.length === 1 ? "" : "s"}`,
        }, "payload_generation");
        let shuffleProofs = await timePeerSyncPhase(
          "submit_action:build_local_shuffle_proofs",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
          },
          () => buildLocalShuffleProofsForRequirements(
            cryptoRequirements,
            nextSequence
          )
        );
        recordPeerSyncPerf("submit_action:build_local_shuffle_proofs:summary", {
          ...submitPerf,
          shuffle_proofs: Array.isArray(shuffleProofs) ? shuffleProofs.length : 0,
          bytes: payloadSizeBytes(shuffleProofs),
        });
        updateLocalActionProgress({
          kind: "local_payload",
          operation: "Building random reveals",
          detail: `${cryptoRequirements.length} requirement${cryptoRequirements.length === 1 ? "" : "s"}`,
        }, "payload_generation");
        const rngReveals = await timePeerSyncPhase(
          "submit_action:build_local_rng_reveals",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
          },
          () => buildLocalRngRevealsForRequirements(
            cryptoRequirements,
            nextSequence,
            {
              command,
              actorIndex: session.localPlayerIndex,
              prevStateHash: preActionStateHash,
              publicCheckpointHash: preActionPublicCheckpointHash,
              actionIntent: signedActionIntent,
            }
          )
        );
        recordPeerSyncPerf("submit_action:build_local_rng_reveals:summary", {
          ...submitPerf,
          rng_reveals: Array.isArray(rngReveals) ? rngReveals.length : 0,
          bytes: payloadSizeBytes(rngReveals),
        });
        let actionCryptoOptions = {
          command,
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          prevStateHash: preActionStateHash,
          publicCheckpointHash: preActionPublicCheckpointHash,
          preActionPublicCheckpointHash,
          actionIntent: signedActionIntent,
          requirements: cryptoRequirements,
          uiState: preSubmitState,
          updateState: false,
        };
        await timePeerSyncPhase(
          "submit_action:inject_crypto_material",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
            shuffle_proofs: Array.isArray(shuffleProofs) ? shuffleProofs.length : 0,
            rng_reveals: Array.isArray(rngReveals) ? rngReveals.length : 0,
          },
          () => injectCryptoMaterialForRequirements(cryptoRequirements, {
            shuffleProofs,
            rngReveals,
          }, actionCryptoOptions)
        );
        if (shuffleProofs.length > 0) {
          cryptoRequirements = await timePeerSyncPhase(
            "submit_action:refresh_requirements_after_shuffle",
            submitPerf,
            async () => filterCryptoRequirementsForCommand(
              command,
              preSubmitState,
              freshCryptoRequirementsForSequence(
                nextSequence,
                await previewRequirementsForCommand(command)
              )
            )
          );
          recordPeerSyncPerf("submit_action:refresh_requirements_after_shuffle:summary", {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
          });
          rememberActionCryptoRequirements(nextSequence, cryptoRequirements);
          shuffleProofs = alignShuffleProofsWithRequirements(shuffleProofs, cryptoRequirements);
          requestRemoteCryptoPreview = shouldRequestRemoteCryptoPreview(
            command,
            preSubmitState,
            cryptoRequirements
          );
          actionCryptoOptions = {
            ...actionCryptoOptions,
            requirements: cryptoRequirements,
          };
        }
        let remoteCryptoMaterial = await timePeerSyncPhase(
          "submit_action:collect_remote_crypto_material_pre",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
            request_preview: requestRemoteCryptoPreview,
          },
          () => collectRemoteCryptoMaterialForRequirements(
            cryptoRequirements,
            {
              command,
              seq: nextSequence,
              actorIndex: session.localPlayerIndex,
              requestPreview: requestRemoteCryptoPreview,
              prevStateHash: preActionStateHash,
              publicCheckpointHash: preActionPublicCheckpointHash,
              actionIntent: signedActionIntent,
            }
          )
        );
        updateLocalActionProgress({
          kind: "local_payload",
          operation: "Opening remote hidden-card material",
          detail: `${Array.isArray(remoteCryptoMaterial.openings) ? remoteCryptoMaterial.openings.length : 0} opening${Array.isArray(remoteCryptoMaterial.openings) && remoteCryptoMaterial.openings.length === 1 ? "" : "s"}`,
        }, "crypto_material");
        recordPeerSyncPerf("submit_action:collect_remote_crypto_material_pre:summary", {
          ...submitPerf,
          material: summarizeCryptoMaterialForPerf(remoteCryptoMaterial),
        });
        await timePeerSyncPhase(
          "submit_action:reveal_remote_openings_pre",
          {
            ...submitPerf,
            openings: Array.isArray(remoteCryptoMaterial.openings) ? remoteCryptoMaterial.openings.length : 0,
          },
          () => revealAuditOpenings(remoteCryptoMaterial.openings || [], {
            timing: "pre",
            shuffleProofs,
            updateState: false,
          })
        );
        const preOpenings = await timePeerSyncPhase(
          "submit_action:build_local_openings_pre",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(cryptoRequirements),
          },
          () => buildLocalOpeningsForCommand(command, cryptoRequirements, {
            ...actionCryptoOptions,
            onOpeningBuilt: previewBuiltLocalOpening,
          })
        );
        recordPeerSyncPerf("submit_action:build_local_openings_pre:summary", {
          ...submitPerf,
          openings: Array.isArray(preOpenings) ? preOpenings.length : 0,
          bytes: payloadSizeBytes(preOpenings),
        });
        const publishAppliedStateImmediately = false;
        const expectedPreviousSequence = nextSequence - 1;
        const latestAppliedSequenceBeforeApply = Number(multiplayerRef.current.lastAppliedSequence || 0);
        const latestHistorySequenceBeforeApply = Number(
          actionHistoryRef.current.at(-1)?.seq ?? latestAppliedSequenceBeforeApply
        );
        if (
          awaitingStateResyncRef.current
          || latestAppliedSequenceBeforeApply !== expectedPreviousSequence
          || latestHistorySequenceBeforeApply !== expectedPreviousSequence
        ) {
          cancelLocalActionIntent("Local action was superseded before local apply");
          if (localSubmissionSnapshot) {
            await restoreSequencedActionValidationSnapshotIfCurrent(localSubmissionSnapshot);
          } else if (stagedMatchClockRuntime) {
            restoreMatchClockRuntime(stagedMatchClockRuntime, stateRef.current);
            stagedMatchClockRuntime = null;
          }
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          clearLocalActionWait();
          setStatus("Another action was signed first");
          return;
        }
        const liveStateBeforeApply = gameRef.current ? await gameRef.current.uiState() : stateRef.current;
        if (!isDecisionCommandCompatible(liveStateBeforeApply?.decision, command)) {
          cancelLocalActionIntent("Action is no longer available before local apply");
          if (localSubmissionSnapshot) {
            await restoreSequencedActionValidationSnapshotIfCurrent(localSubmissionSnapshot);
          } else if (stagedMatchClockRuntime) {
            restoreMatchClockRuntime(stagedMatchClockRuntime, stateRef.current);
            stagedMatchClockRuntime = null;
          }
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          clearLocalActionWait();
          setStatus("That action is no longer available");
          return;
        }
        const expectedActorBeforeApply = liveStateBeforeApply?.decision?.player;
        if (
          expectedActorBeforeApply !== null
          && expectedActorBeforeApply !== undefined
          && Number(expectedActorBeforeApply) !== Number(session.localPlayerIndex)
        ) {
          cancelLocalActionIntent("Action actor no longer has priority before local apply");
          if (localSubmissionSnapshot) {
            await restoreSequencedActionValidationSnapshotIfCurrent(localSubmissionSnapshot);
          } else if (stagedMatchClockRuntime) {
            restoreMatchClockRuntime(stagedMatchClockRuntime, stateRef.current);
            stagedMatchClockRuntime = null;
          }
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          clearLocalActionWait();
          setStatus("It is not your turn to act");
          return;
        }
        updateLocalActionProgress({
          kind: "engine_work",
          title: "Resolving action locally",
          operation: "Engine applying command",
          detail: label || summarizePeerCommand(command)?.type || String(command?.type || "action"),
        }, "engine_work", PROTOCOL_RESPONSE_TIMEOUT_MS);
        setStatus("Engine is applying the action locally");
        let appliedState = await timePeerSyncPhase(
          "submit_action:apply_synced_command",
          {
            ...submitPerf,
            publish_state: publishAppliedStateImmediately,
          },
          () => applySyncedCommand(command, label || "", {
            actorIndex: session.localPlayerIndex,
            sequence: nextSequence,
            publishState: publishAppliedStateImmediately,
          })
        );
        const remotePostOpeningState = await timePeerSyncPhase(
          "submit_action:reveal_remote_openings_post",
          {
            ...submitPerf,
            openings: Array.isArray(remoteCryptoMaterial.openings) ? remoteCryptoMaterial.openings.length : 0,
          },
          () => revealAuditOpenings(
            remoteCryptoMaterial.openings || [],
            {
              timing: "post",
              shuffleProofs,
              updateState: false,
            }
          )
        );
        if (remotePostOpeningState) {
          appliedState = remotePostOpeningState;
        }
        const appliedRequirements = await timePeerSyncPhase(
          "submit_action:applied_requirements_from_state",
          submitPerf,
          async () => filterCryptoRequirementsForCommand(
            command,
            preSubmitState,
            freshCryptoRequirementsForSequence(
              nextSequence,
              cryptoRequirementsFromState(appliedState)
            )
          )
        );
        recordPeerSyncPerf("submit_action:applied_requirements_from_state:summary", {
          ...submitPerf,
          requirements: summarizeCryptoRequirementsForPerf(appliedRequirements),
        });
        rememberActionCryptoRequirements(nextSequence, appliedRequirements);
        if (shuffleProofs.length > 0) {
          shuffleProofs = alignShuffleProofsWithRequirements(
            shuffleProofs,
            appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements
          );
        }
        const postShuffleRequirements = missingShuffleRequirements(
          appliedRequirements,
          shuffleProofs
        );
        if (postShuffleRequirements.length > 0) {
          const postShuffleProofs = await timePeerSyncPhase(
            "submit_action:build_local_post_shuffle_proofs",
            {
              ...submitPerf,
              requirements: summarizeCryptoRequirementsForPerf(postShuffleRequirements),
            },
            () => buildLocalShuffleProofsForRequirements(
              postShuffleRequirements,
              nextSequence
            )
          );
          shuffleProofs = mergeShuffleProofs(shuffleProofs, postShuffleProofs);
        }
        await timePeerSyncPhase(
          "submit_action:verify_shuffle_proofs",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf([...cryptoRequirements, ...appliedRequirements]),
            shuffle_proofs: Array.isArray(shuffleProofs) ? shuffleProofs.length : 0,
          },
          () => verifyShuffleProofsForRequirements(
            [...cryptoRequirements, ...appliedRequirements],
            shuffleProofs
          )
        );
        const shuffleApplicationRequirements = [
          ...appliedRequirements,
          ...cryptoRequirements,
        ];
        const localizedShuffleProofs = alignShuffleProofsWithRequirements(
          shuffleProofs,
          shuffleApplicationRequirements
        );
        await timePeerSyncPhase(
          "submit_action:apply_verified_shuffle_proofs",
          {
            ...submitPerf,
            shuffle_proofs: Array.isArray(localizedShuffleProofs) ? localizedShuffleProofs.length : 0,
          },
          () => applyVerifiedShuffleProofs(localizedShuffleProofs)
        );
        await timePeerSyncPhase(
          "submit_action:reveal_local_ziffle_hand",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf([...cryptoRequirements, ...appliedRequirements]),
          },
          () => revealLocalZiffleHand(matchStartPayloadRef.current, {
            skipIfHandUnchanged: true,
            stateHint: appliedState,
            command,
            seq: nextSequence,
            actorIndex: session.localPlayerIndex,
            actionIntent: signedActionIntent,
            requirements: [...cryptoRequirements, ...appliedRequirements],
            updateState: false,
          })
        );
        const openingRequirements = appliedRequirements.length > 0
          ? [...cryptoRequirements, ...appliedRequirements]
          : cryptoRequirements;
        const missingRemotePostOpenRequirements = missingRemotePublicOpenRequirements(
          openingRequirements,
          remoteCryptoMaterial,
          session.localPlayerIndex
        );
        recordPeerSyncPerf("submit_action:missing_remote_post_open_requirements", {
          ...submitPerf,
          requirements: summarizeCryptoRequirementsForPerf(missingRemotePostOpenRequirements),
        });
        if (missingRemotePostOpenRequirements.length > 0) {
          updateLocalActionProgress({
            kind: "crypto_material",
            title: "Requesting post-action openings",
            operation: "Waiting for peer opening material",
            detail:
              `${missingRemotePostOpenRequirements.length} requirement`
              + `${missingRemotePostOpenRequirements.length === 1 ? "" : "s"}`,
          }, "crypto_material");
          const postRemoteCryptoMaterial = await timePeerSyncPhase(
            "submit_action:collect_remote_crypto_material_post",
            {
              ...submitPerf,
              requirements: summarizeCryptoRequirementsForPerf(missingRemotePostOpenRequirements),
            },
            async () => collectRemoteCryptoMaterialForRequirements(
              missingRemotePostOpenRequirements,
              {
                command,
                seq: nextSequence,
                actorIndex: session.localPlayerIndex,
                prevStateHash: preActionStateHash,
                publicCheckpointHash: await ensurePreActionPublicCheckpointHash(),
                actionIntent: await ensureSignedActionIntent(),
              }
            )
          );
          remoteCryptoMaterial = {
            openings: mergeAuditOpenings(
              remoteCryptoMaterial.openings,
              postRemoteCryptoMaterial.openings
            ),
            privateViewProofs: mergePrivateViewProofs(
              remoteCryptoMaterial.privateViewProofs,
              postRemoteCryptoMaterial.privateViewProofs
            ),
          };
          const postRemoteOpeningState = await timePeerSyncPhase(
            "submit_action:reveal_remote_openings_post_missing",
            {
              ...submitPerf,
              openings: Array.isArray(postRemoteCryptoMaterial.openings)
                ? postRemoteCryptoMaterial.openings.length
                : 0,
            },
            () => revealAuditOpenings(postRemoteCryptoMaterial.openings || [], {
              timing: "post",
              shuffleProofs,
              updateState: false,
            })
          );
          if (postRemoteOpeningState) {
            appliedState = postRemoteOpeningState;
          }
        }
        const expectedOpeningPreviewTotal = expectedLocalPublicOpeningPreviewCount(
          openingRequirements,
          session.localPlayerIndex
        );
        localOpeningPreviewTotal = Math.max(
          localOpeningPreviewTotal,
          expectedOpeningPreviewTotal
        );
        updateLocalActionProgress({
          kind: "local_ziffle_reveal",
          title: "Generating reveal openings",
          operation: "Building local public openings",
          detail:
            `${openingRequirements.length} requirement`
            + `${openingRequirements.length === 1 ? "" : "s"}`,
          progressCurrent: 0,
          progressTotal: expectedOpeningPreviewTotal || null,
        }, "opening_generation");
        const postOpenings = await timePeerSyncPhase(
          "submit_action:build_local_openings_post",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(openingRequirements),
          },
	          () => buildLocalOpeningsForCommand(command, openingRequirements, {
	            ...actionCryptoOptions,
	            requirements: openingRequirements,
	            timing: "post",
	            forceZiffleOpeningProof: true,
	            onOpeningBuilt: previewBuiltLocalOpening,
	          })
	        );
        recordPeerSyncPerf("submit_action:build_local_openings_post:summary", {
          ...submitPerf,
          openings: Array.isArray(postOpenings) ? postOpenings.length : 0,
          bytes: payloadSizeBytes(postOpenings),
        });
        const localRequirementOpenings = await timePeerSyncPhase(
          "submit_action:build_local_requirement_openings",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(openingRequirements),
          },
	          () => buildLocalRequirementOpeningsForRequirements(
	            openingRequirements,
	            {
	              ...actionCryptoOptions,
	              requirements: openingRequirements,
	              forceZiffleOpeningProof: true,
	              onOpeningBuilt: previewBuiltLocalOpening,
	            }
	          )
        );
        recordPeerSyncPerf("submit_action:build_local_requirement_openings:summary", {
          ...submitPerf,
          openings: Array.isArray(localRequirementOpenings) ? localRequirementOpenings.length : 0,
          bytes: payloadSizeBytes(localRequirementOpenings),
        });
        const selectedPostOpenings = filterOpeningsForCommandHiddenRefs(postOpenings, command);
        const selectedLocalRequirementOpenings = filterOpeningsForCommandHiddenRefs(
          localRequirementOpenings,
          command,
        );
        const localPostOpeningState = await timePeerSyncPhase(
          "submit_action:reveal_local_openings_post",
          {
            ...submitPerf,
            openings: Array.isArray(selectedPostOpenings) ? selectedPostOpenings.length : 0,
            requirement_openings: Array.isArray(selectedLocalRequirementOpenings)
              ? selectedLocalRequirementOpenings.length
              : 0,
          },
          () => revealAuditOpenings(
            mergeAuditOpenings(selectedPostOpenings, selectedLocalRequirementOpenings),
            {
              timing: "post",
              shuffleProofs,
              updateState: false,
            }
          )
        );
        if (localPostOpeningState) {
          appliedState = localPostOpeningState;
        }
        const openings = mergeAuditOpenings(
          preOpenings,
          selectedPostOpenings,
          selectedLocalRequirementOpenings,
          remoteCryptoMaterial.openings
        );
        const localPrivateViewProofs = await timePeerSyncPhase(
          "submit_action:build_local_private_view_proofs",
          {
            ...submitPerf,
            requirements: summarizeCryptoRequirementsForPerf(
              appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements
            ),
          },
	          () => buildLocalPrivateViewProofsForRequirements(
	            appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements,
	            {
	              ...actionCryptoOptions,
	              liveState: appliedState,
	              uiState: appliedState,
	            }
	          )
	        );
        const privateViewProofs = mergePrivateViewProofs(
          localPrivateViewProofs,
          remoteCryptoMaterial.privateViewProofs
        );
        await timePeerSyncPhase(
          "submit_action:reveal_private_audit_proofs",
          {
            ...submitPerf,
            private_view_proofs: Array.isArray(privateViewProofs) ? privateViewProofs.length : 0,
          },
          () => revealPrivateAuditProofsForLocalViewer({ seq: nextSequence, privateViewProofs }, {
            seq: nextSequence,
            updateState: false,
          })
        );
        const localPublicCheckpointHash = await timePeerSyncPhase(
          "submit_action:current_public_checkpoint_hash",
          submitPerf,
          () => currentPublicAuditCheckpointHash()
        );
        updateLocalActionProgress({
          kind: "local_payload",
          title: "Signing action payload",
          operation: "Building audit payload",
          detail:
            `${Array.isArray(openings) ? openings.length : 0} opening`
            + `${Array.isArray(openings) && openings.length === 1 ? "" : "s"}`,
          progressCurrent: 1,
          progressTotal: 1,
        }, "payload_signing", PROTOCOL_RESPONSE_TIMEOUT_MS);
        setStatus("Generating action verification payload");
        const audit = await timePeerSyncPhase(
          "submit_action:build_sequenced_action_audit",
          {
            ...submitPerf,
            openings: Array.isArray(openings) ? openings.length : 0,
            private_view_proofs: Array.isArray(privateViewProofs) ? privateViewProofs.length : 0,
            shuffle_proofs: Array.isArray(shuffleProofs) ? shuffleProofs.length : 0,
            rng_reveals: Array.isArray(rngReveals) ? rngReveals.length : 0,
            openings_bytes: payloadSizeBytes(openings),
          },
          () => buildSequencedActionAudit({
            seq: nextSequence,
            actorIndex: session.localPlayerIndex,
            command,
            clock,
            openings,
            rngReveals,
            shuffleProofs,
            privateViewProofs,
            publicCheckpointHash: localPublicCheckpointHash,
          })
        );
        const message = {
          type: "apply_action",
          protocolVersion: PROTOCOL_VERSION,
          securityMode: MULTIPLAYER_SECURITY_VERIFIED,
          seq: nextSequence,
          actorIndex: session.localPlayerIndex,
          command,
          label: label || "",
          audit,
        };
        recordPeerSyncPerf("submit_action:message_summary", summarizeSequencedActionForPerf(message));
        await timePeerSyncPhase(
          "submit_action:verify_sequenced_action_audit",
          summarizeSequencedActionForPerf(message),
          () => verifySequencedActionAudit({
            audit,
            seq: nextSequence,
            actorIndex: session.localPlayerIndex,
            command,
          })
        );
        await timePeerSyncPhase(
          "submit_action:verify_audit_satisfies_crypto_requirements",
          {
            ...summarizeSequencedActionForPerf(message),
            requirements: summarizeCryptoRequirementsForPerf(
              appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements
            ),
          },
          () => verifyAuditSatisfiesCryptoRequirements({
            requirements: appliedRequirements.length > 0 ? appliedRequirements : cryptoRequirements,
            audit,
          })
        );
        if (String(audit.publicCheckpointHash || "") !== localPublicCheckpointHash) {
          throw new Error("Local public checkpoint hash does not match signed action");
        }
        clearLocalActionWait();
        setStatus("Waiting for peers to verify action payload");
        const quorumCertificate = await timePeerSyncPhase(
          "submit_action:collect_action_quorum_certificate",
          summarizeSequencedActionForPerf(message),
          () => collectActionQuorumCertificate(message)
        );
        if (quorumCertificate) {
          message.audit = {
            ...message.audit,
            quorumCertificate,
          };
          await verifyActionQuorumForMessage(message);
        }
        commitMatchClockAudit(clock, appliedState);
        await appendAppliedSequencedAction(message);
        localSubmissionCommitted = true;
        if (signedActionIntent) {
          broadcastActionIntentProgress(
            signedActionIntent,
            "action_broadcast",
            actionBroadcastResponseTimeoutMs(message),
            { action: message }
          );
        }
        relaySequencedAction(message);
        recordPeerSyncPerf("submit_action:done", summarizeSequencedActionForPerf(message));
        stopLocalActionIntentProgress();
        await publishCurrentRuntimeState(
          viewedCardsStateHint(localPostOpeningState, remotePostOpeningState, appliedState)
        );
        await drainPendingSequencedActions();
        clearLocalActionWait();
        setStatus("Action signed and broadcast to peers");
      } catch (err) {
        stopLocalActionIntentProgress();
        clearLocalActionWait();
        let restoredLocalSubmissionSnapshot = false;
        if (localSubmissionSnapshot && !localSubmissionCommitted) {
          try {
            restoredLocalSubmissionSnapshot =
              await restoreSequencedActionValidationSnapshotIfCurrent(localSubmissionSnapshot);
          } catch {
            // Preserve the original submit failure below.
          }
        }
        if (
          stagedMatchClockRuntime
          && !localSubmissionSnapshot
          && !restoredLocalSubmissionSnapshot
          && !localSubmissionCommitted
        ) {
          restoreMatchClockRuntime(stagedMatchClockRuntime, stateRef.current);
        }
        if (!restoredLocalSubmissionSnapshot) {
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
        }
        const failureReason = toErrorMessage(err);
        recordPeerSyncPerf("submit_action:error", {
          command: summarizePeerCommand(command),
          error: failureReason,
        });
        if (signedActionIntent && !localSubmissionCommitted) {
          broadcastActionIntentCancel(signedActionIntent, failureReason);
        }
        const protocolTimeoutClaim = protocolResponseTimeoutClaimFromError(err);
        if (protocolTimeoutClaim) {
          await submitProtocolResponseTimeoutClaim(protocolTimeoutClaim);
          return;
        }
        if (
          failureReason.includes("Action quorum certificate")
          || failureReason.includes("action quorum vote")
        ) {
          setStatus(`Action rejected by quorum: ${failureReason}`, true);
          return;
        }
        throw err;
      }
    },
    [
      buildLocalOpeningsForCommand,
      buildLocalPrivateViewProofsForRequirements,
      buildLocalRequirementOpeningsForRequirements,
      buildLocalRngRevealsForRequirements,
      collectRemoteCryptoMaterialForRequirements,
      buildSequencedActionAudit,
      currentPublicAuditCheckpointHash,
      revealAuditOpenings,
      verifySequencedActionAudit,
      verifyAuditSatisfiesCryptoRequirements,
      setStatus,
      updateMultiplayer,
      beginPeerWait,
      clearPeerWait,
      waitForPeerResyncs,
      waitForSubmissionIdle,
    ]
  );

  const submitMultiplayerAddCardCheat = useCallback(
    async ({ playerIndex, cardName, zone = "hand", skipTriggers = false } = {}) => {
      const session = multiplayerRef.current;
      if (!session.matchStarted) {
        setStatus("Match has not started yet", true);
        return;
      }
      if (session.localPlayerIndex == null) {
        setStatus("Local player seat is not assigned", true);
        return;
      }
      if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
        setStatus("Add-card audit cheat is only available in verified matches", true);
        return;
      }
      const name = String(cardName || "").trim();
      if (!name) {
        setStatus("Enter a card name to add", true);
        return;
      }
      const actorIndex = Number(session.localPlayerIndex);
      const nextSequence = Number(session.lastAppliedSequence || 0) + 1;
      const command = {
        type: "add_card_to_zone",
        player: Number(playerIndex ?? actorIndex),
        cardName: name,
        zone: String(zone || "hand"),
        skipTriggers: Boolean(skipTriggers),
        unauthorized: true,
      };
      const audit = await buildSequencedActionAudit({
        seq: nextSequence,
        actorIndex,
        command,
      });
      const message = {
        type: "apply_action",
        protocolVersion: PROTOCOL_VERSION,
        seq: nextSequence,
        actorIndex,
        command,
        label: `Unauthorized add ${name}`,
        audit,
      };
      await verifySequencedActionAudit({
        audit,
        seq: nextSequence,
        actorIndex,
        command,
      });
      relaySequencedAction(message);
      setStatus(`Local add-card cheat broadcast for audit: ${name}`);
    },
    [
      buildSequencedActionAudit,
      setStatus,
      verifySequencedActionAudit,
    ]
  );

  useEffect(() => {
    if (!multiplayer.matchStarted || !matchClockConfigRef.current.initialMs) {
      const idleSnapshot = createMatchClockSnapshot({
        policy: matchClockConfigRef.current,
      });
      matchClockRef.current = {
        policy: matchClockConfigRef.current,
        playerCount: 0,
        baseRemainingMsByPlayer: [],
        activePlayerIndex: null,
        epochStartedAtMs: null,
        clockHash: INITIAL_MATCH_CLOCK_HASH,
        lastSequence: 0,
      };
      timeoutClaimInFlightRef.current = "";
      updateMultiplayer((prev) => {
        const current = prev.matchClock || {};
        if (
          Boolean(current.enabled) === Boolean(idleSnapshot.enabled)
          && Number(current.initialMs || 0) === Number(idleSnapshot.initialMs || 0)
          && current.activePlayerIndex == null
          && current.startedAtMs == null
        ) {
          return prev;
        }
        return {
          ...prev,
          matchClock: idleSnapshot,
          actionTimer: actionTimerSnapshotFromMatchClock(idleSnapshot),
        };
      });
      return undefined;
    }

    let disposed = false;
    const tick = async () => {
      if (disposed) return;
      const session = multiplayerRef.current;
      if (!session.matchStarted || awaitingStateResyncRef.current) return;

      let liveState = stateRef.current;
      try {
        if (gameRef.current && typeof gameRef.current.uiState === "function") {
          liveState = await gameRef.current.uiState();
        }
      } catch {
        liveState = stateRef.current;
      }

      const timer = updateMatchClockForState(liveState);
      if (
        !timer.enabled
        || !timer.expired
        || timer.activePlayerIndex == null
        || session.submittingAction
        || session.localPlayerIndex == null
        || resyncingPeerIdsRef.current.size > 0
      ) {
        return;
      }

      const claimKey = [
        session.lastAppliedSequence,
        timer.activePlayerIndex,
        timer.clockHash,
      ].join(":");
      if (timeoutClaimInFlightRef.current === claimKey) return;
      if (Number(timer.remainingMs ?? 0) > Number(timer.graceMs || 0) + MATCH_CLOCK_CLAIM_SKEW_MS) return;

      timeoutClaimInFlightRef.current = claimKey;
      const playerName = playerNameForIndex(session.players, timer.activePlayerIndex);
      const command = {
        type: "forfeit_player",
        player: Number(timer.activePlayerIndex),
        reason: "peer_claimed_match_clock_timeout",
        timeout_ms: Number(timer.initialMs),
        match_clock_hash: String(timer.clockHash || ""),
        remaining_ms: Number(timer.remainingMs || 0),
        claimed_at_ms: Date.now(),
        basis_sequence: Number(session.lastAppliedSequence || 0),
      };
      try {
        await submitMultiplayerCommand(
          command,
          `${playerName} was peer-claimed inactive after their match clock expired`
        );
      } catch (err) {
        timeoutClaimInFlightRef.current = "";
        const message = err instanceof Error ? err.message : String(err);
        if (!message.includes("Match clock has not expired")) {
          setStatus(`Timeout forfeit failed: ${message}`, true);
          console.error(err);
        }
      }
    };

    void tick();
    const timerId = window.setInterval(() => {
      void tick();
    }, MATCH_CLOCK_TICK_MS);
    return () => {
      disposed = true;
      window.clearInterval(timerId);
    };
  }, [
    multiplayer.matchStarted,
    setStatus,
    submitMultiplayerCommand,
    updateMultiplayer,
  ]);

  useEffect(() => {
    if (!multiplayer.matchStarted) {
      disconnectForfeitInFlightRef.current.clear();
      return undefined;
    }

    let disposed = false;
    const tick = async () => {
      if (disposed) return;
      updateMultiplayer((prev) => {
        const hasDisconnectedPlayers = (prev.players || []).some((player) =>
          player.connected === false && player.disconnectedAtMs
        );
        if (!hasDisconnectedPlayers) return prev;
        const nowMs = Date.now();
        let changed = false;
        const players = (prev.players || []).map((player) => {
          if (player.connected !== false || !player.disconnectedAtMs) return player;
          const autoForfeitAtMs = Number(player.autoForfeitAtMs || 0)
            || Number(player.disconnectedAtMs) + DISCONNECT_AUTO_FORFEIT_MS;
          const disconnectRemainingMs = Math.max(0, autoForfeitAtMs - nowMs);
          if (
            Number(player.autoForfeitAtMs || 0) === autoForfeitAtMs
            && Math.abs(Number(player.disconnectRemainingMs ?? disconnectRemainingMs) - disconnectRemainingMs) < 250
          ) {
            return player;
          }
          changed = true;
          return {
            ...player,
            autoForfeitAtMs,
            disconnectRemainingMs,
          };
        });
        if (!changed) return prev;
        return {
          ...prev,
          players,
        };
      });

      const session = multiplayerRef.current;
      if (
        !session.matchStarted
        || session.submittingAction
        || session.localPlayerIndex == null
        || awaitingStateResyncRef.current
        || resyncingPeerIdsRef.current.size > 0
      ) {
        return;
      }

      const alreadyForfeited = forfeitedPlayersForQuorum();
      const expired = (session.connectionWarnings || [])
        .filter((warning) =>
          !warning.local
          && warning.expired
          && !alreadyForfeited.has(Number(warning.playerIndex))
        )
        .sort((left, right) => Number(left.playerIndex) - Number(right.playerIndex));
      if (expired.length === 0) return;

      const warning = expired[0];
      const claimKey = [
        session.lastAppliedSequence,
        warning.playerIndex,
        warning.disconnectedAtMs,
      ].join(":");
      if (disconnectForfeitInFlightRef.current.has(claimKey)) return;
      disconnectForfeitInFlightRef.current.add(claimKey);
      const playerName = playerNameForIndex(session.players, warning.playerIndex);
      const command = {
        type: "forfeit_player",
        player: Number(warning.playerIndex),
        reason: DISCONNECT_FORFEIT_REASON,
        disconnected_peer_id: String(warning.peerId || ""),
        disconnect_timeout_ms: DISCONNECT_AUTO_FORFEIT_MS,
        disconnected_at_ms: Number(warning.disconnectedAtMs || 0),
        auto_forfeit_at_ms: Number(warning.autoForfeitAtMs || 0),
        claimed_at_ms: Date.now(),
        basis_sequence: Number(session.lastAppliedSequence || 0),
      };
      try {
        await submitMultiplayerCommand(
          command,
          `${playerName} forfeited by unanimous disconnect timeout policy`
        );
      } catch (err) {
        disconnectForfeitInFlightRef.current.delete(claimKey);
        const message = toErrorMessage(err);
        if (!message.includes("Disconnect timeout has not elapsed")) {
          setStatus(`Disconnect timeout policy failed: ${message}`, true);
          console.error(err);
        }
      }
    };

    void tick();
    const timerId = window.setInterval(() => {
      void tick();
    }, 1000);
    return () => {
      disposed = true;
      window.clearInterval(timerId);
    };
  }, [
    multiplayer.matchStarted,
    setStatus,
    submitMultiplayerCommand,
    updateMultiplayer,
  ]);

  const exportAuditTranscript = useCallback(async ({ includeLiveCheckpoint = true } = {}) => {
    if (!liveAuditTranscriptRef.current) return null;
    const transcript = cloneMultiplayerPayload(liveAuditTranscriptRef.current);
    const currentGame = gameRef.current;
    const finalPublicCheckpoint = includeLiveCheckpoint && currentGame
      && typeof currentGame.exportPublicAuditCheckpoint === "function"
      ? await currentGame.exportPublicAuditCheckpoint()
      : null;
    const actions = Array.isArray(transcript.actions) ? transcript.actions : [];
    const finalPublicCheckpointHash = finalPublicCheckpoint
      ? await publicCheckpointHash(finalPublicCheckpoint)
      : String(actions.at(-1)?.audit?.publicCheckpointHash || transcript.initialPublicCheckpointHash || "");
    const finalStateHash = auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH;
    const disputes = Array.isArray(transcript.disputes) ? transcript.disputes : [];
    const matchId = String(transcript.matchId || transcript.match?.auditMatchId || currentAuditMatchId());
	    const privateViewDisclosures = [...privateViewDisclosuresRef.current.values()]
	      .filter((disclosure) =>
	        String(disclosure?.matchId || disclosure?.payload?.matchId || "") === matchId
	      )
	      .map((disclosure) => cloneMultiplayerPayload(disclosure));
	    return {
      ...transcript,
      exportedAt: new Date().toISOString(),
      privateViewDisclosures,
      finalStateHash,
      finalPublicCheckpoint,
      finalPublicCheckpointHash,
      disputes,
      outcome: buildExportedMatchOutcome({
        uiState: stateRef.current,
        finalPublicCheckpoint,
        finalStateHash,
        finalPublicCheckpointHash,
        matchDisputed: multiplayerRef.current.matchDisputed || null,
        disputes,
      }),
    };
  }, [currentAuditMatchId]);

  useEffect(
    () => () => {
      teardownPeer();
    },
    [teardownPeer]
  );

  return {
    multiplayer,
    canStartHostedMatch: canHostedMatchStart(multiplayer),
    createLobby,
    joinLobby,
    leaveLobby,
    startHostedMatch,
    updateLobbyDeck,
    startRematchSideboarding,
    updateRematchDecks,
    readyForRematch,
    submitMultiplayerCommand,
    submitMultiplayerAddCardCheat,
    exportAuditTranscript,
    routePeerIdForPlayer,
  };
}
