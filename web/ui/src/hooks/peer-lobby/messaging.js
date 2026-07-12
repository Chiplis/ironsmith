import {
  CURRENT_AUDIT_MAX_PLAYERS,
  CURRENT_AUDIT_MIN_PLAYERS,
  DEFAULT_OPENING_HAND_SIZE,
  DISCONNECT_AUTO_FORFEIT_MS,
  INITIAL_AUDIT_STATE_HASH,
  MATCH_FORMAT_COMMANDER,
  MATCH_FORMAT_NORMAL,
  MULTIPLAYER_SECURITY_TRUSTED,
  MULTIPLAYER_SECURITY_VERIFIED,
  PEER_CONNECT_TIMEOUT_MS,
  PEER_OPEN_TIMEOUT_MS,
  PROTOCOL_RESPONSE_TIMEOUT_MS,
  PROTOCOL_VERSION,
  Peer,
  actionTimerSnapshotFromMatchClock,
  assertResyncActionsExtendLocalTranscript,
  buildRematchStateFromPayload,
  buildSignedMatchGenesis,
  canHostedMatchStart,
  cloneMultiplayerPayload,
  compactZiffleCeremonyForDiagnostics,
  connectionHeartbeatKey,
  createEmptyState,
  createMatchSeed,
  createOfflinePeerId,
  createPeer,
  deckSlotOpeningsForManifest,
  decklistHashForCards,
  emitSyncFailureNotice,
  enqueueAsync,
  ensurePromotedLocalPlayer,
  findLocalMatchPlayer,
  findNextHostPlayer,
  formatPeerError,
  importAuditPublicKey,
  isCurrentAuditPlayerCount,
  isRecoverablePeerError,
  isSupportedZiffleDeckCount,
  isTrustedMultiplayerSecurityMode,
  isVerifiedMultiplayerSecurityMode,
  markHostPeerDisconnected,
  markPlayerConnectionState,
  matchClockPolicyPayload,
  matchPayloadSecurityMode,
  normalizeMatchFormat,
  normalizeMultiplayerSecurityMode,
  normalizePlayerIndex,
  openDecklistPlayerFields,
  parseDeckSubmission,
  playerCryptoSeatBindingReady,
  playerMatchesPresentedAuditIdentity,
  publicDeckManifest,
  randomAuditHex,
  reindexPlayers,
  rematchPlayersReady,
  rememberDefaultLobbyDeck,
  resolveLocalPlayerIndex,
  resolveReconnectPlayerIndex,
  safeSend,
  sanitizeCardList,
  sanitizeDeckSlotOpenings,
  sanitizePlayerName,
  sessionSecurityMode,
  summarizeMatchValidationIssues,
  toErrorMessage,
  toLobbyPlayers,
  toPublicPlayer,
  toPublicPlayers,
  useCallback,
  validationCommandersForMatchPayload,
  validationDecksForMatchPayload,
  validationSideboardsForMatchPayload,
  verifyLiveAuditTranscript,
  verifySignedMatchGenesis,
  verifySignedResyncEnvelope,
  waitForCryptoSeatBindingsFromSession,
  withDeckState,
  writeStoredPlayerIndex,
} from "./shared.js";

export function usePeerLobbyMessaging(base, servicesRef) {
  const { actionCryptoRequirementsRef, actionHistoryRef, applySyncedCommand, applyingSequencedActionsRef, auditEncryptionPublicKeyRef, auditKeyPairRef, auditPublicKeyRef, auditStateHashRef, awaitingStateResyncRef, clientConnectionsRef, clientMessageQueueRef, drainingPendingSequencedActionsRef, ensureDirectPeerConnectionsRef, gameRef, hostConnectionRef, hostMessageQueueRef, ignoredActionIntentKeysRef, initialPublicCheckpointHashRef, liveAuditTranscriptRef, localZiffleRevealInFlightRef, matchClockConfigRef, matchClockObservationExemptSequenceRef, matchStartPayloadRef, multiplayerRef, peerConnectionsRef, peerMessageQueueRef, peerOptionsRef, peerRef, peerServerLabelRef, pendingSequencedActionsRef, reconnectChallengesRef, relayedActionIdsRef, resyncingPeerIdsRef, setState, setStatus, stateRef } = base;
  const alignMatchClockObservationFromHostSnapshot = useCallback((...args) => servicesRef.current.alignMatchClockObservationFromHostSnapshot(...args), [servicesRef]);
  const answerActionQuorumVoteRequest = useCallback((...args) => servicesRef.current.answerActionQuorumVoteRequest(...args), [servicesRef]);
  const answerCryptoMaterialRequest = useCallback((...args) => servicesRef.current.answerCryptoMaterialRequest(...args), [servicesRef]);
  const answerDisconnectForfeitVoteRequest = useCallback((...args) => servicesRef.current.answerDisconnectForfeitVoteRequest(...args), [servicesRef]);
  const answerProtocolResponseTimeoutVoteRequest = useCallback((...args) => servicesRef.current.answerProtocolResponseTimeoutVoteRequest(...args), [servicesRef]);
  const answerRngCommitRequest = useCallback((...args) => servicesRef.current.answerRngCommitRequest(...args), [servicesRef]);
  const answerRngRevealRequest = useCallback((...args) => servicesRef.current.answerRngRevealRequest(...args), [servicesRef]);
  const answerTimeoutVoteRequest = useCallback((...args) => servicesRef.current.answerTimeoutVoteRequest(...args), [servicesRef]);
  const answerZiffleRevealTokenRequest = useCallback((...args) => servicesRef.current.answerZiffleRevealTokenRequest(...args), [servicesRef]);
  const answerZiffleShuffleStepRequest = useCallback((...args) => servicesRef.current.answerZiffleShuffleStepRequest(...args), [servicesRef]);
  const applyMatchStart = useCallback((...args) => servicesRef.current.applyMatchStart(...args), [servicesRef]);
  const applySequencedActionMessage = useCallback((...args) => servicesRef.current.applySequencedActionMessage(...args), [servicesRef]);
  const assertZiffleShuffleProofBoundToSignedMatch = useCallback((...args) => servicesRef.current.assertZiffleShuffleProofBoundToSignedMatch(...args), [servicesRef]);
  const broadcastMatchPresence = useCallback((...args) => servicesRef.current.broadcastMatchPresence(...args), [servicesRef]);
  const broadcastToClients = useCallback((...args) => servicesRef.current.broadcastToClients(...args), [servicesRef]);
  const buildHostedResyncPayload = useCallback((...args) => servicesRef.current.buildHostedResyncPayload(...args), [servicesRef]);
  const buildLocalDeckAuditManifest = useCallback((...args) => servicesRef.current.buildLocalDeckAuditManifest(...args), [servicesRef]);
  const buildZiffleCeremoniesForPayload = useCallback((...args) => servicesRef.current.buildZiffleCeremoniesForPayload(...args), [servicesRef]);
  const clearAllConnectionHeartbeats = useCallback((...args) => servicesRef.current.clearAllConnectionHeartbeats(...args), [servicesRef]);
  const clearAllPeerResyncs = useCallback((...args) => servicesRef.current.clearAllPeerResyncs(...args), [servicesRef]);
  const clearAllPendingActionIntents = useCallback((...args) => servicesRef.current.clearAllPendingActionIntents(...args), [servicesRef]);
  const clearConnectionHeartbeat = useCallback((...args) => servicesRef.current.clearConnectionHeartbeat(...args), [servicesRef]);
  const clearLocalDisconnectObservation = useCallback((...args) => servicesRef.current.clearLocalDisconnectObservation(...args), [servicesRef]);
  const createSequencedActionValidationSnapshot = useCallback((...args) => servicesRef.current.createSequencedActionValidationSnapshot(...args), [servicesRef]);
  const currentAuditMatchId = useCallback((...args) => servicesRef.current.currentAuditMatchId(...args), [servicesRef]);
  const emitZiffleDiagnosticNotice = useCallback((...args) => servicesRef.current.emitZiffleDiagnosticNotice(...args), [servicesRef]);
  const ensureAuditIdentity = useCallback((...args) => servicesRef.current.ensureAuditIdentity(...args), [servicesRef]);
  const ensureDirectPeerConnections = useCallback((...args) => servicesRef.current.ensureDirectPeerConnections(...args), [servicesRef]);
  const ensureZiffleIdentity = useCallback((...args) => servicesRef.current.ensureZiffleIdentity(...args), [servicesRef]);
  const finishPeerResync = useCallback((...args) => servicesRef.current.finishPeerResync(...args), [servicesRef]);
  const handleActionIntentCancelMessage = useCallback((...args) => servicesRef.current.handleActionIntentCancelMessage(...args), [servicesRef]);
  const handleActionIntentProgressMessage = useCallback((...args) => servicesRef.current.handleActionIntentProgressMessage(...args), [servicesRef]);
  const handleConnectionHeartbeatMessage = useCallback((...args) => servicesRef.current.handleConnectionHeartbeatMessage(...args), [servicesRef]);
  const leaveLobby = useCallback((...args) => servicesRef.current.leaveLobby(...args), [servicesRef]);
  const makeZiffleRequestId = useCallback((...args) => servicesRef.current.makeZiffleRequestId(...args), [servicesRef]);
  const markConnectionAlive = useCallback((...args) => servicesRef.current.markConnectionAlive(...args), [servicesRef]);
  const openZiffleRoute = useCallback((...args) => servicesRef.current.openZiffleRoute(...args), [servicesRef]);
  const privateDeckManifestForOwner = useCallback((...args) => servicesRef.current.privateDeckManifestForOwner(...args), [servicesRef]);
  const publicZiffleKey = useCallback((...args) => servicesRef.current.publicZiffleKey(...args), [servicesRef]);
  const rememberLocalDisconnectObservation = useCallback((...args) => servicesRef.current.rememberLocalDisconnectObservation(...args), [servicesRef]);
  const resolveActionQuorumVote = useCallback((...args) => servicesRef.current.resolveActionQuorumVote(...args), [servicesRef]);
  const resolveCryptoMaterial = useCallback((...args) => servicesRef.current.resolveCryptoMaterial(...args), [servicesRef]);
  const resolveRngCommit = useCallback((...args) => servicesRef.current.resolveRngCommit(...args), [servicesRef]);
  const resolveRngReveal = useCallback((...args) => servicesRef.current.resolveRngReveal(...args), [servicesRef]);
  const resolveTimeoutVote = useCallback((...args) => servicesRef.current.resolveTimeoutVote(...args), [servicesRef]);
  const resolveZiffleRevealToken = useCallback((...args) => servicesRef.current.resolveZiffleRevealToken(...args), [servicesRef]);
  const resolveZiffleShuffleStep = useCallback((...args) => servicesRef.current.resolveZiffleShuffleStep(...args), [servicesRef]);
  const restoreMatchClockRuntimeFromActionTranscript = useCallback((...args) => servicesRef.current.restoreMatchClockRuntimeFromActionTranscript(...args), [servicesRef]);
  const restoreSequencedActionValidationSnapshot = useCallback((...args) => servicesRef.current.restoreSequencedActionValidationSnapshot(...args), [servicesRef]);
  const revealAuditOpenings = useCallback((...args) => servicesRef.current.revealAuditOpenings(...args), [servicesRef]);
  const revealLocalZiffleHand = useCallback((...args) => servicesRef.current.revealLocalZiffleHand(...args), [servicesRef]);
  const routePeerIdForPlayer = useCallback((...args) => servicesRef.current.routePeerIdForPlayer(...args), [servicesRef]);
  const sendHostedStateMessage = useCallback((...args) => servicesRef.current.sendHostedStateMessage(...args), [servicesRef]);
  const sendMatchStartToClients = useCallback((...args) => servicesRef.current.sendMatchStartToClients(...args), [servicesRef]);
  const shouldSuppressProtocolMessageError = useCallback((...args) => servicesRef.current.shouldSuppressProtocolMessageError(...args), [servicesRef]);
  const signPlayerGenesis = useCallback((...args) => servicesRef.current.signPlayerGenesis(...args), [servicesRef]);
  const signReconnectProofForChallenge = useCallback((...args) => servicesRef.current.signReconnectProofForChallenge(...args), [servicesRef]);
  const startConnectionHeartbeat = useCallback((...args) => servicesRef.current.startConnectionHeartbeat(...args), [servicesRef]);
  const teardownPeer = useCallback((...args) => servicesRef.current.teardownPeer(...args), [servicesRef]);
  const updateMultiplayer = useCallback((...args) => servicesRef.current.updateMultiplayer(...args), [servicesRef]);
  const verifyCurrentPublicCheckpointHash = useCallback((...args) => servicesRef.current.verifyCurrentPublicCheckpointHash(...args), [servicesRef]);
  const verifyReconnectProofForChallenge = useCallback((...args) => servicesRef.current.verifyReconnectProofForChallenge(...args), [servicesRef]);
  const verifySequencedActionAudit = useCallback((...args) => servicesRef.current.verifySequencedActionAudit(...args), [servicesRef]);
  const waitForSubmissionIdle = useCallback((...args) => servicesRef.current.waitForSubmissionIdle(...args), [servicesRef]);
  const ziffleRoutePeerCandidates = useCallback((...args) => servicesRef.current.ziffleRoutePeerCandidates(...args), [servicesRef]);
  const ziffleTokensForPosition = useCallback((...args) => servicesRef.current.ziffleTokensForPosition(...args), [servicesRef]);
  const requestResync = useCallback((reason = "Resyncing with host...") => {
    const session = multiplayerRef.current;
    if (session.role !== "client" || !session.matchStarted) return false;
    const conn = hostConnectionRef.current;
    if (!conn || conn.open === false) return false;

    updateMultiplayer((prev) => ({
      ...prev,
      submittingAction: false,
    }));
    awaitingStateResyncRef.current = true;
    safeSend(conn, {
      type: "resync_request",
      protocolVersion: PROTOCOL_VERSION,
      lastSequence: session.lastAppliedSequence,
    });
    setStatus(reason, true);
    return true;
  }, [setStatus, updateMultiplayer]);

  const reportSyncFailure = useCallback(
    (body, resyncReason = "", fallbackStatus = body) => {
      emitSyncFailureNotice("Sync failed", body);
      if (resyncReason && requestResync(resyncReason)) {
        return true;
      }
      setStatus(fallbackStatus, true);
      return false;
    },
    [requestResync, setStatus]
  );

  const applyStateResync = useCallback(
    async (message) => {
      awaitingStateResyncRef.current = true;
      if (multiplayerRef.current.submittingAction) {
        setStatus("Waiting for local action to settle before resync");
        const idle = await waitForSubmissionIdle(PROTOCOL_RESPONSE_TIMEOUT_MS);
        if (!idle) {
          throw new Error("Timed out waiting for local action to settle before resync");
        }
      }
      const matchPayload = message?.match;
      if (!matchPayload || typeof matchPayload !== "object") {
        throw new Error("Resync payload is missing match state");
      }
      if (!message?.checkpoint || typeof message.checkpoint !== "object") {
        throw new Error("Resync payload is missing WASM checkpoint");
      }

      const currentGame = gameRef.current;
      if (!currentGame || typeof currentGame.startMatch !== "function") {
        throw new Error("Game engine cannot replay a resync transcript");
      }
      if (
        typeof currentGame.exportSyncCheckpoint !== "function"
        || typeof currentGame.importSyncCheckpoint !== "function"
      ) {
        throw new Error("Game engine cannot sandbox a resync replay");
      }

      const currentSession = multiplayerRef.current;
      const resyncPlayers = Array.isArray(matchPayload.currentPlayers)
        ? matchPayload.currentPlayers
        : matchPayload.players || [];
      const localEntry = findLocalMatchPlayer(
        resyncPlayers,
        currentSession,
        auditPublicKeyRef.current || "",
        auditEncryptionPublicKeyRef.current || ""
      );

      if (!localEntry) {
        throw new Error("Local player is missing from the resync payload");
      }

	      const actionEntries = Array.isArray(message?.actions)
	        ? [...message.actions].sort((left, right) => Number(left?.seq || 0) - Number(right?.seq || 0))
	        : [];
	      const remoteFinalSequence = Number(actionEntries.at(-1)?.seq ?? message?.lastSequence ?? 0);
	      const localLastSequence = Number(currentSession.lastAppliedSequence || 0);
	      if (
	        Number.isSafeInteger(remoteFinalSequence)
	        && Number.isSafeInteger(localLastSequence)
	        && remoteFinalSequence < localLastSequence
	      ) {
	        awaitingStateResyncRef.current = false;
	        safeSend(hostConnectionRef.current, {
	          type: "resync_ack",
	          protocolVersion: PROTOCOL_VERSION,
	          lastSequence: localLastSequence,
	        });
	        return;
	      }
	      const continuity = assertResyncActionsExtendLocalTranscript({
	        actionEntries,
	        localActions: actionHistoryRef.current,
	        localLastSequence: currentSession.lastAppliedSequence,
	      });
      const messageLastSequence = Number(message?.lastSequence ?? continuity.finalSequence);
      if (messageLastSequence !== continuity.finalSequence) {
        throw new Error("Resync message last sequence does not match action transcript");
      }
      const securityMode = matchPayloadSecurityMode(
        matchPayload,
        sessionSecurityMode(currentSession, MULTIPLAYER_SECURITY_VERIFIED)
      );
      if (isTrustedMultiplayerSecurityMode(securityMode)) {
        const payloadHostPeerId = String(
          matchPayload.currentHostPeerId
          || matchPayload.hostPeerId
          || ""
        ).trim();
        const expectedHostPeerId = String(currentSession.hostPeerId || "").trim();
        if (payloadHostPeerId && expectedHostPeerId && payloadHostPeerId !== expectedHostPeerId) {
          throw new Error("Resync payload was not sent by the current match host");
        }
        const currentHostPlayer = (resyncPlayers || []).find(
          (player) => String(player?.peerId || "").trim() === (payloadHostPeerId || expectedHostPeerId)
            || String(player?.currentPeerId || "").trim() === (payloadHostPeerId || expectedHostPeerId)
        );
        const expectedHostSeat = normalizePlayerIndex(matchPayload.currentHostPlayerIndex)
          ?? normalizePlayerIndex(currentHostPlayer?.index)
          ?? 0;
        await currentGame.importSyncCheckpoint(
          message.checkpoint,
          localEntry.index ?? currentSession.localPlayerIndex ?? 0
        );
        if (typeof currentGame.setPerspective === "function") {
          await currentGame.setPerspective(localEntry.index);
        }
        const nextState = typeof currentGame.uiState === "function"
          ? await currentGame.uiState()
          : stateRef.current;
        stateRef.current = nextState;
        setState(nextState);
        restoreMatchClockRuntimeFromActionTranscript(actionEntries, nextState, matchPayload);
        const matchClock = alignMatchClockObservationFromHostSnapshot(
          matchPayload.currentMatchClock,
          nextState
        );
        matchClockObservationExemptSequenceRef.current = messageLastSequence + 1;
        const acceptedMatchPayload = {
          ...cloneMultiplayerPayload(matchPayload),
          securityMode: MULTIPLAYER_SECURITY_TRUSTED,
        };
        const acceptedSessionPlayersBase = Array.isArray(matchPayload.currentPlayers)
          ? cloneMultiplayerPayload(matchPayload.currentPlayers)
          : acceptedMatchPayload.players || [];
        const acceptedSessionPlayers = acceptedSessionPlayersBase.map((player) =>
          expectedHostSeat != null && Number(player?.index) === Number(expectedHostSeat)
            ? {
                ...player,
                currentPeerId: payloadHostPeerId || expectedHostPeerId || player.currentPeerId,
                connected: true,
              }
            : player
        );
        acceptedMatchPayload.currentHostPeerId =
          payloadHostPeerId || expectedHostPeerId || acceptedMatchPayload.currentHostPeerId || "";
        acceptedMatchPayload.currentHostPlayerIndex = expectedHostSeat;
        acceptedMatchPayload.currentPlayers = cloneMultiplayerPayload(acceptedSessionPlayers);
        matchStartPayloadRef.current = cloneMultiplayerPayload(acceptedMatchPayload);
        actionHistoryRef.current = actionEntries.map((entry) => ({
          ...cloneMultiplayerPayload(entry),
          securityMode: normalizeMultiplayerSecurityMode(
            entry.securityMode,
            MULTIPLAYER_SECURITY_TRUSTED
          ),
        }));
        actionCryptoRequirementsRef.current.clear();
        relayedActionIdsRef.current.clear();
        applyingSequencedActionsRef.current.clear();
        pendingSequencedActionsRef.current.clear();
        drainingPendingSequencedActionsRef.current = false;
        localZiffleRevealInFlightRef.current = null;
        auditStateHashRef.current = INITIAL_AUDIT_STATE_HASH;
        initialPublicCheckpointHashRef.current =
          acceptedMatchPayload.initialPublicCheckpointHash
          || initialPublicCheckpointHashRef.current
          || "";
        liveAuditTranscriptRef.current = {
          version: 1,
          kind: "ironsmith-live-browser-trusted-log-v1",
          securityMode: MULTIPLAYER_SECURITY_TRUSTED,
          match: cloneMultiplayerPayload(acceptedMatchPayload),
          matchId: acceptedMatchPayload.auditMatchId || currentAuditMatchId(),
          lobbyId: acceptedMatchPayload.lobbyId || acceptedMatchPayload.hostPeerId || "",
          protocolVersion: PROTOCOL_VERSION,
          signatureAlgorithm: "none",
          genesis: null,
          initialStateHash: INITIAL_AUDIT_STATE_HASH,
          initialPublicCheckpointHash: initialPublicCheckpointHashRef.current,
          actions: actionHistoryRef.current.map((entry) => cloneMultiplayerPayload(entry)),
        };
        clearAllPendingActionIntents();
        ignoredActionIntentKeysRef.current.clear();
        writeStoredPlayerIndex(
          acceptedMatchPayload.lobbyId || acceptedMatchPayload.hostPeerId,
          localEntry.index
        );
        updateMultiplayer((prev) => ({
          ...prev,
          role: acceptedMatchPayload.hostPeerId === prev.localPeerId ? "host" : prev.role,
          lobbyId: acceptedMatchPayload.lobbyId || prev.lobbyId,
          hostPeerId:
            acceptedMatchPayload.currentHostPeerId
            || acceptedMatchPayload.hostPeerId
            || prev.hostPeerId,
          desiredPlayers: acceptedMatchPayload.players?.length ?? prev.desiredPlayers,
          startingLife: Number(acceptedMatchPayload.startingLife || prev.startingLife || 20),
          format: normalizeMatchFormat(acceptedMatchPayload.format || prev.format),
          securityMode: MULTIPLAYER_SECURITY_TRUSTED,
          localPlayerIndex: localEntry.index ?? prev.localPlayerIndex,
          players: acceptedSessionPlayers.length > 0 ? acceptedSessionPlayers : prev.players,
          lastAppliedSequence: messageLastSequence,
          submittingAction: false,
          matchStarted: true,
          mode: "in_match",
          matchClock,
          actionTimer: actionTimerSnapshotFromMatchClock(matchClock),
        }));
        ensureDirectPeerConnections(acceptedSessionPlayers);
        setStatus(
          actionEntries.length > 0
            ? `Resynced with trusted host at action ${messageLastSequence}`
            : "Resynced with trusted host",
        );
        awaitingStateResyncRef.current = false;
        safeSend(hostConnectionRef.current, {
          type: "resync_ack",
          protocolVersion: PROTOCOL_VERSION,
          lastSequence: messageLastSequence,
        });
        return;
      }
      await verifySignedMatchGenesis(matchPayload);
      const verifyTranscriptShuffleProof = async (proof) => {
        if (!currentGame || typeof currentGame.ziffleVerifyShuffle !== "function") {
          throw new Error("Ziffle mental-poker backend is not available");
        }
        assertZiffleShuffleProofBoundToSignedMatch(proof, null, { matchPayload });
        const verified = await currentGame.ziffleVerifyShuffle({
          deckCount: Number(proof.deckCount),
          context: String(proof.context || ""),
          keyContext: String(proof.keyContext || proof.context || ""),
          keys: cloneMultiplayerPayload(proof.keys || []),
          steps: cloneMultiplayerPayload(proof.steps || []),
        });
        if (String(verified.deckHash || "") !== String(proof.deckHash || "")) {
          throw new Error(`Ziffle shuffle proof mismatch for player ${Number(proof.owner) + 1}`);
        }
      };
      const verifyTranscriptZiffleOpening = async ({ proof, ceremony }) => {
        if (!currentGame || typeof currentGame.ziffleRevealCard !== "function") {
          throw new Error("Ziffle mental-poker backend is not available");
        }
        const reveal = await currentGame.ziffleRevealCard({
          deckCount: Number(ceremony.deckCount),
          context: String(ceremony.context || ""),
          keyContext: String(ceremony.keyContext || ceremony.context || ""),
          keys: cloneMultiplayerPayload(ceremony.keys || []),
          steps: cloneMultiplayerPayload(ceremony.steps || []),
          cardPosition: Number(proof.position),
          tokens: ziffleTokensForPosition(proof.tokens || [], proof.position),
        });
        return { originalSlot: Number(reveal.originalSlot) };
      };
      const transcriptReport = await verifyLiveAuditTranscript({
        version: 1,
        kind: "ironsmith-live-browser-audit-v1",
        match: cloneMultiplayerPayload(matchPayload),
        matchId: matchPayload.auditMatchId || currentAuditMatchId(),
        lobbyId: matchPayload.lobbyId || matchPayload.hostPeerId || "",
        protocolVersion: PROTOCOL_VERSION,
        signatureAlgorithm: "ecdsa-p256-sha256",
        genesis: cloneMultiplayerPayload(matchPayload.genesis),
        initialStateHash: INITIAL_AUDIT_STATE_HASH,
        initialPublicCheckpointHash: matchPayload.initialPublicCheckpointHash || "",
        actions: actionEntries.map((entry) => cloneMultiplayerPayload(entry)),
      }, globalThis.crypto, {
        verifyShuffleProof: verifyTranscriptShuffleProof,
        verifyZiffleOpening: verifyTranscriptZiffleOpening,
        requireEngineReplay: false,
        requirePrivateViewDisclosures: false,
      });
      const payloadHostPeerId = String(
        matchPayload.currentHostPeerId
        || matchPayload.hostPeerId
        || ""
      ).trim();
      const expectedHostPeerId = String(currentSession.hostPeerId || "").trim();
      if (payloadHostPeerId && expectedHostPeerId && payloadHostPeerId !== expectedHostPeerId) {
        throw new Error("Resync payload was not sent by the current match host");
      }
      const currentHostPlayer = (resyncPlayers || []).find(
        (player) => String(player?.peerId || "").trim() === (payloadHostPeerId || expectedHostPeerId)
          || String(player?.currentPeerId || "").trim() === (payloadHostPeerId || expectedHostPeerId)
      );
      const expectedHostSeat = normalizePlayerIndex(matchPayload.currentHostPlayerIndex)
        ?? normalizePlayerIndex(currentHostPlayer?.index)
        ?? normalizePlayerIndex(matchPayload.genesis?.hostSeat)
        ?? 0;
      const resyncSigner = normalizePlayerIndex(message?.resyncEnvelope?.signer) ?? expectedHostSeat;
      if (resyncSigner !== expectedHostSeat) {
        throw new Error("Resync envelope was not signed by the match host");
      }
      const signerPlayer = (matchPayload.players || []).find(
        (player) => Number(player.index) === resyncSigner
      );
      const signerKey = await importAuditPublicKey(String(signerPlayer?.auditPublicKey || ""));
      await verifySignedResyncEnvelope({
        envelope: message.resyncEnvelope,
        publicKey: signerKey,
        checkpoint: message.checkpoint,
        actions: actionEntries,
      });
      if (Number(message.resyncEnvelope?.lastSequence ?? continuity.finalSequence) !== continuity.finalSequence) {
        throw new Error("Resync envelope last sequence does not match action transcript");
      }
      if (
        message.resyncEnvelope
        && String(message.resyncEnvelope.finalStateHash || "") !== String(transcriptReport.finalStateHash || "")
      ) {
        throw new Error("Resync transcript final hash does not match signed envelope");
      }
      const finalAction = actionEntries.at(-1);
      const finalPublicCheckpointHash = finalAction?.audit?.publicCheckpointHash || "";
      if (actionEntries.length > 0 && !finalPublicCheckpointHash) {
        throw new Error("Resync transcript is missing final public checkpoint hash");
      }
      const expectedPublicCheckpointHash = actionEntries.length > 0
        ? finalPublicCheckpointHash
        : (
          matchPayload.initialPublicCheckpointHash
          || initialPublicCheckpointHashRef.current
          || ""
        );

      const validationSnapshot = await createSequencedActionValidationSnapshot();
      const multiplayerSnapshot = cloneMultiplayerPayload(currentSession);
      const replayMatchPayload = cloneMultiplayerPayload(matchPayload);
      let nextState;
      try {
        await applyMatchStart(replayMatchPayload, {
          deferLocalZiffleReveal: true,
        });
        for (const action of actionEntries) {
          await applySequencedActionMessage(cloneMultiplayerPayload(action), {
            relay: false,
            allowWhileAwaitingResync: true,
            throwOnOrderMismatch: true,
            throwOnFailure: true,
            failureResyncReason: "",
            enforceMatchClockObservationBounds: false,
          });
        }
        nextState = typeof currentGame.uiState === "function"
          ? await currentGame.uiState()
          : stateRef.current;
        if (expectedPublicCheckpointHash) {
          await verifyCurrentPublicCheckpointHash(
            expectedPublicCheckpointHash,
            "Resync replay public state does not match the signed action transcript"
          );
        }
      } catch (err) {
        try {
          await restoreSequencedActionValidationSnapshot(validationSnapshot);
          updateMultiplayer(() => ({
            ...multiplayerSnapshot,
            submittingAction: false,
          }));
        } catch {
          // Keep the resync verification error as the actionable failure.
        }
        awaitingStateResyncRef.current = false;
        throw err;
      }
      setState(nextState);
      const matchClock = alignMatchClockObservationFromHostSnapshot(
        matchPayload.currentMatchClock,
        nextState
      );

      const lastSequence = continuity.finalSequence;
      matchClockObservationExemptSequenceRef.current = lastSequence + 1;
      const acceptedMatchPayload = matchStartPayloadRef.current
        ? cloneMultiplayerPayload(matchStartPayloadRef.current)
        : cloneMultiplayerPayload(replayMatchPayload);
      const acceptedSessionPlayersBase = Array.isArray(matchPayload.currentPlayers)
        ? cloneMultiplayerPayload(matchPayload.currentPlayers)
        : acceptedMatchPayload.players || [];
      const acceptedSessionPlayers = acceptedSessionPlayersBase.map((player) =>
        expectedHostSeat != null && Number(player?.index) === Number(expectedHostSeat)
          ? {
              ...player,
              currentPeerId: payloadHostPeerId || expectedHostPeerId || player.currentPeerId,
              connected: true,
            }
          : player
      );
      matchStartPayloadRef.current = cloneMultiplayerPayload(acceptedMatchPayload);
      matchStartPayloadRef.current.currentHostPeerId =
        payloadHostPeerId || expectedHostPeerId || acceptedMatchPayload.currentHostPeerId || "";
      matchStartPayloadRef.current.currentHostPlayerIndex = expectedHostSeat;
      matchStartPayloadRef.current.currentPlayers = cloneMultiplayerPayload(acceptedSessionPlayers);
      actionHistoryRef.current = actionEntries.map((entry) => cloneMultiplayerPayload(entry));
      relayedActionIdsRef.current.clear();
      applyingSequencedActionsRef.current.clear();
      pendingSequencedActionsRef.current.clear();
      drainingPendingSequencedActionsRef.current = false;
      localZiffleRevealInFlightRef.current = null;
      auditStateHashRef.current =
        actionEntries.at(-1)?.audit?.nextStateHash || INITIAL_AUDIT_STATE_HASH;
      initialPublicCheckpointHashRef.current =
        acceptedMatchPayload.initialPublicCheckpointHash
        || transcriptReport.initialPublicCheckpointHash
        || initialPublicCheckpointHashRef.current;
      liveAuditTranscriptRef.current = {
        version: 1,
        kind: "ironsmith-live-browser-audit-v1",
        match: cloneMultiplayerPayload(acceptedMatchPayload),
        matchId: acceptedMatchPayload.auditMatchId || currentAuditMatchId(),
        lobbyId: acceptedMatchPayload.lobbyId || acceptedMatchPayload.hostPeerId || "",
        protocolVersion: PROTOCOL_VERSION,
        signatureAlgorithm: "ecdsa-p256-sha256",
        genesis: cloneMultiplayerPayload(acceptedMatchPayload.genesis),
        initialStateHash: INITIAL_AUDIT_STATE_HASH,
        initialPublicCheckpointHash: initialPublicCheckpointHashRef.current,
        actions: actionHistoryRef.current.map((entry) => cloneMultiplayerPayload(entry)),
      };
      clearAllPendingActionIntents();
      ignoredActionIntentKeysRef.current.clear();
      writeStoredPlayerIndex(
        acceptedMatchPayload.lobbyId || acceptedMatchPayload.hostPeerId,
        localEntry.index
      );
      updateMultiplayer((prev) => ({
        ...prev,
        role: acceptedMatchPayload.hostPeerId === prev.localPeerId ? "host" : prev.role,
        lobbyId: acceptedMatchPayload.lobbyId || prev.lobbyId,
        hostPeerId: matchPayload.currentHostPeerId || acceptedMatchPayload.hostPeerId || prev.hostPeerId,
        desiredPlayers: acceptedMatchPayload.players?.length ?? prev.desiredPlayers,
        startingLife: Number(acceptedMatchPayload.startingLife || prev.startingLife || 20),
        format: normalizeMatchFormat(acceptedMatchPayload.format || prev.format),
        securityMode,
        localPlayerIndex: localEntry.index ?? prev.localPlayerIndex,
        players: acceptedSessionPlayers.length > 0 ? acceptedSessionPlayers : prev.players,
        lastAppliedSequence: lastSequence,
        submittingAction: false,
        matchStarted: true,
        mode: "in_match",
        matchClock,
        actionTimer: actionTimerSnapshotFromMatchClock(matchClock),
      }));
      ensureDirectPeerConnections(acceptedSessionPlayers);
      setStatus(
        actionEntries.length > 0
          ? `Resynced with host at action ${lastSequence}`
          : "Resynced with host",
      );
      awaitingStateResyncRef.current = false;
      await revealLocalZiffleHand(acceptedMatchPayload);

      safeSend(hostConnectionRef.current, {
        type: "resync_ack",
        protocolVersion: PROTOCOL_VERSION,
        lastSequence,
      });
    },
    [
      assertZiffleShuffleProofBoundToSignedMatch,
      alignMatchClockObservationFromHostSnapshot,
      currentAuditMatchId,
      setState,
      setStatus,
      updateMultiplayer,
      waitForSubmissionIdle,
      verifyCurrentPublicCheckpointHash,
      applyMatchStart,
      ensureDirectPeerConnections,
    ]
  );

  const broadcastLobbyState = useCallback(() => {
    const session = multiplayerRef.current;
    if (session.role !== "host") return;
    broadcastToClients({
      type: "lobby_state",
      protocolVersion: PROTOCOL_VERSION,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      desiredPlayers: session.desiredPlayers,
      startingLife: session.startingLife,
      format: session.format,
      securityMode: sessionSecurityMode(session),
      players: toLobbyPlayers(session.players),
      matchStarted: session.matchStarted,
    });
  }, [broadcastToClients]);

  const startTrustedMatchFromPlayers = useCallback(async (rawPlayers, options = {}) => {
    const session = multiplayerRef.current;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.startMatch !== "function") {
      setStatus("Game engine is not ready for multiplayer", true);
      return;
    }
    const players = reindexPlayers(rawPlayers || session.players);
    if (!isCurrentAuditPlayerCount(players.length)) {
      setStatus("Trusted multiplayer requires 2, 3, or 4 players", true);
      return;
    }
    const format = normalizeMatchFormat(session.format);
    const sideboards = players.map((player) => sanitizeCardList(player.sideboard));
    const commanders =
      format === MATCH_FORMAT_COMMANDER
        ? players.map((player) => sanitizeCardList(player.commanders))
        : undefined;
    const payloadPlayers = players.map((player) => ({
      ...toPublicPlayer({
        ...player,
        auditPublicKey: "",
        auditEncryptionPublicKey: "",
        playerGenesisSignature: null,
        deckAuditManifest: null,
        deckSlotOpenings: [],
        ziffleKey: null,
      }),
      ready: false,
    }));
    const payload = {
      type: "match_start",
      protocolVersion: PROTOCOL_VERSION,
      securityMode: MULTIPLAYER_SECURITY_TRUSTED,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      players: payloadPlayers,
      format,
      openDecklists: true,
      decks: players.map((player) => sanitizeCardList(player.deck)),
      sideboards,
      commanders,
      startingLife: session.startingLife,
      openingHandSize: DEFAULT_OPENING_HAND_SIZE,
      timeoutMs: matchClockConfigRef.current.initialMs,
      matchClockPolicy: matchClockPolicyPayload(matchClockConfigRef.current),
      auditMatchId: session.lobbyId || session.localPeerId,
    };
    payload.seed = createMatchSeed(payload);

    updateMultiplayer((prev) => ({
      ...prev,
      mode: "starting",
      ...(options.rematch
        ? {
            rematch: {
              ...(prev.rematch || {}),
              phase: "starting",
              players,
            },
          }
        : { players }),
    }));

    try {
      if (typeof currentGame.validateMatchConfig === "function") {
        const validation = await currentGame.validateMatchConfig({
          playerNames: payload.players.map((player) => player.name),
          startingLife: payload.startingLife,
          seed: payload.seed,
          format: payload.format,
          decks: validationDecksForMatchPayload(payload),
          sideboards: validationSideboardsForMatchPayload(payload),
          commanders: validationCommandersForMatchPayload(payload),
          openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
        });
        if (validation?.valid === false) {
          const summary = summarizeMatchValidationIssues(validation.issues);
          emitSyncFailureNotice(options.rematch ? "Trusted rematch blocked" : "Trusted match start blocked", summary.notice);
          updateMultiplayer((prev) => ({
            ...prev,
            mode: options.rematch ? "in_match" : "lobby",
            ...(options.rematch
              ? {
                  rematch: {
                    ...(prev.rematch || {}),
                    phase: "sideboarding",
                    players,
                  },
                }
              : {}),
          }));
          setStatus(summary.status, true);
          return;
        }
      }

      await applyMatchStart(payload, {
        skipGenesisVerification: true,
      });
      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
      sendMatchStartToClients(payload);
    } catch (err) {
      emitSyncFailureNotice(
        options.rematch ? "Trusted rematch start failed" : "Trusted match start failed",
        err instanceof Error ? err.message : String(err)
      );
      updateMultiplayer((prev) => ({
        ...prev,
        mode: options.rematch ? "in_match" : "lobby",
        ...(options.rematch
          ? {
              rematch: {
                ...(prev.rematch || {}),
                phase: "sideboarding",
                players,
              },
            }
          : {}),
      }));
      setStatus(`Trusted match start failed: ${toErrorMessage(err)}`, true);
    }
  }, [
    applyMatchStart,
    sendMatchStartToClients,
    setStatus,
    updateMultiplayer,
  ]);

  const startHostedMatch = useCallback(async () => {
    const session = multiplayerRef.current;
    if (!canHostedMatchStart(session)) return;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.startMatch !== "function") {
      setStatus("Game engine is not ready for multiplayer", true);
      return;
    }
    if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
      await startTrustedMatchFromPlayers(session.players);
      return;
    }

    await ensureAuditIdentity();
    let players = reindexPlayers(session.players).map((player) => ({
      ...player,
      auditPublicKey:
        player.peerId === session.localPeerId
          ? auditPublicKeyRef.current
          : player.auditPublicKey || "",
      auditEncryptionPublicKey:
        player.peerId === session.localPeerId
          ? auditEncryptionPublicKeyRef.current
          : player.auditEncryptionPublicKey || "",
    })
    );
    if (!isCurrentAuditPlayerCount(players.length)) {
      setStatus("Current P2P anticheat protocol requires 2, 3, or 4 players", true);
      return;
    }

    const sideboards = players.map((player) => sanitizeCardList(player.sideboard));
    const format = normalizeMatchFormat(session.format);
    const commanders =
      format === MATCH_FORMAT_COMMANDER
        ? players.map((player) => sanitizeCardList(player.commanders))
        : null;
    const missingZiffle = players.find((player) => !player.ziffleKey);
    if (missingZiffle) {
      setStatus(`${missingZiffle.name || "A player"} is missing a ziffle mental-poker key`, true);
      return;
    }
    const unsupportedZiffle = players.find(
      (player) => !isSupportedZiffleDeckCount(player.deckCount)
    );
    if (unsupportedZiffle) {
      setStatus(
        `${unsupportedZiffle.name || "A player"} has unsupported ziffle library size ${unsupportedZiffle.deckCount}`,
        true
      );
      return;
    }
    players = await waitForCryptoSeatBindingsFromSession(multiplayerRef);
    const unboundCryptoPlayer = players.find((player) => !playerCryptoSeatBindingReady(player));
    if (unboundCryptoPlayer) {
      setStatus(
        `${unboundCryptoPlayer.name || "A player"} is still binding deck commitments to their seat`,
        true
      );
      return;
    }

    const payload = {
      type: "match_start",
      protocolVersion: PROTOCOL_VERSION,
      securityMode: MULTIPLAYER_SECURITY_VERIFIED,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      players: players.map(toPublicPlayer),
      format,
      openDecklists: true,
      decks: players.map(() => []),
      sideboards,
      commanders: commanders || undefined,
      ziffleKeys: players.map((player) => player.ziffleKey),
      startingLife: session.startingLife,
      openingHandSize: DEFAULT_OPENING_HAND_SIZE,
      timeoutMs: matchClockConfigRef.current.initialMs,
      matchClockPolicy: matchClockPolicyPayload(matchClockConfigRef.current),
    };
    payload.seed = createMatchSeed(payload);
    payload.auditMatchId = payload.lobbyId || payload.hostPeerId;
    players = await Promise.all(
      players.map(async (player) => {
        let manifest = publicDeckManifest(player.deckAuditManifest);
        const hasLocalDeck = player.peerId === session.localPeerId;
        if (hasLocalDeck) {
          manifest = await buildLocalDeckAuditManifest({
            matchId: payload.auditMatchId,
            owner: Number(player.index || 0),
            deck: player.deck,
            sideboard: player.sideboard,
            commanders: player.commanders,
          });
        } else if (!manifest || manifest.matchId !== payload.auditMatchId) {
          throw new Error(`Missing committed deck manifest for ${player.name || "remote player"}`);
        }
        return {
          ...player,
          deckAuditManifest: publicDeckManifest(manifest),
          deckSlotOpenings: sanitizeDeckSlotOpenings(
            player.deckSlotOpenings?.length
              ? player.deckSlotOpenings
              : deckSlotOpeningsForManifest(manifest)
          ),
        };
      })
    );
    payload.players = players.map(toPublicPlayer);
    payload.deckAuditManifests = players.map((player) =>
      publicDeckManifest(player.deckAuditManifest)
    );
    try {
      await buildZiffleCeremoniesForPayload(payload, players);
    } catch (err) {
      emitSyncFailureNotice("Ziffle setup failed", toErrorMessage(err));
      updateMultiplayer((prev) => ({ ...prev, mode: "lobby" }));
      setStatus(`Ziffle setup failed: ${toErrorMessage(err)}`, true);
      return;
    }
    players = await Promise.all(players.map(async (player) => {
      if (player.peerId !== session.localPeerId) return player;
      const publicPlayer = toPublicPlayer(player);
      return {
        ...player,
        playerGenesisSignature: await signPlayerGenesis({
          matchId: payload.auditMatchId,
          player: publicPlayer,
        }),
      };
    }));
    payload.players = players.map(toPublicPlayer);
    payload.genesis = await buildSignedMatchGenesis({
      keyPair: auditKeyPairRef.current,
      match: payload,
      hostSeat: 0,
    });

    updateMultiplayer((prev) => ({ ...prev, mode: "starting", players }));

    try {
      if (typeof currentGame.validateMatchConfig === "function") {
        const validation = await currentGame.validateMatchConfig({
          playerNames: payload.players.map((player) => player.name),
          startingLife: payload.startingLife,
          seed: payload.seed,
          format: payload.format,
          decks: validationDecksForMatchPayload(payload),
          sideboards: validationSideboardsForMatchPayload(payload),
          commanders: validationCommandersForMatchPayload(payload),
          openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
          hiddenDeckManifests: payload.runtimeHiddenDeckManifests,
        });
        if (validation?.valid === false) {
          const summary = summarizeMatchValidationIssues(validation.issues);
          emitSyncFailureNotice("Match start blocked", summary.notice);
          updateMultiplayer((prev) => ({ ...prev, mode: "lobby" }));
          setStatus(summary.status, true);
          return;
        }
      }

      await applyMatchStart(payload, {
        deferLocalZiffleReveal: true,
        skipGenesisVerification: true,
      });
      payload.genesis = await buildSignedMatchGenesis({
        keyPair: auditKeyPairRef.current,
        match: payload,
        hostSeat: 0,
      });
      await verifySignedMatchGenesis(payload);
      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
      if (liveAuditTranscriptRef.current) {
        liveAuditTranscriptRef.current = {
          ...liveAuditTranscriptRef.current,
          match: cloneMultiplayerPayload(payload),
          genesis: cloneMultiplayerPayload(payload.genesis),
          initialPublicCheckpointHash: payload.initialPublicCheckpointHash || "",
        };
      }
      sendMatchStartToClients(payload);
      await revealLocalZiffleHand(payload);
    } catch (err) {
      if (/ziffle/i.test(toErrorMessage(err, ""))) {
        emitZiffleDiagnosticNotice("Match start failed", err, {
          phase: "host_start_match",
          payload: {
            auditMatchId: String(payload.auditMatchId || ""),
            lobbyId: String(payload.lobbyId || ""),
            hostPeerId: String(payload.hostPeerId || ""),
            ziffleCeremonies: (payload.ziffleCeremonies || [])
              .map(compactZiffleCeremonyForDiagnostics)
              .filter(Boolean),
          },
        });
      } else {
        emitSyncFailureNotice(
          "Match start failed",
          err instanceof Error ? err.message : String(err)
        );
      }
      updateMultiplayer((prev) => ({ ...prev, mode: "lobby" }));
      setStatus(`Match start failed: ${err}`, true);
    }
  }, [
    applyMatchStart,
    broadcastToClients,
    buildLocalDeckAuditManifest,
    ensureAuditIdentity,
    emitZiffleDiagnosticNotice,
    sendMatchStartToClients,
    setStatus,
    signPlayerGenesis,
    startTrustedMatchFromPlayers,
    updateMultiplayer,
  ]);

  const broadcastRematchState = useCallback((rematch) => {
    broadcastToClients({
      type: "rematch_state",
      protocolVersion: PROTOCOL_VERSION,
      rematch: {
        phase: rematch?.phase || "sideboarding",
        players: (rematch?.players || []).map((player) => ({
          ...toPublicPlayer(player),
          ready: Boolean(player.ready),
        })),
      },
    });
  }, [broadcastToClients]);

  const startRematchFromState = useCallback(async (rematch) => {
    const session = multiplayerRef.current;
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.startMatch !== "function") {
      setStatus("Game engine is not ready for multiplayer", true);
      return;
    }
    if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
      await startTrustedMatchFromPlayers(rematch?.players || [], { rematch: true });
      return;
    }

    await ensureAuditIdentity();
    let players = reindexPlayers(rematch?.players || []).map((player) => ({
      ...player,
      auditPublicKey:
        player.peerId === session.localPeerId
          ? auditPublicKeyRef.current
          : player.auditPublicKey || "",
      auditEncryptionPublicKey:
        player.peerId === session.localPeerId
          ? auditEncryptionPublicKeyRef.current
          : player.auditEncryptionPublicKey || "",
    })
    );
    if (!isCurrentAuditPlayerCount(players.length)) {
      setStatus("Current P2P anticheat protocol requires 2, 3, or 4 players", true);
      return;
    }
    const format = normalizeMatchFormat(session.format);
    const sideboards = players.map((player) => sanitizeCardList(player.sideboard));
    const commanders =
      format === MATCH_FORMAT_COMMANDER
        ? players.map((player) => sanitizeCardList(player.commanders))
        : null;
    const missingZiffle = players.find((player) => !player.ziffleKey);
    if (missingZiffle) {
      setStatus(`${missingZiffle.name || "A player"} is missing a ziffle mental-poker key`, true);
      return;
    }
    const unsupportedZiffle = players.find(
      (player) => !isSupportedZiffleDeckCount(player.deckCount)
    );
    if (unsupportedZiffle) {
      setStatus(
        `${unsupportedZiffle.name || "A player"} has unsupported ziffle library size ${unsupportedZiffle.deckCount}`,
        true
      );
      return;
    }
    players = await waitForCryptoSeatBindingsFromSession({ current: { players } });
    const unboundCryptoPlayer = players.find((player) => !playerCryptoSeatBindingReady(player));
    if (unboundCryptoPlayer) {
      setStatus(
        `${unboundCryptoPlayer.name || "A player"} is still binding deck commitments to their seat`,
        true
      );
      return;
    }
    const payload = {
      type: "match_start",
      protocolVersion: PROTOCOL_VERSION,
      securityMode: MULTIPLAYER_SECURITY_VERIFIED,
      lobbyId: session.lobbyId,
      hostPeerId: session.localPeerId,
      players: players.map((player) => ({
        ...toPublicPlayer(player),
        ready: false,
      })),
      format,
      openDecklists: true,
      decks: players.map(() => []),
      sideboards,
      commanders: commanders || undefined,
      ziffleKeys: players.map((player) => player.ziffleKey),
      startingLife: session.startingLife,
      openingHandSize: DEFAULT_OPENING_HAND_SIZE,
      timeoutMs: matchClockConfigRef.current.initialMs,
      matchClockPolicy: matchClockPolicyPayload(matchClockConfigRef.current),
    };
    payload.seed = createMatchSeed(payload);
    payload.auditMatchId = payload.lobbyId || payload.hostPeerId;
    players = await Promise.all(
      players.map(async (player) => {
        let manifest = publicDeckManifest(player.deckAuditManifest);
        const hasLocalDeck = player.peerId === session.localPeerId;
        if (hasLocalDeck) {
          manifest = await buildLocalDeckAuditManifest({
            matchId: payload.auditMatchId,
            owner: Number(player.index || 0),
            deck: player.deck,
            sideboard: player.sideboard,
            commanders: player.commanders,
          });
        } else if (!manifest || manifest.matchId !== payload.auditMatchId) {
          throw new Error(`Missing committed rematch deck manifest for ${player.name || "remote player"}`);
        }
        return {
          ...player,
          deckAuditManifest: publicDeckManifest(manifest),
          deckSlotOpenings: sanitizeDeckSlotOpenings(
            player.deckSlotOpenings?.length
              ? player.deckSlotOpenings
              : deckSlotOpeningsForManifest(manifest)
          ),
        };
      })
    );
    payload.players = players.map((player) => ({
      ...toPublicPlayer(player),
      ready: false,
    }));
    payload.deckAuditManifests = players.map((player) =>
      publicDeckManifest(player.deckAuditManifest)
    );
    try {
      await buildZiffleCeremoniesForPayload(payload, players);
    } catch (err) {
      emitSyncFailureNotice("Rematch ziffle setup failed", toErrorMessage(err));
      updateMultiplayer((prev) => ({
        ...prev,
        mode: "in_match",
        rematch: {
          ...(prev.rematch || {}),
          phase: "sideboarding",
          players,
        },
      }));
      setStatus(`Rematch ziffle setup failed: ${toErrorMessage(err)}`, true);
      return;
    }
    players = await Promise.all(players.map(async (player) => {
      if (player.peerId !== session.localPeerId) return player;
      const publicPlayer = toPublicPlayer(player);
      return {
        ...player,
        playerGenesisSignature: await signPlayerGenesis({
          matchId: payload.auditMatchId,
          player: publicPlayer,
        }),
      };
    }));
    payload.players = players.map((player) => ({
      ...toPublicPlayer(player),
      ready: false,
    }));
    payload.genesis = await buildSignedMatchGenesis({
      keyPair: auditKeyPairRef.current,
      match: payload,
      hostSeat: 0,
    });

    updateMultiplayer((prev) => ({
      ...prev,
      mode: "starting",
      rematch: {
        ...(prev.rematch || {}),
        phase: "starting",
        players,
      },
    }));

    try {
      if (typeof currentGame.validateMatchConfig === "function") {
        const validation = await currentGame.validateMatchConfig({
          playerNames: payload.players.map((player) => player.name),
          startingLife: payload.startingLife,
          seed: payload.seed,
          format: payload.format,
          decks: validationDecksForMatchPayload(payload),
          sideboards: validationSideboardsForMatchPayload(payload),
          commanders: validationCommandersForMatchPayload(payload),
          openingHandSize: payload.openingHandSize ?? DEFAULT_OPENING_HAND_SIZE,
          hiddenDeckManifests: payload.runtimeHiddenDeckManifests,
        });
        if (validation?.valid === false) {
          const summary = summarizeMatchValidationIssues(validation.issues);
          emitSyncFailureNotice("Rematch blocked", summary.notice);
          updateMultiplayer((prev) => ({
            ...prev,
            mode: "in_match",
            rematch: {
              ...(prev.rematch || {}),
              phase: "sideboarding",
              players,
            },
          }));
          setStatus(summary.status, true);
          return;
        }
      }

      await applyMatchStart(payload, {
        deferLocalZiffleReveal: true,
        skipGenesisVerification: true,
      });
      payload.genesis = await buildSignedMatchGenesis({
        keyPair: auditKeyPairRef.current,
        match: payload,
        hostSeat: 0,
      });
      await verifySignedMatchGenesis(payload);
      matchStartPayloadRef.current = cloneMultiplayerPayload(payload);
      if (liveAuditTranscriptRef.current) {
        liveAuditTranscriptRef.current = {
          ...liveAuditTranscriptRef.current,
          match: cloneMultiplayerPayload(payload),
          genesis: cloneMultiplayerPayload(payload.genesis),
          initialPublicCheckpointHash: payload.initialPublicCheckpointHash || "",
        };
      }
      sendMatchStartToClients(payload);
      await revealLocalZiffleHand(payload);
    } catch (err) {
      if (/ziffle/i.test(toErrorMessage(err, ""))) {
        emitZiffleDiagnosticNotice("Rematch start failed", err, {
          phase: "host_start_rematch",
          payload: {
            auditMatchId: String(payload.auditMatchId || ""),
            lobbyId: String(payload.lobbyId || ""),
            hostPeerId: String(payload.hostPeerId || ""),
            ziffleCeremonies: (payload.ziffleCeremonies || [])
              .map(compactZiffleCeremonyForDiagnostics)
              .filter(Boolean),
          },
        });
      } else {
        emitSyncFailureNotice(
          "Rematch start failed",
          err instanceof Error ? err.message : String(err)
        );
      }
      updateMultiplayer((prev) => ({
        ...prev,
        mode: "in_match",
        rematch: {
          ...(prev.rematch || {}),
          phase: "sideboarding",
          players,
        },
      }));
      setStatus(`Rematch start failed: ${err}`, true);
    }
  }, [
    applyMatchStart,
    broadcastToClients,
    buildLocalDeckAuditManifest,
    ensureAuditIdentity,
    emitZiffleDiagnosticNotice,
    sendMatchStartToClients,
    setStatus,
    signPlayerGenesis,
    startTrustedMatchFromPlayers,
    updateMultiplayer,
  ]);

  const startRematchSideboarding = useCallback((source = "local") => {
    const session = multiplayerRef.current;
    const payload = matchStartPayloadRef.current;
    if (!session.matchStarted || !payload) {
      setStatus("No completed multiplayer match is available to replay", true);
      return;
    }

    if (session.role !== "host") {
      const conn = hostConnectionRef.current;
      if (!conn || conn.open === false) {
        setStatus("Host connection is not available", true);
        return;
      }
      safeSend(conn, {
        type: "rematch_request",
        protocolVersion: PROTOCOL_VERSION,
      });
      setStatus("Waiting for host to open sideboarding");
      return;
    }

    const rematch = buildRematchStateFromPayload(payload, session.localPeerId);
    updateMultiplayer((prev) => ({
      ...prev,
      mode: "in_match",
      rematch,
    }));
    broadcastToClients({
      type: "rematch_start",
      protocolVersion: PROTOCOL_VERSION,
      match: payload,
    });
    setStatus(source === "remote" ? "Sideboarding opened for rematch" : "Sideboard for the next game");
  }, [broadcastToClients, setStatus, updateMultiplayer]);

  const updateRematchDecks = useCallback(({ deck, sideboard }) => {
    updateMultiplayer((prev) => {
      if (!prev.rematch || prev.rematch.phase !== "sideboarding") return prev;
      return {
        ...prev,
        rematch: {
          ...prev.rematch,
          localDeck: sanitizeCardList(deck ?? prev.rematch.localDeck),
          localSideboard: sanitizeCardList(sideboard ?? prev.rematch.localSideboard),
          localReady: false,
        },
      };
    });
  }, [updateMultiplayer]);

  const readyForRematch = useCallback(async () => {
    const session = multiplayerRef.current;
    const rematch = session.rematch;
    if (!rematch || rematch.phase !== "sideboarding") {
      setStatus("Sideboarding is not active", true);
      return;
    }
    const localIndex = resolveLocalPlayerIndex(session);
    if (localIndex == null) {
      setStatus("Local player seat is not assigned", true);
      return;
    }

    const localDeck = sanitizeCardList(rematch.localDeck);
    const localSideboard = sanitizeCardList(rematch.localSideboard);
    const localPlayer = reindexPlayers(rematch.players || []).find(
      (player) => Number(player.index) === Number(localIndex)
    );
    const localCommanders = sanitizeCardList(localPlayer?.commanders);
    if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
      if (session.role !== "host") {
        const conn = hostConnectionRef.current;
        if (!conn || conn.open === false) {
          setStatus("Host connection is not available", true);
          return;
        }
        updateMultiplayer((prev) => ({
          ...prev,
          rematch: prev.rematch
            ? { ...prev.rematch, localReady: true }
            : prev.rematch,
        }));
        safeSend(conn, {
          type: "rematch_ready",
          protocolVersion: PROTOCOL_VERSION,
          securityMode: MULTIPLAYER_SECURITY_TRUSTED,
          deck: localDeck,
          sideboard: localSideboard,
          commanders: localCommanders,
          deckAuditManifest: null,
          deckSlotOpenings: [],
          ziffleKey: null,
          playerGenesisSignature: null,
          deckCount: localDeck.length,
          sideboardCount: localSideboard.length,
          commanderCount: localCommanders.length,
        });
        setStatus("Ready for trusted rematch");
        return;
      }

      const nextSession = updateMultiplayer((prev) => {
        if (!prev.rematch) return prev;
        const players = reindexPlayers(prev.rematch.players || []).map((player) => (
          Number(player.index) === localIndex
            ? {
                ...player,
                deck: localDeck,
                sideboard: localSideboard,
                commanders: localCommanders,
                deckAuditManifest: null,
                deckSlotOpenings: [],
                ziffleKey: null,
                playerGenesisSignature: null,
                deckCount: localDeck.length,
                sideboardCount: localSideboard.length,
                commanderCount: localCommanders.length,
                ready: true,
              }
            : player
        ));
        return {
          ...prev,
          rematch: {
            ...prev.rematch,
            players,
            localDeck,
            localSideboard,
            localReady: true,
          },
        };
      });

      broadcastRematchState(nextSession.rematch);
      if (rematchPlayersReady(nextSession.rematch?.players)) {
        await startRematchFromState(nextSession.rematch);
      } else {
        setStatus("Ready for trusted rematch; waiting for other players");
      }
      return;
    }
    const deckAuditManifest = await buildLocalDeckAuditManifest({
      matchId: session.lobbyId || session.hostPeerId || "pending",
      owner: localIndex,
      deck: localDeck,
      sideboard: localSideboard,
      commanders: localCommanders,
    });
    const deckSlotOpenings = deckSlotOpeningsForManifest(deckAuditManifest);
    const {
      publicKey: auditPublicKey,
      encryptionPublicKey: auditEncryptionPublicKey,
    } = await ensureAuditIdentity();
    const localGenesisPlayer = {
      peerId: localPlayer?.peerId || session.localPeerId,
      name: localPlayer?.name || session.localName,
      index: localIndex,
      auditPublicKey,
      auditEncryptionPublicKey,
      deckAuditManifest: publicDeckManifest(deckAuditManifest),
      ziffleKey: localPlayer?.ziffleKey || null,
      ...openDecklistPlayerFields({
        deck: localDeck,
        sideboard: localSideboard,
        commanders: localCommanders,
        deckSlotOpenings,
      }),
      deckCount: localDeck.length,
      sideboardCount: localSideboard.length,
      commanderCount: localCommanders.length,
    };
    const playerGenesisSignature = await signPlayerGenesis({
      matchId: session.lobbyId || session.hostPeerId || "pending",
      player: localGenesisPlayer,
    });

    if (session.role !== "host") {
      const conn = hostConnectionRef.current;
      if (!conn || conn.open === false) {
        setStatus("Host connection is not available", true);
        return;
      }
      updateMultiplayer((prev) => ({
        ...prev,
        rematch: prev.rematch
          ? { ...prev.rematch, localReady: true }
          : prev.rematch,
      }));
      safeSend(conn, {
        type: "rematch_ready",
        protocolVersion: PROTOCOL_VERSION,
        auditPublicKey,
        auditEncryptionPublicKey,
        playerGenesisSignature,
        deckAuditManifest: publicDeckManifest(deckAuditManifest),
        deck: localDeck,
        sideboard: localSideboard,
        commanders: localCommanders,
        deckSlotOpenings,
        deckCount: localDeck.length,
        sideboardCount: localSideboard.length,
        commanderCount: localCommanders.length,
      });
      setStatus("Ready for rematch");
      return;
    }

    const nextSession = updateMultiplayer((prev) => {
      if (!prev.rematch) return prev;
      const players = reindexPlayers(prev.rematch.players || []).map((player) => (
        Number(player.index) === localIndex
	          ? {
	              ...player,
              auditPublicKey,
              auditEncryptionPublicKey,
              playerGenesisSignature,
	              deck: localDeck,
	              sideboard: localSideboard,
              commanders: localCommanders,
	              deckAuditManifest: publicDeckManifest(deckAuditManifest),
              deckSlotOpenings,
	              deckCount: localDeck.length,
	              sideboardCount: localSideboard.length,
              commanderCount: localCommanders.length,
	              ready: true,
	            }
          : player
      ));
      return {
        ...prev,
        rematch: {
          ...prev.rematch,
          players,
          localDeck,
          localSideboard,
          localReady: true,
        },
      };
    });

    broadcastRematchState(nextSession.rematch);
    if (rematchPlayersReady(nextSession.rematch?.players)) {
      await startRematchFromState(nextSession.rematch);
    } else {
      setStatus("Ready for rematch; waiting for other players");
    }
	  }, [
	    broadcastRematchState,
	    buildLocalDeckAuditManifest,
	    ensureAuditIdentity,
	    setStatus,
	    signPlayerGenesis,
	    startRematchFromState,
	    updateMultiplayer,
	  ]);

	  async function publishLocalDeckUpdateForAssignedSeat(sessionSnapshot = multiplayerRef.current) {
	    const session = sessionSnapshot || multiplayerRef.current;
	    if (session.role !== "client" || session.matchStarted || session.mode === "starting") return;
	    const localPlayerIndex = resolveLocalPlayerIndex(session);
	    if (localPlayerIndex == null) return;
	    const deckSubmission = parseDeckSubmission(
	      session.format,
	      session.localDeckText,
	      session.localCommanderText
	    );
	    if (isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) {
	      updateMultiplayer((prev) => ({
	        ...prev,
	        localDeckCount: deckSubmission.deckCount,
	        localCommanderCount: deckSubmission.commanderCount,
	        players: reindexPlayers(
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
	        ),
	      }));
	      const conn = hostConnectionRef.current;
	      if (!conn || conn.open === false) return;
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
	      return;
	    }
	    const matchId = session.lobbyId || session.hostPeerId || "pending";
    const expectedDecklistHash = await decklistHashForCards({
      matchId,
      owner: localPlayerIndex,
      deck: deckSubmission.deck,
      sideboard: deckSubmission.sideboard,
      commanders: deckSubmission.commanders,
    });
    const localEntry = (session.players || []).find(
      (player) => player.peerId === session.localPeerId
    );
    const currentManifest = publicDeckManifest(localEntry?.deckAuditManifest);
    const currentPrivateManifest = privateDeckManifestForOwner(localPlayerIndex, matchId);
    const currentZifflePlayer = normalizePlayerIndex(localEntry?.ziffleKey?.player);
    const currentSigner = normalizePlayerIndex(localEntry?.playerGenesisSignature?.signer);
    if (
      currentManifest?.owner === localPlayerIndex
      && currentManifest?.matchId === matchId
      && currentPrivateManifest?.decklistHash === expectedDecklistHash
      && currentManifest?.decklistCommitment === currentPrivateManifest?.decklistCommitment
      && currentManifest?.commitmentRoot === currentPrivateManifest?.commitmentRoot
      && currentZifflePlayer === localPlayerIndex
      && currentSigner === localPlayerIndex
    ) {
      return;
    }

    const {
      publicKey: auditPublicKey,
      encryptionPublicKey: auditEncryptionPublicKey,
    } = await ensureAuditIdentity();
    const deckAuditManifest = await buildLocalDeckAuditManifest({
      matchId,
      owner: localPlayerIndex,
      deck: deckSubmission.deck,
      sideboard: deckSubmission.sideboard,
      commanders: deckSubmission.commanders,
    });
    const deckSlotOpenings = deckSlotOpeningsForManifest(deckAuditManifest);
    const ziffleKeyPair = await ensureZiffleIdentity({
      context: matchId,
      deckCount: deckSubmission.deckCount || 60,
    });
    const localZiffleKey = publicZiffleKey(ziffleKeyPair, localPlayerIndex);
    const playerGenesisSignature = await signPlayerGenesis({
      matchId,
      player: {
        peerId: session.localPeerId,
        name: session.localName,
	        index: localPlayerIndex,
	        auditPublicKey,
	        auditEncryptionPublicKey,
	        deckAuditManifest: publicDeckManifest(deckAuditManifest),
        ...openDecklistPlayerFields({
          deck: deckSubmission.deck,
          sideboard: deckSubmission.sideboard,
          commanders: deckSubmission.commanders,
          deckSlotOpenings,
        }),
        ziffleKey: localZiffleKey,
        deckCount: deckSubmission.deckCount,
        sideboardCount: deckSubmission.sideboard.length,
        commanderCount: deckSubmission.commanderCount,
      },
    });

    updateMultiplayer((prev) => ({
      ...prev,
      localDeckCount: deckSubmission.deckCount,
      localCommanderCount: deckSubmission.commanderCount,
      players: reindexPlayers(
        prev.players.map((player) =>
          player.peerId === prev.localPeerId
            ? {
	                ...player,
	                auditPublicKey,
	                auditEncryptionPublicKey,
                playerGenesisSignature,
                deckAuditManifest: publicDeckManifest(deckAuditManifest),
                deck: deckSubmission.deck,
                sideboard: deckSubmission.sideboard,
                commanders: deckSubmission.commanders,
                deckSlotOpenings,
                ziffleKey: localZiffleKey,
                deckCount: deckSubmission.deckCount,
                sideboardCount: deckSubmission.sideboard.length,
                commanderCount: deckSubmission.commanderCount,
                ready: deckSubmission.ready,
              }
            : player
        )
      ),
    }));

    const conn = hostConnectionRef.current;
    if (!conn || conn.open === false) return;
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

  const handleHostMessage = useCallback(
    async (message) => {
      if (!message || typeof message !== "object") return;
      if (message.protocolVersion !== PROTOCOL_VERSION) {
        setStatus("Lobby protocol version mismatch", true);
        return;
      }

      switch (message.type) {
        case "lobby_state": {
          const nextSession = updateMultiplayer((prev) => {
            const localEntry = (message.players || []).find(
              (player) => player.peerId === prev.localPeerId
            );
            if (localEntry) {
              writeStoredPlayerIndex(
                message.lobbyId || message.hostPeerId || prev.lobbyId,
                localEntry.index
              );
            }
            const nextFormat = normalizeMatchFormat(message.format || prev.format);
            const nextSecurityMode = normalizeMultiplayerSecurityMode(
              message.securityMode,
              prev.securityMode
            );
            const localDeckSubmission = parseDeckSubmission(
              nextFormat,
              prev.localDeckText,
              prev.localCommanderText
            );
            return {
              ...prev,
              mode: message.matchStarted ? "starting" : "lobby",
              lobbyId: message.lobbyId || prev.lobbyId,
              hostPeerId: message.hostPeerId || prev.hostPeerId,
              desiredPlayers: Number(message.desiredPlayers || prev.desiredPlayers || 0),
              startingLife: Number(message.startingLife || prev.startingLife || 20),
              format: nextFormat,
              securityMode: nextSecurityMode,
              localDeckCount: localDeckSubmission.deckCount,
              localCommanderCount: localDeckSubmission.commanderCount,
              players: message.players || [],
              localPlayerIndex: localEntry ? localEntry.index : prev.localPlayerIndex,
              matchStarted: Boolean(message.matchStarted),
              submittingAction: false,
            };
          });
          rememberDefaultLobbyDeck(
            nextSession.localDeckText,
            nextSession.localCommanderText
          );
          ensureDirectPeerConnections(message.players || []);
          void publishLocalDeckUpdateForAssignedSeat(nextSession).catch((err) => {
            emitSyncFailureNotice(
              "Deck commitment update failed",
              err instanceof Error ? err.message : String(err)
            );
            setStatus(`Deck commitment update failed: ${toErrorMessage(err)}`, true);
          });
          return;
        }
        case "reject":
          leaveLobby(message.reason || "Lobby join rejected", {
            clearStoredPlayer: message.reason !== "Player slot already filled",
            isError: true,
          });
          return;
        case "reconnect_challenge": {
          const conn = hostConnectionRef.current;
          if (!conn || conn.open === false) {
            setStatus("Reconnect challenge arrived without an open host connection", true);
            return;
          }
          const {
            publicKey: auditPublicKey,
            encryptionPublicKey: auditEncryptionPublicKey,
          } = await ensureAuditIdentity();
          const proof = await signReconnectProofForChallenge(message);
          safeSend(conn, {
            type: "join_request",
            protocolVersion: PROTOCOL_VERSION,
            name: multiplayerRef.current.localName || "Player",
            requestedPlayerIndex: Number(message.playerIndex),
            auditPublicKey,
            auditEncryptionPublicKey,
            reconnectChallengeId: message.requestId,
            reconnectProof: proof,
          });
          setStatus("Proving reconnect identity to host");
          return;
        }
        case "action_error":
          updateMultiplayer((prev) => ({ ...prev, submittingAction: false }));
          if (message.rollbackSynced) {
            const reason = message.reason || "Host rejected the local action";
            const rollbackSequence = Number(message.rollbackSequence || 0);
            emitSyncFailureNotice("Action reverted", reason);
            if (
              !requestResync(
                rollbackSequence > 0
                  ? `Host reverted invalid action at ${rollbackSequence}. Resyncing with host...`
                  : "Host reverted invalid action. Resyncing with host..."
              )
            ) {
              setStatus(reason, true);
            }
            return;
          }
          reportSyncFailure(
            message.reason || "Action rejected by multiplayer host",
            "Host rejected the local action. Resyncing with host...",
            message.reason || "Action rejected"
          );
          return;
        case "match_start":
          awaitingStateResyncRef.current = false;
          await applyMatchStart(message);
          return;
        case "ziffle_shuffle_step_request":
          await answerZiffleShuffleStepRequest(hostConnectionRef.current, message);
          return;
        case "ziffle_shuffle_step_response":
          resolveZiffleShuffleStep(message);
          return;
        case "rng_commit_request":
          await answerRngCommitRequest(hostConnectionRef.current, message);
          return;
        case "rng_reveal_request":
          await answerRngRevealRequest(hostConnectionRef.current, message);
          return;
        case "rng_commit_response":
          resolveRngCommit(message);
          return;
        case "rng_reveal_response":
          resolveRngReveal(message);
          return;
        case "timeout_vote_request":
          await answerTimeoutVoteRequest(hostConnectionRef.current, message);
          return;
        case "timeout_vote_response":
          resolveTimeoutVote(message);
          return;
        case "disconnect_forfeit_vote_request":
          await answerDisconnectForfeitVoteRequest(hostConnectionRef.current, message);
          return;
        case "disconnect_forfeit_vote_response":
          resolveTimeoutVote(message);
          return;
        case "protocol_timeout_vote_request":
          await answerProtocolResponseTimeoutVoteRequest(hostConnectionRef.current, message);
          return;
        case "protocol_timeout_vote_response":
          resolveTimeoutVote(message);
          return;
        case "action_quorum_vote_request":
          await answerActionQuorumVoteRequest(hostConnectionRef.current, message);
          return;
        case "action_quorum_vote_response":
          resolveActionQuorumVote(message);
          return;
        case "crypto_material_request":
          await answerCryptoMaterialRequest(hostConnectionRef.current, message);
          return;
        case "crypto_material_response":
          resolveCryptoMaterial(message);
          return;
        case "action_intent_progress":
          await handleActionIntentProgressMessage(message);
          return;
        case "action_intent_cancel":
          await handleActionIntentCancelMessage(message);
          return;
        case "ziffle_reveal_token_request":
          await answerZiffleRevealTokenRequest(hostConnectionRef.current, message);
          return;
        case "ziffle_reveal_token_response":
          resolveZiffleRevealToken(message);
          return;
        case "resync_ack":
          awaitingStateResyncRef.current = false;
          setStatus(`Already synced with host at action ${Number(message.lastSequence ?? 0)}`);
          return;
        case "rematch_start": {
          const rematch = buildRematchStateFromPayload(
            message.match,
            multiplayerRef.current.localPeerId
          );
          updateMultiplayer((prev) => ({
            ...prev,
            mode: "in_match",
            rematch,
          }));
          setStatus("Sideboard for the next game");
          return;
        }
        case "rematch_state": {
          const publicPlayersByPeer = new Map(
            (message.rematch?.players || []).map((player) => [player.peerId, player])
          );
          updateMultiplayer((prev) => {
            if (!prev.rematch) return prev;
            const players = (prev.rematch.players || []).map((player) => {
              const update = publicPlayersByPeer.get(player.peerId);
              if (!update) return player;
              return {
                ...player,
                ...update,
                deck: sanitizeCardList(update.deck),
                sideboard: sanitizeCardList(update.sideboard),
                commanders: sanitizeCardList(update.commanders),
                deckSlotOpenings: sanitizeDeckSlotOpenings(update.deckSlotOpenings),
                ready: Boolean(update.ready),
              };
            });
            const localPlayer = players.find((player) => player.peerId === prev.localPeerId);
            return {
              ...prev,
              rematch: {
                ...prev.rematch,
                phase: message.rematch?.phase || prev.rematch.phase,
                players,
                localReady: Boolean(localPlayer?.ready),
              },
            };
          });
          return;
        }
        case "state_resync":
          await applyStateResync(message);
          return;
        case "player_presence":
          updateMultiplayer((prev) => {
            const peerId = String(message.peerId || "").trim();
            const playerIndex = normalizePlayerIndex(message.playerIndex);
            const players = (prev.players || []).some((player) => player.peerId === peerId)
              ? prev.players
              : (prev.players || []).map((player) =>
                  playerIndex != null && Number(player.index) === playerIndex
                    ? { ...player, peerId }
                    : player
                );
            return {
              ...prev,
              players: markPlayerConnectionState(
                players,
                peerId,
                message.connected !== false,
                message.disconnectedAtMs == null ? Date.now() : Number(message.disconnectedAtMs)
              ),
            };
          });
          return;
        case "apply_action": {
          await applySequencedActionMessage(message);
          return;
        }
        default:
          return;
      }
    },
    [
      answerCryptoMaterialRequest,
      answerActionQuorumVoteRequest,
      answerTimeoutVoteRequest,
      answerDisconnectForfeitVoteRequest,
      answerProtocolResponseTimeoutVoteRequest,
      applyMatchStart,
      applyStateResync,
      applySyncedCommand,
      ensureDirectPeerConnections,
      leaveLobby,
      reportSyncFailure,
      requestResync,
      revealAuditOpenings,
      publishLocalDeckUpdateForAssignedSeat,
      resolveCryptoMaterial,
      resolveActionQuorumVote,
      resolveRngCommit,
      resolveRngReveal,
      resolveTimeoutVote,
      resolveZiffleRevealToken,
      resolveZiffleShuffleStep,
      setStatus,
      updateMultiplayer,
      verifySequencedActionAudit,
    ]
  );

  const handleClientDisconnect = useCallback(
    (peerId) => {
      clearConnectionHeartbeat(connectionHeartbeatKey("client", peerId));
      for (const [key, conn] of clientConnectionsRef.current.entries()) {
        if (String(key || "") === String(peerId || "") || String(conn?.peer || "") === String(peerId || "")) {
          clientConnectionsRef.current.delete(key);
        }
      }
      finishPeerResync(peerId);
      const departed = multiplayerRef.current.players.find(
        (player) => player.peerId === peerId
      );
      const disconnectedAtMs = Date.now();
      rememberLocalDisconnectObservation(peerId, {
        playerIndex: departed?.index,
        disconnectedAtMs,
        source: "host_client_connection",
      });
      updateMultiplayer((prev) => ({
        ...prev,
        players: markPlayerConnectionState(prev.players, peerId, false, disconnectedAtMs),
      }));
      if (departed) {
        setStatus(`${departed.name} disconnected; waiting 60s for timeout policy`);
      }
      if (multiplayerRef.current.matchStarted) {
        broadcastMatchPresence(peerId, false, {
          disconnectedAtMs,
          autoForfeitAtMs: disconnectedAtMs + DISCONNECT_AUTO_FORFEIT_MS,
        });
      } else {
        broadcastLobbyState();
      }
    },
    [
      broadcastLobbyState,
      broadcastMatchPresence,
      clearConnectionHeartbeat,
      finishPeerResync,
      setStatus,
      updateMultiplayer,
    ]
  );

  const handlePeerMessage = useCallback(async (conn, message) => {
    if (!message || typeof message !== "object") return;
    if (message.protocolVersion !== PROTOCOL_VERSION) {
      conn.close();
      return;
    }
    switch (message.type) {
      case "peer_ready":
        return;
      case "apply_action":
        await applySequencedActionMessage(message);
        return;
      case "ziffle_shuffle_step_request":
        await answerZiffleShuffleStepRequest(conn, message);
        return;
      case "ziffle_shuffle_step_response":
        resolveZiffleShuffleStep(message);
        return;
      case "ziffle_reveal_token_request":
        await answerZiffleRevealTokenRequest(conn, message);
        return;
      case "ziffle_reveal_token_response":
        resolveZiffleRevealToken(message);
        return;
      case "rng_commit_request":
        await answerRngCommitRequest(conn, message);
        return;
      case "rng_reveal_request":
        await answerRngRevealRequest(conn, message);
        return;
      case "rng_commit_response":
        resolveRngCommit(message);
        return;
      case "rng_reveal_response":
        resolveRngReveal(message);
        return;
      case "timeout_vote_request":
        await answerTimeoutVoteRequest(conn, message);
        return;
      case "timeout_vote_response":
        resolveTimeoutVote(message);
        return;
      case "disconnect_forfeit_vote_request":
        await answerDisconnectForfeitVoteRequest(conn, message);
        return;
      case "disconnect_forfeit_vote_response":
        resolveTimeoutVote(message);
        return;
      case "protocol_timeout_vote_request":
        await answerProtocolResponseTimeoutVoteRequest(conn, message);
        return;
      case "protocol_timeout_vote_response":
        resolveTimeoutVote(message);
        return;
      case "action_quorum_vote_request":
        await answerActionQuorumVoteRequest(conn, message);
        return;
      case "action_quorum_vote_response":
        resolveActionQuorumVote(message);
        return;
      case "crypto_material_request":
        await answerCryptoMaterialRequest(conn, message);
        return;
      case "crypto_material_response":
        resolveCryptoMaterial(message);
        return;
      case "action_intent_progress":
        await handleActionIntentProgressMessage(message);
        return;
      case "action_intent_cancel":
        await handleActionIntentCancelMessage(message);
        return;
      default:
        return;
    }
  }, [
    answerCryptoMaterialRequest,
    answerActionQuorumVoteRequest,
    answerTimeoutVoteRequest,
    answerDisconnectForfeitVoteRequest,
    answerProtocolResponseTimeoutVoteRequest,
    resolveActionQuorumVote,
    resolveCryptoMaterial,
    resolveRngCommit,
    resolveRngReveal,
    resolveTimeoutVote,
    resolveZiffleRevealToken,
    resolveZiffleShuffleStep,
  ]);

  const handlePeerDisconnect = useCallback(
    (peerId) => {
      clearConnectionHeartbeat(connectionHeartbeatKey("peer", peerId));
      if (peerConnectionsRef.current.get(peerId)?.peer === peerId) {
        peerConnectionsRef.current.delete(peerId);
      }
      if (multiplayerRef.current.matchStarted) {
        const departed = multiplayerRef.current.players.find(
          (player) => player.peerId === peerId
        );
        rememberLocalDisconnectObservation(peerId, {
          playerIndex: departed?.index,
          disconnectedAtMs: Date.now(),
          source: "direct_peer_connection",
        });
        if (departed) {
          setStatus(`${departed.name} disconnected from direct peer sync`, true);
        }
      }
    },
    [clearConnectionHeartbeat, setStatus, updateMultiplayer]
  );

  const configurePeerConnection = useCallback(
    (conn) => {
      const heartbeatKey = connectionHeartbeatKey("peer", conn.peer);
      const beginHeartbeat = () => {
        peerConnectionsRef.current.set(conn.peer, conn);
        const connectedPlayer = multiplayerRef.current.players.find(
          (player) => player.peerId === conn.peer
        );
        clearLocalDisconnectObservation(conn.peer, connectedPlayer?.index);
        if (multiplayerRef.current.matchStarted) {
          updateMultiplayer((prev) => ({
            ...prev,
            players: markPlayerConnectionState(prev.players, conn.peer, true),
          }));
        }
        startConnectionHeartbeat(heartbeatKey, conn, () => {
          if (peerConnectionsRef.current.get(conn.peer) !== conn) return;
          handlePeerDisconnect(conn.peer);
        });
        safeSend(conn, {
          type: "peer_ready",
          protocolVersion: PROTOCOL_VERSION,
          peerId: multiplayerRef.current.localPeerId || "",
        });
      };
      if (conn.open) {
        beginHeartbeat();
      } else {
        conn.on("open", beginHeartbeat);
      }
      conn.on("data", (message) => {
        markConnectionAlive(heartbeatKey);
        if (handleConnectionHeartbeatMessage(conn, message)) return;
        if (message?.type === "apply_action") {
          void enqueueAsync(peerMessageQueueRef, () =>
            handlePeerMessage(conn, message)
          ).catch((err) => {
            setStatus(`Peer message failed: ${toErrorMessage(err)}`, true);
          });
          return;
        }
        if (
          message?.type === "ziffle_shuffle_step_request"
          || message?.type === "ziffle_shuffle_step_response"
          || message?.type === "ziffle_reveal_token_request"
          || message?.type === "ziffle_reveal_token_response"
          || message?.type === "rng_commit_request"
          || message?.type === "rng_commit_response"
          || message?.type === "rng_reveal_request"
          || message?.type === "rng_reveal_response"
          || message?.type === "timeout_vote_request"
          || message?.type === "timeout_vote_response"
          || message?.type === "disconnect_forfeit_vote_request"
          || message?.type === "disconnect_forfeit_vote_response"
          || message?.type === "protocol_timeout_vote_request"
          || message?.type === "protocol_timeout_vote_response"
          || message?.type === "action_quorum_vote_request"
          || message?.type === "action_quorum_vote_response"
          || message?.type === "crypto_material_request"
          || message?.type === "crypto_material_response"
          || message?.type === "action_intent_progress"
          || message?.type === "action_intent_cancel"
        ) {
          void handlePeerMessage(conn, message).catch((err) => {
            if (shouldSuppressProtocolMessageError(err, message)) return;
            setStatus(`Peer message failed: ${toErrorMessage(err)}`, true);
          });
          return;
        }
        void enqueueAsync(peerMessageQueueRef, () =>
          handlePeerMessage(conn, message)
        ).catch((err) => {
          setStatus(`Peer message failed: ${toErrorMessage(err)}`, true);
        });
      });
      conn.on("close", () => {
        clearConnectionHeartbeat(heartbeatKey);
        if (peerConnectionsRef.current.get(conn.peer) === conn) {
          handlePeerDisconnect(conn.peer);
        }
      });
      conn.on("error", () => {
        clearConnectionHeartbeat(heartbeatKey);
        if (peerConnectionsRef.current.get(conn.peer) === conn) {
          handlePeerDisconnect(conn.peer);
        }
      });
    },
    [
      clearConnectionHeartbeat,
      handleConnectionHeartbeatMessage,
      handlePeerDisconnect,
      handlePeerMessage,
      markConnectionAlive,
      setStatus,
      startConnectionHeartbeat,
      updateMultiplayer,
    ]
  );

  const connectDirectPeer = useCallback(
    (peerId) => {
      const target = String(peerId || "").trim();
      const peer = peerRef.current;
      const session = multiplayerRef.current;
      if (!target || target === session.localPeerId || !peer || peer.destroyed) return null;
      const existing = peerConnectionsRef.current.get(target);
      if (existing?.open) return existing;
      if (existing) {
        try {
          existing.close();
        } catch (err) {
          void err;
        }
      }
	      const conn = peer.connect(target, {
	        reliable: true,
	        serialization: "binary",
	        metadata: {
          channel: "peer-direct",
          lobbyId: session.lobbyId || session.hostPeerId || "",
          from: session.localPeerId || "",
        },
      });
      peerConnectionsRef.current.set(target, conn);
      configurePeerConnection(conn);
      return conn;
    },
    [configurePeerConnection]
  );

  ensureDirectPeerConnectionsRef.current = (players = multiplayerRef.current.players) => {
    const session = multiplayerRef.current;
    const localPeerId = String(session.localPeerId || "").trim();
	    const hostPeerId = String(session.hostPeerId || "").trim();
	    if (!localPeerId || !peerRef.current || peerRef.current.destroyed) return;
	    for (const player of players || []) {
	      const peerId = routePeerIdForPlayer(player);
	      if (!peerId || peerId === localPeerId || peerId === hostPeerId) continue;
	      // Deterministic initiator prevents duplicate client-client channels.
	      if (localPeerId < peerId) {
	        connectDirectPeer(peerId);
      }
    }
  };

  const sendDirectPeerMessage = useCallback(
    (peerId, payload) => {
      const target = String(peerId || "").trim();
      if (!target) return false;
      const session = multiplayerRef.current;
      if (target === session.localPeerId) return true;
      const existingRoute = openZiffleRoute(target);
      if (existingRoute) {
        safeSend(existingRoute, payload);
        return true;
      }
      if (session.role === "host") {
        let conn = null;
        for (const candidate of ziffleRoutePeerCandidates(target)) {
          if (!candidate || candidate === session.localPeerId) continue;
          conn = connectDirectPeer(candidate);
          if (conn && conn.open !== false) break;
        }
        if (!conn || conn.open === false) return false;
        safeSend(conn, payload);
        return true;
      }
      if (target === session.hostPeerId) {
        let conn = hostConnectionRef.current;
        if (!conn || conn.open === false) {
          conn = peerConnectionsRef.current.get(target);
        }
        if (!conn || conn.open === false) {
          conn = connectDirectPeer(target);
        }
        if (!conn || conn.open === false) return false;
        safeSend(conn, payload);
        return true;
      }
      let conn = peerConnectionsRef.current.get(target);
      if (!conn || conn.open === false) {
        conn = connectDirectPeer(target);
      }
      if (!conn || conn.open === false) return false;
      safeSend(conn, payload);
      return true;
    },
    [connectDirectPeer]
  );

  function reconnectChallengeMapKey(peerId, requestId) {
    return [String(peerId || ""), String(requestId || "")].join(":");
  }

  function clearReconnectChallenge(peerId, requestId) {
    const key = reconnectChallengeMapKey(peerId, requestId);
    const challenge = reconnectChallengesRef.current.get(key);
    if (challenge?.timeoutId) {
      window.clearTimeout(challenge.timeoutId);
    }
    reconnectChallengesRef.current.delete(key);
  }

  function issueReconnectChallenge(conn, playerIndex) {
    const session = multiplayerRef.current;
    const requestId = makeZiffleRequestId("reconnect-proof");
    const challenge = {
      type: "reconnect_challenge",
      protocolVersion: PROTOCOL_VERSION,
      requestId,
      matchId: currentAuditMatchId(),
      playerIndex: Number(playerIndex),
      peerId: String(conn?.peer || ""),
      hostPeerId: String(session.localPeerId || session.hostPeerId || ""),
      transcriptHash: String(auditStateHashRef.current || INITIAL_AUDIT_STATE_HASH),
      nonce: randomAuditHex(32),
    };
    const key = reconnectChallengeMapKey(conn?.peer, requestId);
    const timeoutId = window.setTimeout(() => {
      reconnectChallengesRef.current.delete(key);
    }, PEER_CONNECT_TIMEOUT_MS);
    reconnectChallengesRef.current.set(key, {
      ...challenge,
      timeoutId,
    });
    safeSend(conn, challenge);
  }

  const handleClientMessage = useCallback(
    async (conn, message) => {
      if (!message || typeof message !== "object") return;
      if (message.protocolVersion !== PROTOCOL_VERSION) {
        safeSend(conn, {
          type: "reject",
          protocolVersion: PROTOCOL_VERSION,
          reason: "Protocol version mismatch",
        });
        conn.close();
        return;
      }

      switch (message.type) {
      case "ziffle_shuffle_step_request":
        await answerZiffleShuffleStepRequest(conn, message);
        return;
      case "ziffle_shuffle_step_response":
        resolveZiffleShuffleStep(message);
        return;
      case "rng_commit_response":
        resolveRngCommit(message);
        return;
      case "rng_reveal_response":
        resolveRngReveal(message);
        return;
      case "timeout_vote_response":
        resolveTimeoutVote(message);
        return;
      case "disconnect_forfeit_vote_response":
        resolveTimeoutVote(message);
        return;
      case "action_quorum_vote_response":
        resolveActionQuorumVote(message);
        return;
      case "crypto_material_response":
        resolveCryptoMaterial(message);
        return;
      case "apply_action":
        await applySequencedActionMessage(message);
        return;
      case "rng_commit_request":
        await answerRngCommitRequest(conn, message);
        return;
      case "rng_reveal_request":
        await answerRngRevealRequest(conn, message);
        return;
      case "timeout_vote_request":
        await answerTimeoutVoteRequest(conn, message);
        return;
      case "disconnect_forfeit_vote_request":
        await answerDisconnectForfeitVoteRequest(conn, message);
        return;
      case "protocol_timeout_vote_request":
        await answerProtocolResponseTimeoutVoteRequest(conn, message);
        return;
      case "protocol_timeout_vote_response":
        resolveTimeoutVote(message);
        return;
      case "action_quorum_vote_request":
        await answerActionQuorumVoteRequest(conn, message);
        return;
      case "crypto_material_request":
        await answerCryptoMaterialRequest(conn, message);
        return;
      case "action_intent_progress":
        await handleActionIntentProgressMessage(message);
        return;
      case "action_intent_cancel":
        await handleActionIntentCancelMessage(message);
        return;
      case "ziffle_reveal_token_request":
        await answerZiffleRevealTokenRequest(conn, message);
          return;
        case "ziffle_reveal_token_response":
          resolveZiffleRevealToken(message);
          return;
        case "join_request": {
          const session = multiplayerRef.current;
          const requestedPlayerIndex = normalizePlayerIndex(message.requestedPlayerIndex);
          const indexedPlayers = reindexPlayers(session.players);
          let targetPlayerIndex = null;

          if (requestedPlayerIndex != null) {
            const requestedSlot = indexedPlayers.find(
              (player) => Number(player.index) === requestedPlayerIndex
            );
            if (!requestedSlot || requestedPlayerIndex >= session.desiredPlayers) {
              safeSend(conn, {
                type: "reject",
                protocolVersion: PROTOCOL_VERSION,
                reason: "Player slot does not exist",
              });
              conn.close();
              return;
            }
            if (requestedSlot.connected !== false && requestedSlot.peerId !== conn.peer) {
              safeSend(conn, {
                type: "reject",
                protocolVersion: PROTOCOL_VERSION,
                reason: "Player slot already filled",
              });
              conn.close();
              return;
            }
            targetPlayerIndex = requestedPlayerIndex;
          } else {
            const disconnectedSlots = [...indexedPlayers]
              .sort((left, right) => Number(left.index || 0) - Number(right.index || 0))
              .filter((player) => player.connected === false);
            const matchingDisconnectedSlot = session.matchStarted
              ? disconnectedSlots.find((player) =>
                  playerMatchesPresentedAuditIdentity(
                    player,
                    message.auditPublicKey,
                    message.auditEncryptionPublicKey
                  )
                )
              : null;
            const disconnectedSlot = matchingDisconnectedSlot || disconnectedSlots[0] || null;
            if (disconnectedSlot) {
              targetPlayerIndex = Number(disconnectedSlot.index);
            } else if (!session.matchStarted && indexedPlayers.length < session.desiredPlayers) {
              targetPlayerIndex = indexedPlayers.length;
            }
          }

	          if (targetPlayerIndex == null) {
	            safeSend(conn, {
              type: "reject",
              protocolVersion: PROTOCOL_VERSION,
              reason: session.matchStarted
                ? "No disconnected player slots are available"
                : "Lobby is full",
            });
            conn.close();
	            return;
	          }

	          if (session.matchStarted) {
	            const existingSlot = indexedPlayers.find(
	              (player) => Number(player.index) === targetPlayerIndex
	            );
	            if (isVerifiedMultiplayerSecurityMode(sessionSecurityMode(session))) {
	              const expectedAuditKey = String(existingSlot?.auditPublicKey || "").trim();
	              const presentedAuditKey = String(message.auditPublicKey || "").trim();
	              const expectedEncryptionKey = String(existingSlot?.auditEncryptionPublicKey || "").trim();
	              const presentedEncryptionKey = String(message.auditEncryptionPublicKey || "").trim();
	              const proof = message.reconnectProof || null;
	              if (!expectedAuditKey || !expectedEncryptionKey) {
	                safeSend(conn, {
	                  type: "reject",
	                  protocolVersion: PROTOCOL_VERSION,
	                  reason: "Reconnect audit identity is missing from the player slot",
	                });
	                conn.close();
	                return;
	              }
	              const presentedIdentityMatches = Boolean(
	                presentedAuditKey
	                && presentedAuditKey === expectedAuditKey
	                && presentedEncryptionKey
	                && presentedEncryptionKey === expectedEncryptionKey
	              );
	              if (!presentedIdentityMatches && !proof) {
	                issueReconnectChallenge(conn, targetPlayerIndex);
	                return;
	              }
	              if (!presentedIdentityMatches) {
	                safeSend(conn, {
	                  type: "reject",
	                  protocolVersion: PROTOCOL_VERSION,
	                  reason: "Reconnect audit identity does not match the player slot",
	                });
	                conn.close();
	                return;
	              }
	              const challengeId = String(
	                proof?.challengeId
	                || proof?.requestId
                || message.reconnectChallengeId
                || ""
              );
              if (!challengeId) {
                issueReconnectChallenge(conn, targetPlayerIndex);
                return;
              }
              const challengeKey = reconnectChallengeMapKey(conn.peer, challengeId);
              const challenge = reconnectChallengesRef.current.get(challengeKey);
              if (!challenge) {
                safeSend(conn, {
                  type: "reject",
                  protocolVersion: PROTOCOL_VERSION,
                  reason: "Reconnect challenge expired; retry reconnect",
                });
                conn.close();
                return;
              }
              try {
                await verifyReconnectProofForChallenge(proof, challenge, expectedAuditKey);
              } catch (err) {
                clearReconnectChallenge(conn.peer, challengeId);
                safeSend(conn, {
                  type: "reject",
                  protocolVersion: PROTOCOL_VERSION,
                  reason: toErrorMessage(err, "Reconnect audit proof is invalid"),
                });
                conn.close();
                return;
              }
              clearReconnectChallenge(conn.peer, challengeId);
            }
		          }

	          clientConnectionsRef.current.set(conn.peer, conn);
          const name = sanitizePlayerName(
            message.name,
            `Player ${targetPlayerIndex + 1}`
          );
          const nextSession = updateMultiplayer((prev) => {
            const nextPlayers = [...reindexPlayers(prev.players)];
            const existingSlot = nextPlayers.find(
              (player) => Number(player.index) === targetPlayerIndex
            );
            const basePlayer = existingSlot || {
              name,
              index: targetPlayerIndex,
              ready: false,
              deck: [],
              commanders: [],
            };
	            const nextPlayer = prev.matchStarted
	              ? {
	                  ...basePlayer,
	                  peerId: basePlayer.peerId || conn.peer,
	                  currentPeerId: conn.peer,
	                  connected: true,
	                }
              : {
                  ...basePlayer,
	                  peerId: conn.peer,
	                  name,
	                  auditPublicKey: String(message.auditPublicKey || basePlayer.auditPublicKey || ""),
	                  auditEncryptionPublicKey: String(
	                    message.auditEncryptionPublicKey || basePlayer.auditEncryptionPublicKey || ""
	                  ),
	                  playerGenesisSignature:
                    message.playerGenesisSignature || basePlayer.playerGenesisSignature || null,
	                  deckAuditManifest:
	                    publicDeckManifest(message.deckAuditManifest)
	                    || publicDeckManifest(basePlayer.deckAuditManifest),
                  deck: sanitizeCardList(message.deck),
                  sideboard: sanitizeCardList(message.sideboard),
	                  ziffleKey: message.ziffleKey || basePlayer.ziffleKey || null,
	                  commanders: sanitizeCardList(message.commanders),
                  deckSlotOpenings: sanitizeDeckSlotOpenings(message.deckSlotOpenings),
	                  ready: Boolean(message.ready),
                  deckCount: Number(message.deckCount || 0),
                  sideboardCount: Number(message.sideboardCount || 0),
                  commanderCount: Number(message.commanderCount || 0),
                  connected: true,
                };
            const replaced = existingSlot
              ? nextPlayers.map((player) =>
                  Number(player.index) === targetPlayerIndex ? nextPlayer : player
                )
              : [...nextPlayers, nextPlayer];
            return {
              ...prev,
              mode: prev.matchStarted ? "in_match" : "lobby",
              players: reindexPlayers(replaced),
            };
          });
          const stablePeerId = nextSession.players.find(
            (player) => Number(player.index) === targetPlayerIndex
          )?.peerId;
          if (stablePeerId && stablePeerId !== conn.peer) {
            clientConnectionsRef.current.set(stablePeerId, conn);
          }

          clearLocalDisconnectObservation(conn.peer, targetPlayerIndex);

          if (nextSession.matchStarted) {
            const matchPayload = buildHostedResyncPayload();
            if (!matchPayload) {
              safeSend(conn, {
                type: "action_error",
                protocolVersion: PROTOCOL_VERSION,
                reason: "Host cannot rebuild the current match state",
              });
              return;
            }
            resyncingPeerIdsRef.current.add(conn.peer);
            try {
              await sendHostedStateMessage(conn, {
                type: "state_resync",
                protocolVersion: PROTOCOL_VERSION,
                match: matchPayload,
                lastSequence: nextSession.lastAppliedSequence,
              });
            } catch (err) {
              finishPeerResync(conn.peer);
              safeSend(conn, {
                type: "action_error",
                protocolVersion: PROTOCOL_VERSION,
                reason: toErrorMessage(err, "Host could not serialize current match state"),
              });
              return;
            }
            broadcastMatchPresence(conn.peer, true);
            setStatus(`${name} took over player ${targetPlayerIndex + 1}`);
            return;
          }

          setStatus(`${name} joined as player ${targetPlayerIndex + 1}`);
          broadcastLobbyState();
          return;
        }
        case "resync_ack": {
          const ackSequence = Number(message.lastSequence ?? 0);
          const wasPending = resyncingPeerIdsRef.current.has(conn.peer);
          finishPeerResync(conn.peer);
          if (!wasPending) return;
          const remaining = resyncingPeerIdsRef.current.size;
          setStatus(
            remaining > 0
              ? `Peer resynced; waiting for ${remaining} more`
              : `Peers resynced at action ${ackSequence}`
          );
          return;
        }
        case "resync_request": {
          const session = multiplayerRef.current;
          if (!session.matchStarted) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Match has not started",
            });
            return;
          }

          const existingPlayer = session.players.find((player) =>
            String(player.peerId || "") === String(conn.peer || "")
            || String(player.currentPeerId || "") === String(conn.peer || "")
          );
          if (!existingPlayer) {
            safeSend(conn, {
              type: "reject",
              protocolVersion: PROTOCOL_VERSION,
              reason: "This peer is not part of the active match",
            });
            conn.close();
            return;
          }

          const requesterSequence = Number(message.lastSequence ?? 0);
          if (
            existingPlayer.connected !== false
            && Number.isSafeInteger(requesterSequence)
            && requesterSequence >= Number(session.lastAppliedSequence || 0)
          ) {
            clientConnectionsRef.current.set(conn.peer, conn);
            clearLocalDisconnectObservation(conn.peer, existingPlayer?.index);
            updateMultiplayer((prev) => ({
              ...prev,
              players: markPlayerConnectionState(prev.players, conn.peer, true),
            }));
            safeSend(conn, {
              type: "resync_ack",
              protocolVersion: PROTOCOL_VERSION,
              lastSequence: session.lastAppliedSequence,
            });
            return;
          }

          if (isVerifiedMultiplayerSecurityMode(sessionSecurityMode(session))) {
            if (!message.reconnectProof) {
              issueReconnectChallenge(conn, existingPlayer.index);
              return;
            }
            const challengeId = String(
              message.reconnectProof?.challengeId
              || message.reconnectProof?.requestId
              || message.reconnectChallengeId
              || ""
            );
            const challenge = reconnectChallengesRef.current.get(
              reconnectChallengeMapKey(conn.peer, challengeId)
            );
            if (!challenge) {
              safeSend(conn, {
                type: "reject",
                protocolVersion: PROTOCOL_VERSION,
                reason: "Reconnect challenge expired; retry reconnect",
              });
              conn.close();
              return;
            }
            try {
              await verifyReconnectProofForChallenge(
                message.reconnectProof,
                challenge,
                String(existingPlayer.auditPublicKey || "")
              );
            } catch (err) {
              clearReconnectChallenge(conn.peer, challengeId);
              safeSend(conn, {
                type: "reject",
                protocolVersion: PROTOCOL_VERSION,
                reason: toErrorMessage(err, "Reconnect audit proof is invalid"),
              });
              conn.close();
              return;
            }
            clearReconnectChallenge(conn.peer, challengeId);
          }

          clientConnectionsRef.current.set(conn.peer, conn);
          clearLocalDisconnectObservation(conn.peer, existingPlayer?.index);
          const nextSession = updateMultiplayer((prev) => ({
            ...prev,
            players: markPlayerConnectionState(prev.players, conn.peer, true),
          }));
          const matchPayload = buildHostedResyncPayload();
          if (!matchPayload) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Host cannot rebuild the current match state",
            });
            return;
          }

          resyncingPeerIdsRef.current.add(conn.peer);
          try {
            await sendHostedStateMessage(conn, {
              type: "state_resync",
              protocolVersion: PROTOCOL_VERSION,
              match: matchPayload,
              lastSequence: nextSession.lastAppliedSequence,
            });
          } catch (err) {
            finishPeerResync(conn.peer);
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: toErrorMessage(err, "Host could not serialize current match state"),
            });
            return;
          }
          broadcastMatchPresence(conn.peer, true);
          setStatus(`${existingPlayer.name} is resyncing; host actions paused`);
          return;
        }
        case "deck_update": {
          const session = multiplayerRef.current;
          if (session.matchStarted) return;
          updateMultiplayer((prev) => ({
            ...prev,
            players: reindexPlayers(
              prev.players.map((player) =>
                player.peerId === conn.peer
                  ? {
	                      ...player,
	                      auditPublicKey: String(message.auditPublicKey || player.auditPublicKey || ""),
	                      auditEncryptionPublicKey: String(
	                        message.auditEncryptionPublicKey || player.auditEncryptionPublicKey || ""
	                      ),
	                      playerGenesisSignature:
                        message.playerGenesisSignature || player.playerGenesisSignature || null,
	                      deckAuditManifest:
	                        publicDeckManifest(message.deckAuditManifest)
	                        || publicDeckManifest(player.deckAuditManifest),
                      deck: sanitizeCardList(message.deck),
                      sideboard: sanitizeCardList(message.sideboard),
	                      ziffleKey: message.ziffleKey || player.ziffleKey || null,
	                      commanders: sanitizeCardList(message.commanders),
                      deckSlotOpenings: sanitizeDeckSlotOpenings(message.deckSlotOpenings),
	                      ready: Boolean(message.ready),
                      deckCount: Number(message.deckCount || 0),
                      sideboardCount: Number(message.sideboardCount || 0),
                      commanderCount: Number(message.commanderCount || 0),
                    }
                  : player
              )
            ),
          }));
          broadcastLobbyState();
          return;
        }
        case "rematch_request": {
          const session = multiplayerRef.current;
          if (!session.matchStarted) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Match has not started",
            });
            return;
          }
          const actor = session.players.find((player) => player.peerId === conn.peer);
          if (!actor) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "This peer is not assigned to an active seat",
            });
            return;
          }
          startRematchSideboarding("remote");
          return;
        }
        case "rematch_ready": {
          const session = multiplayerRef.current;
          const rematch = session.rematch;
          if (!session.matchStarted || !rematch || rematch.phase !== "sideboarding") {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "Sideboarding is not active",
            });
            return;
          }
          const actor = rematch.players.find((player) => player.peerId === conn.peer);
          if (!actor) {
            safeSend(conn, {
              type: "action_error",
              protocolVersion: PROTOCOL_VERSION,
              reason: "This peer is not assigned to an active seat",
            });
            return;
          }

          const trustedRematch = isTrustedMultiplayerSecurityMode(sessionSecurityMode(session));
          const nextSession = updateMultiplayer((prev) => {
            if (!prev.rematch) return prev;
            const players = reindexPlayers(prev.rematch.players || []).map((player) => (
	              player.peerId === conn.peer
	                ? {
	                    ...player,
                    auditPublicKey: trustedRematch
                      ? ""
                      : String(message.auditPublicKey || player.auditPublicKey || ""),
                    auditEncryptionPublicKey: trustedRematch
                      ? ""
                      : String(message.auditEncryptionPublicKey || player.auditEncryptionPublicKey || ""),
                    playerGenesisSignature:
                      trustedRematch
                        ? null
                        : message.playerGenesisSignature || player.playerGenesisSignature || null,
                    deck: sanitizeCardList(message.deck),
                    sideboard: sanitizeCardList(message.sideboard),
                    commanders: sanitizeCardList(message.commanders || player.commanders),
	                    deckAuditManifest:
                      trustedRematch
                        ? null
                        : publicDeckManifest(message.deckAuditManifest)
                          || publicDeckManifest(player.deckAuditManifest),
                    deckSlotOpenings: trustedRematch
                      ? []
                      : sanitizeDeckSlotOpenings(message.deckSlotOpenings),
                    ziffleKey: trustedRematch ? null : message.ziffleKey || player.ziffleKey || null,
	                    deckCount: Number(message.deckCount || 0),
	                    sideboardCount: Number(message.sideboardCount || 0),
                    commanderCount: Number(message.commanderCount || player.commanderCount || 0),
	                    ready: true,
	                  }
                : player
            ));
            return {
              ...prev,
              rematch: {
                ...prev.rematch,
                players,
              },
            };
          });
          broadcastRematchState(nextSession.rematch);
          if (rematchPlayersReady(nextSession.rematch?.players)) {
            await startRematchFromState(nextSession.rematch);
          } else {
            setStatus(`${actor.name} is ready for rematch`);
          }
          return;
        }
        default:
          return;
      }
    },
    [
      answerCryptoMaterialRequest,
      answerActionQuorumVoteRequest,
      answerTimeoutVoteRequest,
      answerDisconnectForfeitVoteRequest,
      answerProtocolResponseTimeoutVoteRequest,
      buildHostedResyncPayload,
      broadcastMatchPresence,
      broadcastLobbyState,
      broadcastRematchState,
      finishPeerResync,
      sendHostedStateMessage,
      setStatus,
      startRematchFromState,
      startRematchSideboarding,
      updateMultiplayer,
      resolveCryptoMaterial,
      resolveActionQuorumVote,
      resolveRngCommit,
      resolveRngReveal,
      resolveTimeoutVote,
      resolveZiffleRevealToken,
      resolveZiffleShuffleStep,
    ]
  );

  const configureHostConnection = useCallback(
    (conn) => {
      const heartbeatKey = connectionHeartbeatKey("client", conn.peer);
      const beginHeartbeat = () => startConnectionHeartbeat(heartbeatKey, conn, () => {
        if (clientConnectionsRef.current.get(conn.peer) !== conn) return;
        handleClientDisconnect(conn.peer);
      });
      if (conn.open) {
        beginHeartbeat();
      } else {
        conn.on("open", beginHeartbeat);
      }
      conn.on("data", (message) => {
        markConnectionAlive(heartbeatKey);
        if (handleConnectionHeartbeatMessage(conn, message)) return;
        const handleError = (err) => {
          if (shouldSuppressProtocolMessageError(err, message)) return;
          safeSend(conn, {
            type: "action_error",
            protocolVersion: PROTOCOL_VERSION,
            reason: toErrorMessage(err),
            rollbackSynced: Boolean(err?.syncedRollbackBroadcast),
            rollbackSequence:
              err?.syncedRollbackSequence == null ? null : Number(err.syncedRollbackSequence),
          });
        };
        if (message?.type === "apply_action") {
          void enqueueAsync(clientMessageQueueRef, () =>
            handleClientMessage(conn, message)
          ).catch(handleError);
          return;
        }
        // Acks must not wait behind actions that are blocked on those same acks.
        if (
          message?.type === "resync_ack"
          || message?.type === "ziffle_shuffle_step_request"
          || message?.type === "ziffle_shuffle_step_response"
          || message?.type === "ziffle_reveal_token_request"
          || message?.type === "ziffle_reveal_token_response"
          || message?.type === "rng_commit_request"
          || message?.type === "rng_commit_response"
          || message?.type === "rng_reveal_request"
          || message?.type === "rng_reveal_response"
          || message?.type === "timeout_vote_request"
          || message?.type === "timeout_vote_response"
          || message?.type === "disconnect_forfeit_vote_request"
          || message?.type === "disconnect_forfeit_vote_response"
          || message?.type === "protocol_timeout_vote_request"
          || message?.type === "protocol_timeout_vote_response"
          || message?.type === "action_quorum_vote_request"
          || message?.type === "action_quorum_vote_response"
          || message?.type === "action_intent_progress"
          || message?.type === "action_intent_cancel"
        ) {
          void handleClientMessage(conn, message).catch(handleError);
          return;
        }
        void enqueueAsync(clientMessageQueueRef, () =>
          handleClientMessage(conn, message)
        ).catch(handleError);
      });
      conn.on("close", () => {
        clearConnectionHeartbeat(heartbeatKey);
        if (clientConnectionsRef.current.get(conn.peer) !== conn) return;
        handleClientDisconnect(conn.peer);
      });
      conn.on("error", () => {
        clearConnectionHeartbeat(heartbeatKey);
        if (clientConnectionsRef.current.get(conn.peer) !== conn) return;
        handleClientDisconnect(conn.peer);
      });
    },
    [
      clearConnectionHeartbeat,
      handleClientDisconnect,
      handleClientMessage,
      handleConnectionHeartbeatMessage,
      markConnectionAlive,
      startConnectionHeartbeat,
    ]
  );

  const configureIncomingConnection = useCallback(
    (conn) => {
      if (conn?.metadata?.channel === "peer-direct") {
        configurePeerConnection(conn);
        return;
      }
      configureHostConnection(conn);
    },
    [configureHostConnection, configurePeerConnection]
  );

  const promoteLocalPlayerToHost = useCallback(
    (reason = "Lobby host disconnected.") => {
      const session = multiplayerRef.current;
      const lobbyId = String(session.lobbyId || session.hostPeerId || "").trim();
      const localPlayerIndex = resolveReconnectPlayerIndex(session, lobbyId);
      if (session.role !== "client" || !lobbyId || localPlayerIndex == null) {
        return false;
      }

	      // Older clients did not preserve a stable hostPeerId in every payload.
	      // If the host id no longer matches any slot, treat player 1 as the
	      // disconnected original host so the remaining clients can still elect.
	      const reclaimingOriginalHost = localPlayerIndex === 0;
	      if (reclaimingOriginalHost) {
	        updateMultiplayer((prev) => ({
	          ...prev,
	          mode: prev.matchStarted ? "in_match" : "joining",
	          hostPeerId: lobbyId,
	          submittingAction: false,
	        }));
	        return false;
	      }
	      const fallbackHostIndex = 0;
	      const playersWithHostOffline = markHostPeerDisconnected(
	        session.players,
	        session.hostPeerId || lobbyId,
	        lobbyId,
	        fallbackHostIndex
	      );
      const nextHost = findNextHostPlayer(playersWithHostOffline);
      if (!nextHost || Number(nextHost.index) !== localPlayerIndex) {
        updateMultiplayer((prev) => ({
          ...prev,
          mode: prev.matchStarted ? "in_match" : "joining",
          hostPeerId: lobbyId,
          players: playersWithHostOffline,
          submittingAction: false,
        }));
        return false;
      }

      const previousPeer = peerRef.current;
      const previousHostConnection = hostConnectionRef.current;
      hostConnectionRef.current = null;
      if (previousHostConnection) {
        try {
          previousHostConnection.close();
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
      clearAllConnectionHeartbeats();
      clearAllPeerResyncs();
      awaitingStateResyncRef.current = false;

      if (previousPeer) {
        try {
          previousPeer.destroy();
        } catch (err) {
          void err;
        }
      }
      peerRef.current = null;

      updateMultiplayer((prev) => ({
        ...prev,
        role: "host",
        mode: "hosting",
        lobbyId,
        hostPeerId: lobbyId,
        localPlayerIndex,
        desiredPlayers: Math.max(
          Number(prev.desiredPlayers || 0),
          playersWithHostOffline.length,
          2
        ),
        players: reclaimingOriginalHost
          ? ensurePromotedLocalPlayer(prev.players, prev, lobbyId, localPlayerIndex)
          : markHostPeerDisconnected(
              prev.players,
              prev.hostPeerId || lobbyId,
              lobbyId,
              fallbackHostIndex
            ),
        submittingAction: false,
      }));

      let takeoverAttempts = 0;
      const startTakeoverPeer = () => {
        const latestSession = multiplayerRef.current;
        if (
          latestSession.role !== "host" ||
          latestSession.lobbyId !== lobbyId ||
          normalizePlayerIndex(latestSession.localPlayerIndex) !== localPlayerIndex
        ) {
          return;
        }

        takeoverAttempts += 1;
        const takeoverPeer = createPeer(lobbyId, peerOptionsRef.current);
        peerRef.current = takeoverPeer;
        let reconnectTimer = null;
        let reconnectAttempts = 0;
        const clearReconnect = () => {
          if (reconnectTimer) {
            clearTimeout(reconnectTimer);
            reconnectTimer = null;
          }
          reconnectAttempts = 0;
        };
        const scheduleReconnect = (statusReason) => {
          if (peerRef.current !== takeoverPeer || takeoverPeer.destroyed || reconnectTimer) return;
          reconnectAttempts += 1;
          const delay = Math.min(8000, 1000 * reconnectAttempts);
          setStatus(
            `${statusReason} Retrying signaling in ${Math.ceil(delay / 1000)}s...`,
            true
          );
          reconnectTimer = window.setTimeout(() => {
            reconnectTimer = null;
            if (peerRef.current !== takeoverPeer || takeoverPeer.destroyed) return;
            try {
              takeoverPeer.reconnect();
            } catch (err) {
              setStatus(formatPeerError(err, "Could not reconnect lobby signaling"), true);
              leaveLobby("");
            }
          }, delay);
        };
        const scheduleTakeoverRetry = (err) => {
          if (peerRef.current !== takeoverPeer) return;
          try {
            takeoverPeer.destroy();
          } catch (destroyErr) {
            void destroyErr;
          }
          peerRef.current = null;
          const delay = Math.min(8000, 1000 * takeoverAttempts);
          setStatus(
            `${formatPeerError(err, "Could not take over lobby host")} Retrying host takeover in ${Math.ceil(delay / 1000)}s...`,
            true
          );
          window.setTimeout(startTakeoverPeer, delay);
        };
        const openTimeout = window.setTimeout(() => {
          if (peerRef.current !== takeoverPeer || takeoverPeer.open) return;
          scheduleTakeoverRetry("Timed out while claiming the lobby host id");
        }, PEER_OPEN_TIMEOUT_MS);

        setStatus(
          takeoverAttempts === 1
            ? `${reason} Taking over lobby host...`
            : "Retrying lobby host takeover..."
        );

        takeoverPeer.on("open", (peerId) => {
          if (peerRef.current !== takeoverPeer) return;
          clearTimeout(openTimeout);
          clearReconnect();
          const current = multiplayerRef.current;
          const currentDeck = parseDeckSubmission(
            current.format,
            current.localDeckText,
            current.localCommanderText
          );
          const nextPlayers = reindexPlayers(
            current.players.map((player) => {
              if (Number(player.index) === localPlayerIndex) {
                const nextPlayer = {
                  ...player,
                  peerId,
                  currentPeerId: peerId,
                  auditPublicKey: current.matchStarted
                    ? player.auditPublicKey
                    : auditPublicKeyRef.current || player.auditPublicKey || "",
                  auditEncryptionPublicKey: current.matchStarted
                    ? player.auditEncryptionPublicKey
                    : auditEncryptionPublicKeyRef.current || player.auditEncryptionPublicKey || "",
                  connected: true,
                };
                return current.matchStarted
                  ? nextPlayer
                  : withDeckState(
                      nextPlayer,
                      current.format,
                      currentDeck.deck,
                      currentDeck.commanders,
                      currentDeck.sideboard
                    );
              }
              return {
                ...player,
                peerId:
                  player.peerId === peerId
                    ? createOfflinePeerId(lobbyId, player.index)
                    : player.peerId,
                connected: false,
                disconnectedAtMs: player.disconnectedAtMs || Date.now(),
                autoForfeitAtMs:
                  player.autoForfeitAtMs
                  || (Date.now() + DISCONNECT_AUTO_FORFEIT_MS),
              };
            })
          );
          writeStoredPlayerIndex(lobbyId, localPlayerIndex);
          const nextSession = updateMultiplayer((prev) => ({
            ...prev,
            role: "host",
            mode: prev.matchStarted ? "in_match" : "lobby",
            lobbyId,
            hostPeerId: peerId,
            localPeerId: peerId,
            localPlayerIndex,
            localDeckCount: currentDeck.deckCount,
            localCommanderCount: currentDeck.commanderCount,
            players: nextPlayers,
            submittingAction: false,
          }));

	          if (matchStartPayloadRef.current) {
	            matchStartPayloadRef.current = {
	              ...cloneMultiplayerPayload(matchStartPayloadRef.current),
	              currentLobbyId: lobbyId,
	              currentHostPeerId: peerId,
	              currentHostPlayerIndex: localPlayerIndex,
	              currentPlayers: toPublicPlayers(nextSession.players),
	            };
	          }

          rememberDefaultLobbyDeck(current.localDeckText, current.localCommanderText);
          setStatus(`You are now the lobby host: ${lobbyId}`);
        });
        takeoverPeer.on("connection", configureIncomingConnection);
        takeoverPeer.on("error", (err) => {
          if (peerRef.current !== takeoverPeer) return;
          clearTimeout(openTimeout);
          const type = String(err?.type || "").trim();
          if (type === "unavailable-id" || isRecoverablePeerError(err)) {
            scheduleTakeoverRetry(err);
            return;
          }
          setStatus(formatPeerError(err, "Lobby host takeover failed"), true);
          leaveLobby("");
        });
        takeoverPeer.on("disconnected", () => {
          if (peerRef.current !== takeoverPeer) return;
          clearTimeout(openTimeout);
          scheduleReconnect(
            `Disconnected from the PeerJS signaling server (${peerServerLabelRef.current}).`
          );
        });
        takeoverPeer.on("close", () => {
          clearTimeout(openTimeout);
          clearReconnect();
        });
      };

      startTakeoverPeer();
      return true;
    },
    [
      clearAllPeerResyncs,
      clearAllConnectionHeartbeats,
      configureIncomingConnection,
      leaveLobby,
      peerOptionsRef,
      setStatus,
      updateMultiplayer,
    ]
  );

  const createLobby = useCallback(
    async ({
      name,
      desiredPlayers,
      startingLife,
      format = MATCH_FORMAT_NORMAL,
      securityMode = MULTIPLAYER_SECURITY_TRUSTED,
      deckText = "",
      commanderText = "",
    }) => {
      teardownPeer();
      const normalizedSecurityMode = normalizeMultiplayerSecurityMode(securityMode);
      const auditIdentity = isVerifiedMultiplayerSecurityMode(normalizedSecurityMode)
        ? await ensureAuditIdentity()
        : { publicKey: "", encryptionPublicKey: "" };
      const {
        publicKey: auditPublicKey,
        encryptionPublicKey: auditEncryptionPublicKey,
      } = auditIdentity;
      const localName = sanitizePlayerName(name, "Host");
      const targetPlayers = Math.max(
        CURRENT_AUDIT_MIN_PLAYERS,
        Math.min(
          CURRENT_AUDIT_MAX_PLAYERS,
          Number(desiredPlayers) || CURRENT_AUDIT_MIN_PLAYERS
        )
      );
      const lifeTotal = Math.max(1, Number(startingLife) || 20);
      const normalizedFormat = normalizeMatchFormat(format);
      const deckSubmission = parseDeckSubmission(
        normalizedFormat,
        deckText,
        commanderText
      );
      const peer = createPeer("", peerOptionsRef.current);
      peerRef.current = peer;
      let reconnectTimer = null;
      let reconnectAttempts = 0;
      const clearReconnect = () => {
        if (reconnectTimer) {
          clearTimeout(reconnectTimer);
          reconnectTimer = null;
        }
        reconnectAttempts = 0;
      };
      const scheduleReconnect = (reason) => {
        if (peerRef.current !== peer || peer.destroyed || reconnectTimer) return;
        reconnectAttempts += 1;
        const delay = Math.min(8000, 1000 * reconnectAttempts);
        setStatus(
          `${reason} Retrying signaling in ${Math.ceil(delay / 1000)}s...`,
          true
        );
        reconnectTimer = window.setTimeout(() => {
          reconnectTimer = null;
          if (peerRef.current !== peer || peer.destroyed) return;
          try {
            peer.reconnect();
          } catch (err) {
            setStatus(formatPeerError(err, "Could not reconnect lobby signaling"), true);
            leaveLobby("");
          }
        }, delay);
      };
      const openTimeout = window.setTimeout(() => {
        if (peerRef.current !== peer || peer.open) return;
        setStatus(
          `Could not register the lobby with the PeerJS signaling server (${peerServerLabelRef.current}).`,
          true
        );
        leaveLobby("");
      }, PEER_OPEN_TIMEOUT_MS);

      updateMultiplayer({
        ...createEmptyState(),
        role: "host",
        mode: "hosting",
        localName,
        desiredPlayers: targetPlayers,
        startingLife: lifeTotal,
        format: normalizedFormat,
        securityMode: normalizedSecurityMode,
        signalingServer: peerServerLabelRef.current,
        localDeckText: String(deckText || ""),
        localCommanderText: String(commanderText || ""),
        localDeckCount: deckSubmission.deckCount,
        localCommanderCount: deckSubmission.commanderCount,
      });
      setStatus(`Registering lobby with PeerJS (${peerServerLabelRef.current})...`);

      peer.on("open", async (peerId) => {
        clearTimeout(openTimeout);
        clearReconnect();
        const session = multiplayerRef.current;
        const isReconnect =
          session.role === "host" && session.localPeerId && session.localPeerId === peerId;
        if (isReconnect) {
          updateMultiplayer((prev) => ({
            ...prev,
            mode: prev.matchStarted ? "in_match" : "lobby",
            lobbyId: prev.lobbyId || peerId,
            hostPeerId: peerId,
            localPeerId: peerId,
            players: prev.players.map((player) =>
              player.peerId === peerId
                ? { ...player, auditPublicKey, auditEncryptionPublicKey }
                : player
            ),
          }));
          if (!session.matchStarted) {
            broadcastLobbyState();
          }
          setStatus(`Lobby signaling reconnected: ${peerId}`);
          return;
        }
        const currentDeck = parseDeckSubmission(
          session.format,
          session.localDeckText,
          session.localCommanderText
        );
        let deckAuditManifest = null;
        let deckSlotOpenings = [];
        let ziffleKey = null;
        let playerGenesisSignature = null;
        if (isVerifiedMultiplayerSecurityMode(normalizedSecurityMode)) {
          deckAuditManifest = await buildLocalDeckAuditManifest({
            matchId: peerId,
            owner: 0,
            deck: currentDeck.deck,
            sideboard: currentDeck.sideboard,
            commanders: currentDeck.commanders,
          });
          deckSlotOpenings = deckSlotOpeningsForManifest(deckAuditManifest);
          const ziffleKeyPair = await ensureZiffleIdentity({
            context: peerId,
            deckCount: currentDeck.deckCount || 60,
          });
          ziffleKey = publicZiffleKey(ziffleKeyPair, 0);
          playerGenesisSignature = await signPlayerGenesis({
            matchId: peerId,
            player: {
              peerId,
              name: localName,
              index: 0,
              auditPublicKey,
              auditEncryptionPublicKey,
              deckAuditManifest: publicDeckManifest(deckAuditManifest),
              ziffleKey,
              ...openDecklistPlayerFields({
                deck: currentDeck.deck,
                sideboard: currentDeck.sideboard,
                commanders: currentDeck.commanders,
                deckSlotOpenings,
              }),
              deckCount: currentDeck.deckCount,
              sideboardCount: currentDeck.sideboard.length,
              commanderCount: currentDeck.commanderCount,
            },
          });
        }
        writeStoredPlayerIndex(peerId, 0);
        updateMultiplayer((prev) => ({
          ...prev,
          mode: "lobby",
          lobbyId: peerId,
          hostPeerId: peerId,
          localPeerId: peerId,
          localPlayerIndex: 0,
          localDeckCount: currentDeck.deckCount,
          localCommanderCount: currentDeck.commanderCount,
          players: [
            withDeckState(
              {
                peerId,
                name: localName,
                index: 0,
                auditPublicKey,
                auditEncryptionPublicKey,
                deckAuditManifest: publicDeckManifest(deckAuditManifest),
                deckSlotOpenings,
                ziffleKey,
                playerGenesisSignature,
                connected: true,
              },
              prev.format,
              currentDeck.deck,
              currentDeck.commanders,
              currentDeck.sideboard
            ),
          ],
        }));
        rememberDefaultLobbyDeck(session.localDeckText, session.localCommanderText);
        setStatus(`Lobby created: ${peerId}`);
      });
      peer.on("connection", configureIncomingConnection);
      peer.on("error", (err) => {
        clearTimeout(openTimeout);
        if (isRecoverablePeerError(err)) {
          scheduleReconnect(formatPeerError(err, "Lost lobby signaling"));
          return;
        }
        setStatus(formatPeerError(err, "Lobby error"), true);
        leaveLobby("");
      });
      peer.on("disconnected", () => {
        clearTimeout(openTimeout);
        scheduleReconnect(
          `Disconnected from the PeerJS signaling server (${peerServerLabelRef.current}).`
        );
      });
      peer.on("close", () => {
        clearTimeout(openTimeout);
        clearReconnect();
      });
    },
    [
      broadcastLobbyState,
      buildLocalDeckAuditManifest,
      configureIncomingConnection,
      ensureAuditIdentity,
      ensureZiffleIdentity,
      leaveLobby,
      peerOptionsRef,
      publicZiffleKey,
      setStatus,
      signPlayerGenesis,
      teardownPeer,
      updateMultiplayer,
    ]
  );

  const joinLobby = useCallback(
    async ({ name, lobbyId, deckText = "", commanderText = "" }) => {
      teardownPeer();
      const localName = sanitizePlayerName(name, "Guest");
      const targetLobby = String(lobbyId || "").trim();
      if (!targetLobby) {
        setStatus("Lobby code is required", true);
        return;
      }

      const deckSubmission = parseDeckSubmission(
        MATCH_FORMAT_NORMAL,
        deckText,
        commanderText
      );
      const peer = createPeer("", peerOptionsRef.current);
      peerRef.current = peer;
      let reconnectTimer = null;
      let reconnectAttempts = 0;
      let hostReconnectTimer = null;
      let hostReconnectAttempts = 0;
      const clearReconnect = () => {
        if (reconnectTimer) {
          clearTimeout(reconnectTimer);
          reconnectTimer = null;
        }
        reconnectAttempts = 0;
      };
      const clearHostReconnect = () => {
        if (hostReconnectTimer) {
          clearTimeout(hostReconnectTimer);
          hostReconnectTimer = null;
        }
        hostReconnectAttempts = 0;
      };
      const peerOpenTimeout = window.setTimeout(() => {
        if (peerRef.current !== peer || peer.open) return;
        setStatus(
          `Could not connect to the PeerJS signaling server (${peerServerLabelRef.current}).`,
          true
        );
        leaveLobby("");
      }, PEER_OPEN_TIMEOUT_MS);

      updateMultiplayer({
        ...createEmptyState(),
        role: "client",
        mode: "joining",
        lobbyId: targetLobby,
        hostPeerId: targetLobby,
        localName,
        signalingServer: peerServerLabelRef.current,
        localDeckText: String(deckText || ""),
        localCommanderText: String(commanderText || ""),
        localDeckCount: deckSubmission.deckCount,
        localCommanderCount: deckSubmission.commanderCount,
      });
      setStatus(`Connecting to the PeerJS signaling server (${peerServerLabelRef.current})...`);

      const scheduleHostReconnect = (reason) => {
        if (peerRef.current !== peer || peer.destroyed) {
          return;
        }
        const inMatch = multiplayerRef.current.matchStarted;

        updateMultiplayer((prev) => ({
          ...prev,
          mode: inMatch ? "in_match" : "joining",
          submittingAction: false,
        }));

        if (peer.disconnected || !peer.open) {
          setStatus(`${reason} Waiting for the signaling server to reconnect...`, true);
          return;
        }
        if (hostReconnectTimer) return;

        hostReconnectAttempts += 1;
        const delay = Math.min(8000, 1000 * hostReconnectAttempts);
        setStatus(
          `${reason} Retrying ${inMatch ? "match host" : "lobby host"} in ${Math.ceil(delay / 1000)}s...`,
          true
        );
        hostReconnectTimer = window.setTimeout(() => {
          hostReconnectTimer = null;
          if (peerRef.current !== peer || peer.destroyed) {
            return;
          }
          setStatus(inMatch ? "Reconnecting to match host..." : "Reconnecting to lobby host...");
          connectToHost();
        }, delay);
      };

      const connectToHost = () => {
        if (peerRef.current !== peer || peer.destroyed) return;

        const currentConn = hostConnectionRef.current;
        if (currentConn?.open) return;
        if (currentConn) {
          hostConnectionRef.current = null;
          clearConnectionHeartbeat(connectionHeartbeatKey("host", currentConn.peer));
          try {
            currentConn.close();
          } catch (err) {
            void err;
          }
        }

        const hostTarget =
          String(multiplayerRef.current.hostPeerId || targetLobby).trim() || targetLobby;
	        const conn = peer.connect(hostTarget, {
	          reliable: true,
	          serialization: "binary",
	        });
        hostConnectionRef.current = conn;
        const heartbeatKey = connectionHeartbeatKey("host", hostTarget);
        const connOpenTimeout = window.setTimeout(() => {
          if (hostConnectionRef.current !== conn || conn.open) return;
          hostConnectionRef.current = null;
          clearConnectionHeartbeat(heartbeatKey);
          try {
            conn.close();
          } catch (err) {
            void err;
          }
          scheduleHostReconnect(
            "Could not reach the lobby host. If the code is correct, this is usually a WebRTC connectivity issue between the two machines."
          );
        }, PEER_CONNECT_TIMEOUT_MS);
        const clearJoinTimeouts = () => {
          clearTimeout(peerOpenTimeout);
          clearTimeout(connOpenTimeout);
        };
        const handleHostConnectionLost = (reason) => {
          if (hostConnectionRef.current !== conn) return;
          clearJoinTimeouts();
          clearConnectionHeartbeat(heartbeatKey);
          hostConnectionRef.current = null;
          const hostPlayer = multiplayerRef.current.players.find(
            (player) => String(player?.peerId || "") === hostTarget
          );
          rememberLocalDisconnectObservation(hostTarget, {
            playerIndex: hostPlayer?.index,
            disconnectedAtMs: Date.now(),
            source: "host_connection",
          });
          updateMultiplayer((prev) => ({
            ...prev,
            players: markPlayerConnectionState(prev.players, hostTarget, false),
          }));
          if (promoteLocalPlayerToHost(reason)) return;
          scheduleHostReconnect(reason);
        };
        conn.on("open", async () => {
          if (hostConnectionRef.current !== conn) return;
          clearJoinTimeouts();
          clearHostReconnect();
          const hostPlayer = multiplayerRef.current.players.find(
            (player) => String(player?.peerId || "") === hostTarget
          );
          clearLocalDisconnectObservation(hostTarget, hostPlayer?.index);
          startConnectionHeartbeat(heartbeatKey, conn, () => {
            handleHostConnectionLost("Lost heartbeat from lobby host.");
          });
          updateMultiplayer((prev) => ({
            ...prev,
            players: markPlayerConnectionState(prev.players, hostTarget, true),
          }));
	          const session = multiplayerRef.current;
	          if (session.matchStarted) {
	            safeSend(conn, {
              type: "resync_request",
              protocolVersion: PROTOCOL_VERSION,
              lastSequence: session.lastAppliedSequence,
            });
            setStatus(`Reconnected to match host ${hostTarget}`);
            return;
          }

          const currentDeck = parseDeckSubmission(
            session.format,
            session.localDeckText,
            session.localCommanderText
          );
          const requestedPlayerIndex = resolveReconnectPlayerIndex(session, targetLobby);
          const joinRequest = {
            type: "join_request",
            protocolVersion: PROTOCOL_VERSION,
            name: localName,
            securityMode: sessionSecurityMode(session),
            deck: currentDeck.deck,
            sideboard: currentDeck.sideboard,
            deckSlotOpenings: [],
            deckAuditManifest: null,
            ziffleKey: null,
            deckCount: currentDeck.deckCount,
            sideboardCount: currentDeck.sideboard.length,
            commanders: currentDeck.commanders,
            commanderCount: currentDeck.commanderCount,
		            ready: currentDeck.ready,
          };
          if (requestedPlayerIndex != null) {
            joinRequest.requestedPlayerIndex = requestedPlayerIndex;
          }
          safeSend(conn, joinRequest);
          setStatus(`Joined lobby ${targetLobby}`);
        });
        conn.on("iceStateChanged", (state) => {
          if (hostConnectionRef.current !== conn) return;
          if (state === "checking") {
            setStatus("Negotiating direct peer connection...");
            return;
          }
          if (state === "failed") {
            handleHostConnectionLost(
              "Could not establish a direct peer connection to the lobby host. The two machines likely need TURN relay support."
            );
            return;
          }
          if (state === "disconnected") {
            setStatus(
              multiplayerRef.current.matchStarted
                ? "Peer connection interrupted. Attempting to reconnect to the host..."
                : "Peer connection interrupted while joining.",
              true
            );
          }
        });
        conn.on("data", (message) => {
          if (hostConnectionRef.current !== conn) return;
          markConnectionAlive(heartbeatKey);
          if (handleConnectionHeartbeatMessage(conn, message)) return;
          if (message?.type === "apply_action") {
            void enqueueAsync(hostMessageQueueRef, () => handleHostMessage(message)).catch((err) => {
              emitSyncFailureNotice(
                "Sync failed",
                err instanceof Error ? err.message : String(err)
              );
              setStatus(`Lobby message failed: ${err}`, true);
            });
            return;
          }
          if (
            message?.type === "ziffle_shuffle_step_request"
            || message?.type === "ziffle_shuffle_step_response"
            || message?.type === "ziffle_reveal_token_request"
            || message?.type === "ziffle_reveal_token_response"
            || message?.type === "rng_commit_request"
            || message?.type === "rng_commit_response"
            || message?.type === "rng_reveal_request"
            || message?.type === "rng_reveal_response"
            || message?.type === "timeout_vote_request"
            || message?.type === "timeout_vote_response"
            || message?.type === "disconnect_forfeit_vote_request"
            || message?.type === "disconnect_forfeit_vote_response"
            || message?.type === "protocol_timeout_vote_request"
            || message?.type === "protocol_timeout_vote_response"
            || message?.type === "action_quorum_vote_request"
            || message?.type === "action_quorum_vote_response"
            || message?.type === "action_intent_progress"
            || message?.type === "action_intent_cancel"
          ) {
            void handleHostMessage(message).catch((err) => {
              if (shouldSuppressProtocolMessageError(err, message)) return;
              emitZiffleDiagnosticNotice("Ziffle message failed", err, {
                phase: "host_ziffle_message",
                messageType: String(message?.type || ""),
                requestId: String(message?.requestId || ""),
              });
              setStatus(`Ziffle message failed: ${err}`, true);
            });
            return;
          }
          void enqueueAsync(hostMessageQueueRef, () => handleHostMessage(message)).catch((err) => {
            emitSyncFailureNotice(
              "Sync failed",
              err instanceof Error ? err.message : String(err)
            );
            setStatus(`Lobby message failed: ${err}`, true);
          });
        });
        conn.on("close", () => {
          if (hostConnectionRef.current !== conn) return;
          handleHostConnectionLost("Disconnected from lobby host.");
        });
        conn.on("error", (err) => {
          if (hostConnectionRef.current !== conn) return;
          const reason = formatPeerError(err, "Lobby connection failed");
          handleHostConnectionLost(reason);
        });
      };
      const scheduleReconnect = (reason) => {
        if (peerRef.current !== peer || peer.destroyed || reconnectTimer) return;
        reconnectAttempts += 1;
        const delay = Math.min(8000, 1000 * reconnectAttempts);
        setStatus(
          `${reason} Retrying signaling in ${Math.ceil(delay / 1000)}s...`,
          true
        );
        reconnectTimer = window.setTimeout(() => {
          reconnectTimer = null;
          if (peerRef.current !== peer || peer.destroyed) return;
          try {
            peer.reconnect();
          } catch (err) {
            setStatus(formatPeerError(err, "Could not reconnect lobby signaling"), true);
            leaveLobby("");
          }
        }, delay);
      };

      peer.on("open", (peerId) => {
        clearTimeout(peerOpenTimeout);
        clearReconnect();
        clearHostReconnect();
        const previousPeerId = multiplayerRef.current.localPeerId;
        updateMultiplayer((prev) => ({
          ...prev,
          localPeerId: peerId,
        }));
        if (previousPeerId === peerId && hostConnectionRef.current?.open) {
          setStatus(`Lobby signaling reconnected: ${peerId}`);
          return;
        }
        setStatus("Connecting to lobby host...");
        connectToHost();
      });
      peer.on("connection", (conn) => {
        if (conn?.metadata?.channel === "peer-direct") {
          configurePeerConnection(conn);
          return;
        }
        try {
          conn.close();
        } catch (err) {
          void err;
        }
      });
      peer.on("error", (err) => {
        clearTimeout(peerOpenTimeout);
        if (isRecoverablePeerError(err)) {
          scheduleReconnect(formatPeerError(err, "Lost lobby signaling"));
          return;
        }
        const errorMessage = formatPeerError(err, "Lobby error");
        if (String(err?.type || "").trim() === "peer-unavailable") {
          const reason = "Lobby host is not registered on the signaling server yet.";
          if (promoteLocalPlayerToHost(reason)) return;
          scheduleHostReconnect(reason);
          return;
        }
        setStatus(errorMessage, true);
        leaveLobby("");
      });
      peer.on("disconnected", () => {
        clearTimeout(peerOpenTimeout);
        scheduleReconnect(
          `Disconnected from the PeerJS signaling server (${peerServerLabelRef.current}).`
        );
      });
      peer.on("close", () => {
        clearTimeout(peerOpenTimeout);
        clearReconnect();
        clearHostReconnect();
      });
    },
    [
      clearConnectionHeartbeat,
      configurePeerConnection,
      emitZiffleDiagnosticNotice,
      handleHostMessage,
      handleConnectionHeartbeatMessage,
      leaveLobby,
      markConnectionAlive,
      peerOptionsRef,
      promoteLocalPlayerToHost,
      setStatus,
      startConnectionHeartbeat,
      teardownPeer,
      updateMultiplayer,
    ]
  );


  return { applyStateResync, broadcastLobbyState, broadcastRematchState, clearReconnectChallenge, configureHostConnection, configureIncomingConnection, configurePeerConnection, connectDirectPeer, createLobby, handleClientDisconnect, handleClientMessage, handleHostMessage, handlePeerDisconnect, handlePeerMessage, issueReconnectChallenge, joinLobby, promoteLocalPlayerToHost, publishLocalDeckUpdateForAssignedSeat, readyForRematch, reconnectChallengeMapKey, reportSyncFailure, requestResync, sendDirectPeerMessage, startHostedMatch, startRematchFromState, startRematchSideboarding, startTrustedMatchFromPlayers, updateRematchDecks };
}
