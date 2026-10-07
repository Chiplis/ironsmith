import { assertPaymentDisclosureAuthority, createPaymentDisclosureJournal } from "../../lib/payment-disclosure-journal.js";
import { isPrivateZiffleEpoch, ziffleInputDeckFields } from "../../lib/ziffle-private-epochs.js";
import { differsBeyondClock } from "../../lib/value-store.js";
import { saveRelayLobby } from '../../lib/relay/session.js';
import {
  ACTION_INTENT_DOMAIN,
  ACTION_SUBMISSION_IDLE_WAIT_MS,
  MATCH_CLOCK_CLAIM_SKEW_MS,
  MAX_PENDING_ACTION_INTENT_MS,
  PROTOCOL_RESPONSE_TIMEOUT_MS,
  PROTOCOL_VERSION,
  Peer,
  ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD,
  actionIntentFingerprint,
  actionIntentKey,
  buildDeckSlotOpening,
  buildSignedPlayerGenesis,
  buildZiffleOpeningProof,
  canonicalMultiplayerPayload,
  clearStoredAuditIdentity,
  cloneMultiplayerPayload,
  compactZiffleCeremonyForDiagnostics,
  compactZiffleDiagnosticsJson,
  createAuditEncryptionKey,
  createAuditSessionKey,
  emitSyncFailureNotice,
  exportAuditEncryptionKeyPair,
  exportAuditEncryptionPublicKey,
  exportAuditKeyPair,
  exportAuditPublicKey,
  getPeerSessionStorage,
  importAuditEncryptionKeyPair,
  importAuditKeyPair,
  isForfeitCommand,
  isNonDispatchSyncCommand,
  importAuditPublicKey,
  isProtocolResponseWaitTimeout,
  mergeActionOpeningPreviews,
  normalizeActionOpeningPreview,
  normalizePlayerIndex,
  normalizeShuffleOrder,
  nowMonotonicMs,
  payloadSizeBytes,
  playerNameForIndex,
  preloadPrivateDeckManifestArt,
  privateDeckManifestStorageKey,
  publicDeckManifest,
  randomAuditHex,
  readStoredAuditIdentity,
  readStoredPrivateDeckManifest,
  readStoredRevealedOpening,
  readStoredZiffleIdentity,
  reconnectProofPayload,
  recordPeerSyncPerf,
  reindexPlayers,
  purgeStoredZifflePositionOpeningsForMatchOwner,
  removeStoredRevealedOpening,
  resolveLocalPlayerIndex,
  resolveLocalPlayerIndexFromPeer,
  safeSend,
  sanitizeDeckSlotOpenings,
  sha256Hex,
  signAuditPayload,
  signedActionIntentPayload,
  sleep,
  stripTransientZifflePositionOpeningFields,
  toErrorMessage,
  useCallback,
  verifyAuditPayload,
  withConnectionWarnings,
  writeStoredAuditIdentity,
  writeStoredPrivateDeckManifest,
  writeStoredRevealedOpening,
  writeStoredZiffleIdentity,
  ziffleCeremonyForOpeningProof,
  ziffleContextFromCeremony,
  ziffleContextFromOpening,
  ziffleDeckHashFromCommitment,
  ziffleDiagnosticNoticeBody,
  ziffleKeyContextForCeremony,
  zifflePositionFromCommitment,
  ziffleRevealTokenTimeoutMs,
  ziffleRuntimeCommitment,
  ziffleOriginAnchorFromOpening,
  assertZiffleOpeningOriginMatchesMetadata,
} from "./shared.js";
import { recordPeerRtt } from "../../lib/action-diagnostics.js";
import { MAX_ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS } from "../../lib/ziffle-timeouts.js";
import { buildZiffleRuntimeManifest } from "../../lib/ziffle-runtime-manifest.js";
import { checkPeerGenesisAck, genesisRosterPlayers, genesisSeedCommitmentFor, localGenesisAck } from "./genesis-binding.js";

export function usePeerLobbyConnections(base, servicesRef) {
  const paymentDisclosures = createPaymentDisclosureJournal(getPeerSessionStorage);
  function paymentDisclosureScope(intent) {
    return { matchId: intent.matchId || currentAuditMatchId(), seq: intent.seq,
      actorIndex: intent.actorIndex ?? intent.actor, prevStateHash: intent.prevStateHash,
      command: intent.command,
      ...(intent.attemptId ? { attemptId: intent.attemptId } : {}),
      ...(intent.preActionPublicCheckpointHash ? { preActionPublicCheckpointHash: intent.preActionPublicCheckpointHash } : {}),
      ...(intent.signature ? { signature: intent.signature } : {}),
    };
  }
  function assertPaymentDisclosureIntent(intent) {
    if (isForfeitCommand(intent.command)) return null;
    return paymentDisclosures.assertCompatible(paymentDisclosureScope(intent));
  }
  function pinnedPaymentDisclosure(intent) {
    return paymentDisclosures.lookup(paymentDisclosureScope(intent));
  }
  function paymentDisclosureTiming(intent, record = null) {
    return {
      intent: cloneMultiplayerPayload(record?.intent || (intent.signature ? intent : null)),
      firstObservedAtMs: Number(record?.firstObservedAtMs || Date.now()),
      observedElapsedAtIntentMs: record?.observedElapsedAtIntentMs ?? null,
      ...(record ? {
        evidence: cloneMultiplayerPayload(record.evidence || null),
        timeoutConfirmation: cloneMultiplayerPayload(record.timeoutConfirmation || null),
      } : {}),
    };
  }
  function pinPaymentDisclosureIntent(intent, material = {}) {
    const record = pendingActionIntentsRef.current.get(actionIntentKey(intent));
    return paymentDisclosures.pin(paymentDisclosureScope(intent), {
      ...material, timing: material.timing || paymentDisclosureTiming(intent, record),
    });
  }
  function persistPendingPaymentTiming(record) {
    if (record?.intent && pinnedPaymentDisclosure(record.intent)) {
      pinPaymentDisclosureIntent(record.intent, { timing: paymentDisclosureTiming(record.intent, record) });
    }
  }
  function acceptPaymentDisclosure(matchId, seq) {
    // Acceptance already advanced the durable transcript. A stale old pin is
    // harmless; a cleanup/storage error must never undo that accepted action.
    try { paymentDisclosures.accepted(matchId, seq); return true; }
    catch (error) { recordPeerSyncPerf("payment_disclosure:cleanup_deferred", { seq, error: toErrorMessage(error) }); return false; }
  }
  async function validatePaymentDisclosureAuthority(intent) {
    const state = await gameRef.current.uiState();
    assertPaymentDisclosureAuthority(intent, {
      matchId: currentAuditMatchId(),
      lastAppliedSequence: Number(base.actionHistoryRef.current.at(-1)?.seq || 0),
      prevStateHash: base.auditStateHashRef.current,
      decisionPlayer: state?.decision?.player,
    });
  }
  async function pinBlindExileOpeningIntent(intent) {
    if (intent?.command?.action_ref?.kind !== "open_exiled_card_for_play") return false;
    await validatePaymentDisclosureAuthority(intent);
    await verifyCurrentPublicCheckpointHash(intent.preActionPublicCheckpointHash,
      "Blind exile opening does not match the accepted public checkpoint");
    await verifySignedActionIntent(intent, { ...paymentDisclosureScope(intent),
      preActionPublicCheckpointHash: intent.preActionPublicCheckpointHash });
    const localCommand = await servicesRef.current.remapCommandForLocalHiddenOpening(intent.command, [], intent.actorIndex);
    const disclosure = await paymentDisclosureForCommand(localCommand);
    if (!disclosure?.required && !disclosure?.active) throw new Error("Blind exile opening lacks native disclosure authority");
    // Reveal shares can unlock the face before a completed opening is sent.
    // An empty-material pin retains the original signed attempt and deadline.
    pinPaymentDisclosureIntent(intent, { evidence: { actionIntent: intent } });
    await gameRef.current.retainPaymentDisclosure(localCommand);
    return true;
  }
  async function pinVerifiedPaymentEnvelope(intent, openings = [], evidence = null) {
    if (!openings.length || isNonDispatchSyncCommand(intent.command)) return false;
    await validatePaymentDisclosureAuthority(intent);
    if (intent.command?.action_ref?.kind === "open_exiled_card_for_play") {
      const localCommand = await servicesRef.current.remapCommandForLocalHiddenOpening(intent.command, [], intent.actorIndex);
      const requirements = await servicesRef.current.previewRequirementsForCommand(localCommand);
      servicesRef.current.assertAuditOpeningsExpected({ openings, requirements });
      await servicesRef.current.verifyAuditSatisfiesCryptoRequirements({ requirements,
        audit: { openings }, allowCachedPublicOpenings: false });
    }
    const snapshot = await servicesRef.current.createSequencedActionValidationSnapshot();
    try {
      await servicesRef.current.verifyAuditOpeningsAgainstManifests(openings, {
        payload: matchStartPayloadRef.current, shuffleProofs: evidence?.audit?.shuffleProofs || [],
      });
      await servicesRef.current.revealAuditOpenings(openings, { timing: "pre", command: intent.command,
        shuffleProofs: evidence?.audit?.shuffleProofs || [], updateState: false });
      const localCommand = await servicesRef.current.remapCommandForLocalHiddenOpening(intent.command, openings, intent.actorIndex);
      const disclosure = await paymentDisclosureForCommand(localCommand);
      if (!disclosure?.required && !disclosure?.active) return false;
      pinPaymentDisclosureIntent(intent, { openings, evidence });
      return true;
    } finally {
      // If pinned, this recovery path reopens the same verified disclosure
      // after restoring the canonical prefix. Otherwise it is a pure probe.
      try { await servicesRef.current.restoreSequencedActionValidationSnapshot(snapshot); }
      finally { await snapshot.release?.(); }
    }
  }
  async function restorePaymentDisclosureAtHead({ sequence, prevStateHash } = {}) {
    const matchId = currentAuditMatchId();
    if (!matchId) return null;
    const entries = paymentDisclosures.entries(matchId).filter(entry =>
      entry.seq === Number(sequence) && entry.prevStateHash === String(prevStateHash || ""));
    if (!entries.length) return null;
    if (entries.length !== 1) throw new Error("Conflicting payment disclosure recovery records");
    const retained = entries[0];
    await validatePaymentDisclosureAuthority(retained);
    const canonicalIntent = retained.signedIntent || retained.evidence?.actionIntent || retained.timing?.intent;
    if (canonicalIntent) {
      await verifySignedActionIntent(canonicalIntent, retained);
    } else if (retained.evidence?.audit) {
      await servicesRef.current.verifySequencedActionAudit({ audit: retained.evidence.audit,
        seq: retained.seq, actorIndex: retained.actorIndex, command: retained.command });
    } else { throw new Error("Disclosed payment recovery lacks signed evidence"); }
    await servicesRef.current.verifyAuditOpeningsAgainstManifests(retained.openings || [], {
      payload: matchStartPayloadRef.current, shuffleProofs: retained.evidence?.audit?.shuffleProofs || [],
    });
    await servicesRef.current.revealAuditOpenings(retained.openings || [], { timing: "pre",
      command: retained.command, shuffleProofs: retained.evidence?.audit?.shuffleProofs || [], updateState: false });
    const localCommand = await servicesRef.current.remapCommandForLocalHiddenOpening(
      retained.command, retained.openings || [], retained.actorIndex);
    if (typeof gameRef.current?.retainPaymentDisclosure !== "function") {
      throw new Error("Engine cannot retain the disclosed payment; replay the accepted prefix with the current engine");
    }
    const state = await gameRef.current.retainPaymentDisclosure(localCommand);
    const pendingIntent = canonicalIntent;
    if (pendingIntent) {
      await verifySignedActionIntent(pendingIntent, retained);
      await rememberPendingActionIntent(pendingIntent, retained.timing?.evidence || {});
    }
    return state;
  }

  async function paymentDisclosureForCommand(command) {
    // Cancel/Undo is not a UiCommand. Native cancelDecision still checks the
    // active commitment, and assertPaymentDisclosureIntent checks a pinned retry.
    if (isNonDispatchSyncCommand(command)) return { required: false, active: false, objects: [] };
    if (typeof gameRef.current?.getPaymentDisclosureForCommand !== "function") {
      throw new Error("Engine lacks the disclosure-safe payment boundary; refresh before announcing this action");
    }
    return await gameRef.current.getPaymentDisclosureForCommand(command);
  }

  const { actionIntentOpeningPreviewKeysRef, actionQuorumVoteWaitersRef, actionSubmissionStartedAtMsRef, auditEncryptionKeyPairRef, auditEncryptionPublicKeyRef, auditKeyPairRef, auditPublicKeyRef, auditVerifyKeyCacheRef, connectionHeartbeatsRef, cryptoMaterialWaitersRef, ensureDirectPeerConnectionsRef, gameRef, ignoredActionIntentKeysRef, protocolWaitObservationsRef, liveZiffleCeremoniesRef, localRevealedOpeningsRef, localZiffleCeremonyLookupRef, matchClockConfigRef, matchStartPayloadRef, multiplayerRef, peerHeartbeatConfigRef, pendingActionIntentTimeoutsRef, pendingActionIntentsRef, privateDeckManifestsRef, privateViewDisclosuresRef, rngCommitWaitersRef, rngRevealWaitersRef, setMultiplayer, setStatus, stateRef, submissionIdleWaitersRef, timeoutVoteWaitersRef, ziffleHandRevealKeyRef, ziffleHandRevealQuickKeyRef, ziffleKeyPairsRef, ziffleOpeningPositionsRef, ziffleRevealTokenCacheRef, ziffleRevealWaitersRef, ziffleShuffleWaitersRef } = base;
  const actionHistoryEntryForSequence = useCallback((...args) => servicesRef.current.actionHistoryEntryForSequence(...args), [servicesRef]);
  const applySequencedActionMessage = useCallback((...args) => servicesRef.current.applySequencedActionMessage(...args), [servicesRef]);
  const collectZiffleRevealTokens = useCallback((...args) => servicesRef.current.collectZiffleRevealTokens(...args), [servicesRef]);
  const currentZiffleOriginForOpening = useCallback((...args) => servicesRef.current.currentZiffleOriginForOpening(...args), [servicesRef]);
  const playerForProtocolResponseTimeout = useCallback((...args) => servicesRef.current.playerForProtocolResponseTimeout(...args), [servicesRef]);
  const previewAuditOpeningInInspector = useCallback((...args) => servicesRef.current.previewAuditOpeningInInspector(...args), [servicesRef]);
  const resolveCommittedZiffleRevealSlot = useCallback((...args) => servicesRef.current.resolveCommittedZiffleRevealSlot(...args), [servicesRef]);
  const routePeerIdForPlayer = useCallback((...args) => servicesRef.current.routePeerIdForPlayer(...args), [servicesRef]);
  const sendDirectPeerMessage = useCallback((...args) => servicesRef.current.sendDirectPeerMessage(...args), [servicesRef]);
  const submitProtocolResponseTimeoutClaim = useCallback((...args) => servicesRef.current.submitProtocolResponseTimeoutClaim(...args), [servicesRef]);
  const verifyCurrentPublicCheckpointHash = useCallback((...args) => servicesRef.current.verifyCurrentPublicCheckpointHash(...args), [servicesRef]);
  const ensureDirectPeerConnections = useCallback((players) => {
    ensureDirectPeerConnectionsRef.current(players);
  }, []);

  const clearConnectionHeartbeat = useCallback((key) => {
    const heartbeat = connectionHeartbeatsRef.current.get(key);
    if (!heartbeat) return;
    window.clearInterval(heartbeat.timer);
    connectionHeartbeatsRef.current.delete(key);
  }, []);

  const clearAllConnectionHeartbeats = useCallback(() => {
    for (const heartbeat of connectionHeartbeatsRef.current.values()) {
      window.clearInterval(heartbeat.timer);
    }
    connectionHeartbeatsRef.current.clear();
  }, []);

  const markConnectionAlive = useCallback((key) => {
    const heartbeat = connectionHeartbeatsRef.current.get(key);
    if (heartbeat) {
      heartbeat.lastSeen = Date.now();
      heartbeat.missedTimeouts = 0;
    }
  }, []);

  function pendingActionIntentSuppressesHeartbeatStale(nowMs = Date.now()) {
    if (
      multiplayerRef.current.submittingAction
      && actionSubmissionStartedAtMsRef.current > 0
      && nowMs < actionSubmissionStartedAtMsRef.current + MAX_PENDING_ACTION_INTENT_MS + MATCH_CLOCK_CLAIM_SKEW_MS
    ) {
      return true;
    }
    for (const record of pendingActionIntentsRef.current.values()) {
      if (nowMs < pendingActionIntentDueAtMs(record)) {
        return true;
      }
    }
    return false;
  }

  // Every peer periodically signs "I started genesis H" on its heartbeats, so
  // peers can detect a host that showed seats different rosters/keys.
  function genesisAckHeartbeatField() {
    const payload = matchStartPayloadRef.current;
    if (!payload?.genesis?.payloadHash || !multiplayerRef.current.matchStarted) return {};
    const ack = localGenesisAck(payload, {
      keyPair: auditKeyPairRef.current,
      localSeat: resolveLocalPlayerIndexFromPeer(multiplayerRef.current, payload.players),
      localPeerId: multiplayerRef.current.localPeerId,
    });
    return ack ? { genesisAck: ack } : {};
  }

  const startConnectionHeartbeat = useCallback((key, conn, onStale) => {
    clearConnectionHeartbeat(key);
    const configured = peerHeartbeatConfigRef.current;
    // Leave idle gaps for Durable Object hibernation while retaining end-to-end liveness checks.
    const intervalMs = conn?.owner?.options?.transport === 'websocket' ? 30000 : configured.intervalMs;
    const timeoutMs = conn?.owner?.options?.transport === 'websocket' ? 120000 : configured.timeoutMs;
    if (!intervalMs || !timeoutMs) return;
    const maxMissedTimeouts = 3;

    const heartbeat = {
      lastSeen: Date.now(),
      lastCheck: Date.now(),
      missedTimeouts: 0,
      lastTimeoutLogAt: 0,
      timer: window.setInterval(() => {
        const dataChannelState = String(conn?.dataChannel?.readyState || "").toLowerCase();
        const iceState = String(conn?.peerConnection?.iceConnectionState || "").toLowerCase();
        if (
          !conn
          || conn.open === false
          || dataChannelState === "closed"
          || iceState === "failed"
          || iceState === "closed"
        ) {
          clearConnectionHeartbeat(key);
          onStale?.("Connection closed");
          return;
        }

        const nowMs = Date.now();
        const localSchedulerDelayMs = nowMs - heartbeat.lastCheck;
        heartbeat.lastCheck = nowMs;
        // Do not condemn the remote peer immediately after this tab's own
        // event loop was suspended or heavily throttled.  Give the queued
        // heartbeat/data events one full timeout window to arrive.
        if (localSchedulerDelayMs > timeoutMs) {
          recordPeerSyncPerf("peer_heartbeat:local_scheduler_stall", {
            connection: key,
            scheduler_delay_ms: localSchedulerDelayMs,
            timeout_ms: timeoutMs,
          });
          heartbeat.lastSeen = nowMs;
        }
        if (
          nowMs - heartbeat.lastSeen > timeoutMs
          && !pendingActionIntentSuppressesHeartbeatStale(nowMs)
        ) {
          if (nowMs - heartbeat.lastTimeoutLogAt >= timeoutMs) {
            recordPeerSyncPerf("peer_heartbeat:timeout", {
              connection: key,
              silent_ms: nowMs - heartbeat.lastSeen,
              timeout_ms: timeoutMs,
              missed_windows: heartbeat.missedTimeouts + 1,
              connection_open: conn.open !== false,
            });
            heartbeat.lastTimeoutLogAt = nowMs;
          }
          heartbeat.missedTimeouts += 1;
          if (heartbeat.missedTimeouts < maxMissedTimeouts) {
            // Application-level silence is not proof that WebRTC is dead: the
            // browser may be backgrounded or the event loop may be suspended.
            // Require several consecutive windows before recovery is started.
            safeSend(conn, {
              type: "peer_heartbeat",
              protocolVersion: PROTOCOL_VERSION,
              at: nowMs,
              ...genesisAckHeartbeatField(),
            });
            return;
          }
          clearConnectionHeartbeat(key);
          try {
            conn.close();
          } catch {
            // Best effort; the stale callback handles local state cleanup.
          }
          onStale?.("Peer heartbeat timed out");
          return;
        }

        safeSend(conn, {
          type: "peer_heartbeat",
          protocolVersion: PROTOCOL_VERSION,
          at: nowMs,
          ...genesisAckHeartbeatField(),
        });
      }, intervalMs),
    };
    connectionHeartbeatsRef.current.set(key, heartbeat);
  }, [clearConnectionHeartbeat]);

  const handleConnectionHeartbeatMessage = useCallback((conn, message) => {
    if (message?.type === "peer_heartbeat_ack") {
      // The ack echoes our own send timestamp, so the difference is a round trip.
      const sentAtMs = Number(message.at);
      if (Number.isFinite(sentAtMs) && sentAtMs > 0) recordPeerRtt(conn?.peer, Date.now() - sentAtMs);
      return message.protocolVersion === PROTOCOL_VERSION;
    }
    if (message?.type !== "peer_heartbeat") return false;
    if (message.protocolVersion !== PROTOCOL_VERSION) return false;
    if (message.genesisAck && matchStartPayloadRef.current?.genesis?.payloadHash) {
      void checkPeerGenesisAck(conn, message.genesisAck, {
        payload: matchStartPayloadRef.current,
        onViolation: (reason, seat) => {
          const body = `Cheat detected: ${reason}`;
          emitSyncFailureNotice("Cheat detected", body);
          servicesRef.current.markMatchDisputed?.(body, {
            type: "genesis_ack_mismatch_v1",
            seat,
            genesisPayloadHash: String(matchStartPayloadRef.current?.genesis?.payloadHash || ""),
            accusedPlayers: [],
          });
        },
      }).catch(() => {});
    }
    safeSend(conn, {
      type: "peer_heartbeat_ack",
      protocolVersion: PROTOCOL_VERSION,
      at: message.at ?? Date.now(),
    });
    return true;
  }, []);

  const resolveSubmissionIdleWaiters = useCallback(() => {
    if (multiplayerRef.current.submittingAction) return;
    const waiters = submissionIdleWaitersRef.current;
    submissionIdleWaitersRef.current = [];
    for (const waiter of waiters) {
      if (waiter.timeoutId) {
        globalThis.clearTimeout(waiter.timeoutId);
      }
      waiter.resolve(true);
    }
  }, []);

  const waitForSubmissionIdle = useCallback((timeoutMs = ACTION_SUBMISSION_IDLE_WAIT_MS) => {
    if (!multiplayerRef.current.submittingAction) {
      return Promise.resolve(true);
    }
    return new Promise((resolve) => {
      const waiter = {
        resolve,
        timeoutId: null,
      };
      waiter.timeoutId = globalThis.setTimeout(() => {
        submissionIdleWaitersRef.current = submissionIdleWaitersRef.current.filter(
          (entry) => entry !== waiter
        );
        resolve(false);
      }, Math.max(0, Number(timeoutMs || 0)));
      submissionIdleWaitersRef.current.push(waiter);
    });
  }, []);

  const updateMultiplayer = useCallback((updater) => {
    const previous = multiplayerRef.current;
    const next =
      typeof updater === "function" ? updater(previous) : updater;
    if (next === previous) {
      return previous;
    }
    const normalized = withConnectionWarnings(next);
    if (normalized.submittingAction && !actionSubmissionStartedAtMsRef.current) {
      actionSubmissionStartedAtMsRef.current = Date.now();
    }
    if (!normalized.submittingAction) {
      actionSubmissionStartedAtMsRef.current = 0;
    }
    multiplayerRef.current = normalized;
    if (Number(previous.lastAppliedSequence || 0) !== Number(normalized.lastAppliedSequence || 0)) {
      servicesRef.current.notifyProtocolActionHead?.();
    }
    try { saveRelayLobby(normalized, previous); } catch (error) {
      setStatus(`Could not save reconnect identity. Keep this tab open: ${toErrorMessage(error)}`, true);
    }
    base.matchClockStore?.current.set(normalized.matchClock || normalized.actionTimer || null);
    // Clock epochs live in a separate subscription; board consumers only see
    // semantic session changes. Check the raw next object before warning shaping.
    if (differsBeyondClock(previous, next)) setMultiplayer(normalized);
    if (!normalized.submittingAction) {
      resolveSubmissionIdleWaiters();
    }
    return normalized;
  }, [resolveSubmissionIdleWaiters, setMultiplayer]);

  const beginPeerWait = useCallback((wait = {}) => {
    const requestId = String(
      wait.requestId || `peer-wait:${Date.now().toString(36)}:${randomAuditHex(8)}`
    );
    const peerWait = {
      kind: String(wait.kind || "peer_response"),
      requestId,
      title: String(wait.title || "Waiting for peers"),
      description: String(wait.description || ""),
      peerIndex: wait.peerIndex == null ? null : Number(wait.peerIndex),
      peerName: wait.peerName == null ? "" : String(wait.peerName),
      peers: Array.isArray(wait.peers) ? cloneMultiplayerPayload(wait.peers) : [],
      detail: wait.detail == null ? "" : String(wait.detail),
      operation: wait.operation == null ? "" : String(wait.operation),
      phase: wait.phase == null ? "" : String(wait.phase),
      cardName: wait.cardName == null ? "" : String(wait.cardName),
      zone: wait.zone == null ? "" : String(wait.zone),
      actionIntentKey: wait.actionIntentKey == null ? "" : String(wait.actionIntentKey),
      progressCurrent: Number.isFinite(Number(wait.progressCurrent))
        ? Number(wait.progressCurrent)
        : null,
      progressTotal: Number.isFinite(Number(wait.progressTotal))
        ? Number(wait.progressTotal)
        : null,
      responseTimeoutMs: Number.isFinite(Number(wait.responseTimeoutMs))
        ? Math.max(1, Math.floor(Number(wait.responseTimeoutMs)))
        : null,
      openingPreviews: mergeActionOpeningPreviews(
        [],
        [
          ...(Array.isArray(wait.openingPreviews) ? wait.openingPreviews : []),
          wait.openingPreview || wait.opening_preview,
        ]
      ),
      local: Boolean(wait.local),
      startedAtMs: Date.now(),
    };
    updateMultiplayer((prev) => ({ ...prev, peerWait }));
    return requestId;
  }, [updateMultiplayer]);

  const updatePeerWait = useCallback((requestId = null, patch = {}) => {
    updateMultiplayer((prev) => {
      if (!prev.peerWait) return prev;
      if (requestId && String(prev.peerWait.requestId || "") !== String(requestId)) {
        return prev;
      }
      const nextPatch = { ...patch };
      if (Object.prototype.hasOwnProperty.call(nextPatch, "progressCurrent")) {
        nextPatch.progressCurrent = Number.isFinite(Number(nextPatch.progressCurrent))
          ? Number(nextPatch.progressCurrent)
          : null;
      }
      if (Object.prototype.hasOwnProperty.call(nextPatch, "progressTotal")) {
        nextPatch.progressTotal = Number.isFinite(Number(nextPatch.progressTotal))
          ? Number(nextPatch.progressTotal)
          : null;
      }
      if (Object.prototype.hasOwnProperty.call(nextPatch, "responseTimeoutMs")) {
        nextPatch.responseTimeoutMs = Number.isFinite(Number(nextPatch.responseTimeoutMs))
          ? Math.max(1, Math.floor(Number(nextPatch.responseTimeoutMs)))
          : null;
      }
      const nextOpeningPreview = normalizeActionOpeningPreview(
        nextPatch.openingPreview || nextPatch.opening_preview
      );
      if (nextOpeningPreview) {
        nextPatch.openingPreviews = mergeActionOpeningPreviews(
          prev.peerWait.openingPreviews || [],
          [nextOpeningPreview]
        );
      } else if (Object.prototype.hasOwnProperty.call(nextPatch, "openingPreviews")) {
        nextPatch.openingPreviews = mergeActionOpeningPreviews([], nextPatch.openingPreviews || []);
      }
      delete nextPatch.opening_preview;
      return {
        ...prev,
        peerWait: {
          ...prev.peerWait,
          ...nextPatch,
        },
      };
    });
  }, [updateMultiplayer]);

  const clearPeerWait = useCallback((requestId = null) => {
    updateMultiplayer((prev) => {
      if (!prev.peerWait) return prev;
      if (requestId && String(prev.peerWait.requestId || "") !== String(requestId)) {
        return prev;
      }
      return { ...prev, peerWait: null };
    });
  }, [updateMultiplayer]);

  function clearPeerWaitForActionIntent(actionIntentKeyValue) {
    const normalizedKey = String(actionIntentKeyValue || "");
    if (!normalizedKey) return;
    updateMultiplayer((prev) => {
      if (String(prev.peerWait?.actionIntentKey || "") !== normalizedKey) return prev;
      return { ...prev, peerWait: null };
    });
  }

  function updatePeerWaitForActionIntent(actionIntentKeyValue, patch = {}) {
    const normalizedKey = String(actionIntentKeyValue || "");
    if (!normalizedKey) return false;
    let updated = false;
    updateMultiplayer((prev) => {
      if (String(prev.peerWait?.actionIntentKey || "") !== normalizedKey) return prev;
      const nextPatch = { ...patch };
      const nextOpeningPreview = normalizeActionOpeningPreview(
        nextPatch.openingPreview || nextPatch.opening_preview
      );
      if (nextOpeningPreview) {
        nextPatch.openingPreviews = mergeActionOpeningPreviews(
          prev.peerWait.openingPreviews || [],
          [nextOpeningPreview]
        );
      } else if (Object.prototype.hasOwnProperty.call(nextPatch, "openingPreviews")) {
        nextPatch.openingPreviews = mergeActionOpeningPreviews([], nextPatch.openingPreviews || []);
      }
      delete nextPatch.opening_preview;
      updated = true;
      return {
        ...prev,
        peerWait: {
          ...prev.peerWait,
          ...nextPatch,
        },
      };
    });
    return updated;
  }

  const ensureAuditIdentity = useCallback(async () => {
    let storedIdentity = null;
    if (!auditKeyPairRef.current) {
      storedIdentity = readStoredAuditIdentity();
      if (storedIdentity) {
        try {
          auditKeyPairRef.current = await importAuditKeyPair(storedIdentity);
        } catch (err) {
          void err;
          clearStoredAuditIdentity();
        }
      }
    }
    if (!auditEncryptionKeyPairRef.current) {
      storedIdentity = storedIdentity || readStoredAuditIdentity();
      if (storedIdentity) {
        try {
          auditEncryptionKeyPairRef.current = await importAuditEncryptionKeyPair(storedIdentity);
        } catch (err) {
          void err;
        }
      }
    }
    if (!auditKeyPairRef.current) {
      auditKeyPairRef.current = await createAuditSessionKey();
    }
    if (!auditEncryptionKeyPairRef.current) {
      auditEncryptionKeyPairRef.current = await createAuditEncryptionKey();
    }
    writeStoredAuditIdentity({
      ...(await exportAuditKeyPair(auditKeyPairRef.current)),
      ...(await exportAuditEncryptionKeyPair(auditEncryptionKeyPairRef.current)),
    });
    if (!auditPublicKeyRef.current) {
      auditPublicKeyRef.current = await exportAuditPublicKey(auditKeyPairRef.current);
    }
    if (!auditEncryptionPublicKeyRef.current) {
      auditEncryptionPublicKeyRef.current = await exportAuditEncryptionPublicKey(
        auditEncryptionKeyPairRef.current
      );
    }
    return {
      keyPair: auditKeyPairRef.current,
      encryptionKeyPair: auditEncryptionKeyPairRef.current,
      publicKey: auditPublicKeyRef.current,
      encryptionPublicKey: auditEncryptionPublicKeyRef.current,
    };
  }, []);

  const signPlayerGenesis = useCallback(async ({ matchId, player }) => {
    const { keyPair } = await ensureAuditIdentity();
    return buildSignedPlayerGenesis({
      keyPair,
      matchId,
      protocolVersion: PROTOCOL_VERSION,
      timeoutMs: matchClockConfigRef.current.initialMs,
      player,
      // Commits this seat's secret match-seed nonce (revealed at match start).
      seedCommitment: await genesisSeedCommitmentFor(matchId, player?.index ?? player?.seat ?? 0),
    });
  }, [ensureAuditIdentity]);

	  const ensureZiffleIdentity = useCallback(async ({ context, deckCount = 60 }) => {
	    const normalizedContext = String(context || "").trim() || "match";
	    if (ziffleKeyPairsRef.current.has(normalizedContext)) {
	      return ziffleKeyPairsRef.current.get(normalizedContext);
	    }
	    const stored = readStoredZiffleIdentity(normalizedContext);
	    if (stored?.publicKeyHex && stored?.secretKeyHex) {
	      ziffleKeyPairsRef.current.set(normalizedContext, stored);
	      return stored;
	    }
	    const currentGame = gameRef.current;
	    if (!currentGame || typeof currentGame.ziffleKeygen !== "function") {
	      throw new Error("Ziffle mental-poker backend is not available in the game engine");
    }
    const keyPair = await currentGame.ziffleKeygen({
      deckCount: Number(deckCount || 60),
      context: normalizedContext,
      entropyHex: randomAuditHex(32),
	    });
	    ziffleKeyPairsRef.current.set(normalizedContext, keyPair);
	    writeStoredZiffleIdentity(normalizedContext, keyPair);
	    return keyPair;
	  }, []);

  const publicZiffleKey = useCallback((keyPair, playerIndex) => {
    if (!keyPair) return null;
    return {
      player: Number(playerIndex || 0),
      publicKeyHex: String(keyPair.publicKeyHex || ""),
      ownershipProofHex: String(keyPair.ownershipProofHex || ""),
    };
  }, []);

  const zifflePublicKeysForPlayers = useCallback((players = multiplayerRef.current.players) => {
    return reindexPlayers(players).map((player) => {
      const key = player.ziffleKey || {};
      return {
        player: Number(player.index || 0),
        publicKeyHex: String(key.publicKeyHex || ""),
        ownershipProofHex: String(key.ownershipProofHex || ""),
      };
    });
  }, []);

  const runtimeManifestForZiffleCeremony = useCallback(buildZiffleRuntimeManifest, []);

  const makeZiffleRequestId = useCallback((prefix) => (
    `${prefix}:${Date.now().toString(36)}:${randomAuditHex(8)}`
  ), []);

  const waitForZiffleShuffleStep = useCallback((requestId, timeoutMs = 60000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "ziffle_shuffle",
        requestId,
        title: "Waiting for shuffle material",
        description: "A peer is producing verifiable shuffle material before the game can continue.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        ziffleShuffleWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for ziffle shuffle step"));
      }, timeoutMs);
      ziffleShuffleWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForZiffleRevealToken = useCallback((requestId, timeoutMs = 60000, metadata = null, wait = {}) => (
    new Promise((resolve, reject) => {
      const normalizedTimeoutMs = Math.max(1, Math.floor(Number(timeoutMs || 1)));
      const startedAtMs = Date.now();
      const hardDueAtMs = startedAtMs + Math.max(normalizedTimeoutMs, MAX_PENDING_ACTION_INTENT_MS);
      let dueAtMs = startedAtMs + normalizedTimeoutMs;
      let timer = null;
      const scheduleTimeout = () => {
        if (timer) window.clearTimeout(timer);
        timer = window.setTimeout(() => {
          ziffleRevealWaitersRef.current.delete(requestId);
          clearPeerWait(requestId);
          const err = new Error(
            metadata
              ? `Timed out waiting for ziffle reveal token: ${compactZiffleDiagnosticsJson(metadata)}`
              : "Timed out waiting for ziffle reveal token"
          );
          err.protocolResponseTimeoutTiming = {
            responseTimeoutMs: normalizedTimeoutMs,
            requestedAtMs: Math.max(1, dueAtMs - normalizedTimeoutMs),
          };
          reject(err);
        }, Math.max(1, Math.ceil(dueAtMs - Date.now())));
      };
      beginPeerWait({
        kind: "ziffle_reveal",
        requestId,
        title: "Waiting for reveal material",
        description: "A peer is sending cryptographic reveal material before this hidden card can open locally.",
        actionIntentKey: metadata?.actionIntentKey || "",
        ...wait,
      });
      scheduleTimeout();
      ziffleRevealWaitersRef.current.set(requestId, {
        metadata,
        actionIntentKey: String(metadata?.actionIntentKey || ""),
        extendTimeout: (additionalMs = normalizedTimeoutMs) => {
          const requestedExtension = Math.max(1, Math.floor(Number(additionalMs || normalizedTimeoutMs)));
          const nextDueAtMs = Math.min(Date.now() + requestedExtension, hardDueAtMs);
          if (nextDueAtMs <= dueAtMs) return false;
          dueAtMs = nextDueAtMs;
          scheduleTimeout();
          return true;
        },
        resolve: (value) => {
          if (timer) window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          if (timer) window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForRngCommit = useCallback((requestId, timeoutMs = 60000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "fair_random_commit",
        requestId,
        title: "Waiting for random commitment",
        description: "A peer must commit to their random contribution before the shared random value can be revealed.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        rngCommitWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for random commitment"));
      }, timeoutMs);
      rngCommitWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForRngReveal = useCallback((requestId, timeoutMs = 60000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "fair_random_reveal",
        requestId,
        title: "Waiting for random reveal",
        description: "A peer must reveal their committed random contribution before the shared random value can be used.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        rngRevealWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for random reveal"));
      }, timeoutMs);
      rngRevealWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForTimeoutVote = useCallback((requestId, timeoutMs = 15000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "timeout_vote",
        requestId,
        title: "Waiting for peer vote",
        description: "A peer must sign the timeout vote before this claim can be submitted.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        timeoutVoteWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for timeout vote"));
      }, timeoutMs);
      timeoutVoteWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForActionQuorumVote = useCallback((requestId, timeoutMs = 30000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "action_quorum",
        requestId,
        title: "Waiting for action quorum",
        description: "Peers are validating and signing this action before it can be accepted.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        actionQuorumVoteWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for action quorum vote"));
      }, timeoutMs);
      actionQuorumVoteWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  const waitForCryptoMaterial = useCallback((requestId, timeoutMs = 60000, wait = {}) => (
    new Promise((resolve, reject) => {
      beginPeerWait({
        kind: "crypto_material",
        requestId,
        title: "Waiting for cryptographic material",
        description: "A peer must send hidden-card opening material before this action can advance.",
        ...wait,
      });
      const timer = window.setTimeout(() => {
        cryptoMaterialWaitersRef.current.delete(requestId);
        clearPeerWait(requestId);
        reject(new Error("Timed out waiting for cryptographic opening material"));
      }, timeoutMs);
      cryptoMaterialWaitersRef.current.set(requestId, {
        resolve: (value) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          resolve(value);
        },
        reject: (err) => {
          window.clearTimeout(timer);
          clearPeerWait(requestId);
          reject(err);
        },
      });
    })
  ), [beginPeerWait, clearPeerWait]);

  async function makeProtocolResponseTimeoutError(cause, claim = {}) {
    if (!multiplayerRef.current.matchStarted) return cause;
    const targetPlayerIndex = normalizePlayerIndex(claim.targetPlayerIndex);
    if (targetPlayerIndex == null) return cause;
    const players = reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || []);
    const target = players.find((player) => Number(player.index) === Number(targetPlayerIndex));
    const responseTimeoutMs = Math.max(
      1,
      Math.floor(Number(claim.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS))
    );
    const requestedAtMs = Math.max(
      1,
      Math.floor(Number(claim.requestedAtMs || Date.now() - responseTimeoutMs))
    );
    const requestPayload = cloneMultiplayerPayload(claim.requestPayload || {});
    const requestPayloadHash = String(
      claim.requestPayloadHash
      || await sha256Hex(canonicalMultiplayerPayload(requestPayload))
    );
    const targetLabel = target?.name || `Player ${targetPlayerIndex + 1}`;
    const err = new Error(
      `${targetLabel} did not respond to ${String(claim.requestType || "protocol request")} `
      + `within ${Math.ceil(responseTimeoutMs / 1000)}s`
    );
    err.cause = cause;
    err.protocolResponseTimeoutClaim = {
      matchId: currentAuditMatchId(),
      basisSequence: Number(claim.basisSequence ?? multiplayerRef.current.lastAppliedSequence ?? 0),
      targetPlayerIndex,
      targetPeerId: String(claim.targetPeerId || target?.peerId || ""),
      targetName: targetLabel,
      requesterIndex: normalizePlayerIndex(claim.requesterIndex)
        ?? resolveLocalPlayerIndex(multiplayerRef.current),
      requestType: String(claim.requestType || ""),
      requestId: String(claim.requestId || ""),
      requestPayloadHash,
      requestPayload,
      responseTimeoutMs,
      requestedAtMs,
      eligibleAtMs: requestedAtMs + responseTimeoutMs,
    };
    return err;
  }

  async function waitForProtocolResponse(waiter, claim) {
    // Publish the wait so every peer times it from its own observation, the
    // responder can be fed the request by any peer, and the active player's
    // match clock pauses while it waits on another seat.
    const localWait = openLocalProtocolWait(claim).catch(() => null);
    try {
      const value = await waiter;
      void localWait.then((entry) => closeLocalProtocolWait(entry, "answered"));
      return value;
    } catch (err) {
      if (!isProtocolResponseWaitTimeout(err)) {
        void localWait.then((entry) => closeLocalProtocolWait(entry, "error"));
        throw err;
      }
      const timing = err?.protocolResponseTimeoutTiming || {};
      throw await makeProtocolResponseTimeoutError(err, {
        ...claim,
        ...(timing.responseTimeoutMs != null ? { responseTimeoutMs: timing.responseTimeoutMs } : {}),
        ...(timing.requestedAtMs != null ? { requestedAtMs: timing.requestedAtMs } : {}),
      });
    }
  }

  // ---------------------------------------------------------------------------
  // Protocol wait observations.
  //
  // A protocol-response-timeout forfeit may only be signed from LOCAL
  // knowledge. The requester broadcasts a signed notice (with the request
  // itself) when it starts waiting on a seat; every other peer records its own
  // observation time and forwards the request to the target, so a requester
  // cannot claim a timeout for a request the target never saw. The target
  // broadcasts a signed answer notice when it responds, which cancels the
  // claim everywhere. Voters measure elapsed time from their own observation,
  // never from the requester's requestedAtMs.
  // ---------------------------------------------------------------------------
  const PROTOCOL_WAIT_NOTICE_DOMAIN = "ironsmith-protocol-wait-notice-v1";
  const PROTOCOL_WAIT_ANSWER_DOMAIN = "ironsmith-protocol-wait-answer-v1";
  const PROTOCOL_WAIT_MAX_OBSERVATIONS = 512;
  const PROTOCOL_WAIT_MAX_FORWARD_BYTES = 256 * 1024;
  const PROTOCOL_WAIT_NOTICE_DISPATCH_GRACE_MS = 3000;
  const PROTOCOL_WAIT_VOTE_DEFER_MAX_MS = 10000;
  // Upper bound on the timeout a requester may declare for one wait; the
  // largest legitimate request is a full-deck reveal-token batch.
  const PROTOCOL_WAIT_MAX_RESPONSE_TIMEOUT_MS = Math.max(
    PROTOCOL_RESPONSE_TIMEOUT_MS,
    MAX_ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS
  );
  const PROTOCOL_WAIT_REQUEST_ANSWERERS = {
    crypto_material_request: "answerCryptoMaterialRequest",
    ziffle_reveal_token_request: "answerZiffleRevealTokenRequest",
    ziffle_shuffle_step_request: "answerZiffleShuffleStepRequest",
    rng_commit_request: "answerRngCommitRequest",
    rng_reveal_request: "answerRngRevealRequest",
    action_quorum_vote_request: "answerActionQuorumVoteRequest",
  };

  function protocolWaitSupportsRequestType(requestType) {
    return typeof requestType === "string"
      && Object.hasOwn(PROTOCOL_WAIT_REQUEST_ANSWERERS, requestType);
  }

  // Validate the transport envelope before treating a payload as an
  // answerable request. Type-specific authorization remains with its answerer.
  // Some requests carry match/seat context inside their signed action instead.
  function protocolWaitRequestMatchesNotice(notice, request) {
    return protocolWaitSupportsRequestType(notice.requestType)
      && request != null && typeof request === "object" && !Array.isArray(request)
      && request.type === notice.requestType
      && request.requestId === notice.requestId
      && request.protocolVersion === PROTOCOL_VERSION
      && (request.requesterIndex === undefined || request.requesterIndex === notice.requester)
      && (request.matchId === undefined || request.matchId === notice.matchId);
  }

  function protocolWaitKey(requester, requestId) {
    return `${Number(requester)}:${String(requestId || "")}`;
  }

  function protocolWaitNoticePayload(notice = {}) {
    return {
      domain: PROTOCOL_WAIT_NOTICE_DOMAIN,
      matchId: String(notice.matchId || ""),
      basisSequence: Math.max(0, Math.floor(Number(notice.basisSequence || 0))),
      requester: Number(notice.requester),
      target: Number(notice.target),
      requestType: String(notice.requestType || ""),
      requestId: String(notice.requestId || ""),
      requestPayloadHash: String(notice.requestPayloadHash || ""),
      responseTimeoutMs: Math.max(1, Math.floor(Number(notice.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS))),
    };
  }

  // A signed notice alone is only the requester's claim that it is waiting.
  // It earns clock-pause credit (and shields the requester from intent
  // timeouts) only when the request itself is known to be real: its payload
  // matched the signed hash and was deliverable to the target, or the target
  // answered it (the signed answer proves receipt, which covers requests too
  // large to forward in the notice).
  function protocolWaitIsSubstantiated(entry) {
    if (!entry || entry.placeholder) return false;
    if (!protocolWaitSupportsRequestType(entry.requestType)) return false;
    if (entry.answerStatus) return true;
    return protocolWaitRequestMatchesNotice(entry, entry.requestPayload) && entry.deliverable !== false;
  }

  function protocolWaitCreditTimeoutMs(entry) {
    const declared = Math.floor(Number(entry?.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS));
    return Math.min(
      PROTOCOL_WAIT_MAX_RESPONSE_TIMEOUT_MS,
      Math.max(1, Number.isFinite(declared) ? declared : PROTOCOL_RESPONSE_TIMEOUT_MS)
    );
  }

  // Past its (bounded) deadline an unanswered wait stops counting: the
  // requester should have claimed a protocol timeout against the target.
  function protocolWaitExpired(entry, nowMonoMs = nowMonotonicMs()) {
    if (!entry || entry.answerStatus) return false;
    const observedAt = Number(entry.observedAtMonoMs);
    if (!Number.isFinite(observedAt)) return true;
    return Number(nowMonoMs) >= observedAt + protocolWaitCreditTimeoutMs(entry);
  }

  function protocolWaitAnswerPayload(answer = {}) {
    return {
      domain: PROTOCOL_WAIT_ANSWER_DOMAIN,
      matchId: String(answer.matchId || ""),
      requester: Number(answer.requester),
      responder: Number(answer.responder),
      requestType: String(answer.requestType || ""),
      requestId: String(answer.requestId || ""),
      status: String(answer.status || "answered"),
      responseHash: String(answer.responseHash || ""),
    };
  }

  function pruneProtocolWaitObservations() {
    const map = protocolWaitObservationsRef.current;
    const matchId = currentAuditMatchId();
    for (const [key, entry] of map.entries()) {
      if (String(entry?.matchId || "") !== matchId) map.delete(key);
    }
    while (map.size > PROTOCOL_WAIT_MAX_OBSERVATIONS) {
      map.delete(map.keys().next().value);
    }
  }

  // Merges a verified notice into the local record. The first observation
  // time is kept: a re-sent notice cannot restart (or backdate) the timer.
  function recordProtocolWaitObservation(notice, extra = {}) {
    const map = protocolWaitObservationsRef.current;
    const key = protocolWaitKey(notice.requester, notice.requestId);
    const existing = map.get(key);
    if (existing && !existing.placeholder) {
      // A request ID cannot acquire a payload or answer belonging to a
      // different signed notice, nor restart its original observation time.
      if (canonicalMultiplayerPayload(protocolWaitNoticePayload(existing))
        !== canonicalMultiplayerPayload(notice)) return null;
      if (!existing.requestPayload && extra.requestPayload) existing.requestPayload = extra.requestPayload;
      return existing;
    }
    // An answer seen before the notice only counts if it came from the seat
    // the notice names.
    const earlyAnswerValid = existing?.answerStatus
      && Number(existing.target) === Number(notice.target)
      && existing.requestType === notice.requestType;
    const entry = {
      ...(existing || {}),
      ...notice,
      kind: "request",
      placeholder: false,
      observedAtMs: Date.now(),
      observedAtMonoMs: nowMonotonicMs(),
      answeredAtMs: earlyAnswerValid ? existing.answeredAtMs : null,
      answeredAtMonoMs: earlyAnswerValid ? existing.answeredAtMonoMs : null,
      answerStatus: earlyAnswerValid ? String(existing.answerStatus) : "",
      requestHandled: Boolean(existing?.requestHandled),
      requestPayload: extra.requestPayload || existing?.requestPayload || null,
      local: Boolean(extra.local),
      deliverable: extra.deliverable !== false,
    };
    map.set(key, entry);
    pruneProtocolWaitObservations();
    return entry;
  }

  function protocolWaitPlaceholder(requester, requestId) {
    const map = protocolWaitObservationsRef.current;
    const key = protocolWaitKey(requester, requestId);
    let entry = map.get(key);
    if (!entry) {
      entry = {
        kind: "request",
        placeholder: true,
        matchId: currentAuditMatchId(),
        requester: Number(requester),
        requestId: String(requestId || ""),
        answerStatus: "",
        answeredAtMs: null,
        answeredAtMonoMs: null,
        requestHandled: false,
      };
      map.set(key, entry);
      pruneProtocolWaitObservations();
    }
    return entry;
  }

  function markProtocolWaitAnswered(entry, status = "answered") {
    if (!entry || entry.answerStatus) return;
    entry.answerStatus = String(status || "answered");
    entry.answeredAtMs = Date.now();
    entry.answeredAtMonoMs = nowMonotonicMs();
  }

  function broadcastProtocolWaitMessage(payload, excludeIndices = []) {
    const session = multiplayerRef.current;
    const excluded = new Set(excludeIndices.map(Number));
    let sent = false;
    for (const player of session.players || []) {
      if (excluded.has(Number(player.index))) continue;
      const peerId = routePeerIdForPlayer(player);
      if (!peerId || peerId === session.localPeerId) continue;
      sent = sendDirectPeerMessage(peerId, payload) || sent;
    }
    return sent;
  }

  function protocolWaitPlayer(index) {
    return reindexPlayers(matchStartPayloadRef.current?.players || multiplayerRef.current.players || [])
      .find((player) => Number(player.index) === Number(index)) || null;
  }

  async function openLocalProtocolWait(claim = {}) {
    const session = multiplayerRef.current;
    if (!session.matchStarted) return null;
    const requester = normalizePlayerIndex(claim.requesterIndex) ?? resolveLocalPlayerIndex(session);
    const target = normalizePlayerIndex(claim.targetPlayerIndex);
    const requestId = String(claim.requestId || "");
    if (requester == null || target == null || requester === target || !requestId) return null;
    if (Number(requester) !== Number(resolveLocalPlayerIndex(session))) return null;
    const requestPayload = cloneMultiplayerPayload(claim.requestPayload || {});
    const requestPayloadHash = await sha256Hex(canonicalMultiplayerPayload(requestPayload));
    if (claim.requestPayloadHash && claim.requestPayloadHash !== requestPayloadHash) return null;
    const notice = protocolWaitNoticePayload({
      matchId: currentAuditMatchId(),
      basisSequence: claim.basisSequence ?? session.lastAppliedSequence ?? 0,
      requester,
      target,
      requestType: claim.requestType || requestPayload.type,
      requestId,
      requestPayloadHash,
      responseTimeoutMs: claim.responseTimeoutMs,
    });
    if (!protocolWaitRequestMatchesNotice(notice, requestPayload)) return null;
    // Peers only credit a wait whose payload they received (or that the
    // target answered), so the requester's own view applies the same rule.
    const deliverable = payloadSizeBytes(requestPayload) <= PROTOCOL_WAIT_MAX_FORWARD_BYTES;
    const entry = recordProtocolWaitObservation(notice, { requestPayload, local: true, deliverable });
    if (!entry) return null;
    const { keyPair } = await ensureAuditIdentity();
    const signature = await signAuditPayload(keyPair, notice);
    broadcastProtocolWaitMessage({
      type: "protocol_wait_notice",
      protocolVersion: PROTOCOL_VERSION,
      notice,
      signature,
      ...(deliverable ? { requestPayload } : {}),
    }, [requester]);
    return entry;
  }

  function closeLocalProtocolWait(entry, status) {
    markProtocolWaitAnswered(entry, status);
  }

  async function handleProtocolWaitNoticeMessage(message) {
    const session = multiplayerRef.current;
    if (!session.matchStarted || !message?.notice) return;
    const notice = protocolWaitNoticePayload(message.notice);
    if (canonicalMultiplayerPayload(notice) !== canonicalMultiplayerPayload(message.notice)) return;
    if (!protocolWaitSupportsRequestType(notice.requestType)) return;
    if (!Number.isSafeInteger(notice.basisSequence) || notice.basisSequence < 0
      || !Number.isSafeInteger(notice.responseTimeoutMs) || notice.responseTimeoutMs <= 0) return;
    if (notice.matchId !== currentAuditMatchId()) return;
    if (notice.basisSequence < Number(session.lastAppliedSequence || 0)) return;
    const localIndex = resolveLocalPlayerIndex(session);
    if (localIndex == null || Number(notice.requester) === Number(localIndex)) return;
    if (!protocolWaitPlayer(notice.requester) || !protocolWaitPlayer(notice.target)
      || notice.requester === notice.target || !notice.requestId) return;
    const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(notice.requester));
    if (!await verifyAuditPayload(publicKey, notice, String(message.signature || ""))) {
      throw new Error("Protocol wait notice signature is invalid");
    }
    let requestPayload = null;
    if (message.requestPayload !== undefined) {
      if (!protocolWaitRequestMatchesNotice(notice, message.requestPayload)
        || payloadSizeBytes(message.requestPayload) > PROTOCOL_WAIT_MAX_FORWARD_BYTES) return;
      const hash = await sha256Hex(canonicalMultiplayerPayload(message.requestPayload));
      if (hash !== notice.requestPayloadHash) return;
      requestPayload = cloneMultiplayerPayload(message.requestPayload);
    }
    const entry = recordProtocolWaitObservation(notice, { requestPayload });
    if (!entry) return;
    if (Number(notice.target) === Number(localIndex)) {
      scheduleProtocolRequestFromNotice(entry);
      return;
    }
    // Relay the signed request to the target so a requester cannot keep it
    // from the seat it later accuses.
    if (!message.forwardedBy && entry.requestPayload && !entry.forwarded) {
      entry.forwarded = true;
      const targetPlayer = protocolWaitPlayer(notice.target);
      const routePeerId = targetPlayer ? routePeerIdForPlayer(targetPlayer) : "";
      if (routePeerId && routePeerId !== session.localPeerId) {
        sendDirectPeerMessage(routePeerId, {
          type: "protocol_wait_notice",
          protocolVersion: PROTOCOL_VERSION,
          notice,
          signature: String(message.signature || ""),
          requestPayload: entry.requestPayload,
          forwardedBy: Number(localIndex),
        });
      }
    }
  }

  function scheduleProtocolRequestFromNotice(entry) {
    if (!entry?.requestPayload || entry.requestHandled || entry.answerStatus || entry.dispatchTimer) return;
    entry.dispatchTimer = window.setTimeout(() => {
      entry.dispatchTimer = null;
      if (entry.requestHandled || entry.answerStatus) return;
      void answerProtocolRequestFromNotice(entry).catch(() => {});
    }, PROTOCOL_WAIT_NOTICE_DISPATCH_GRACE_MS);
  }

  // Target: the direct request never arrived, so answer the copy carried by
  // the signed notice. The response still goes only to the requester.
  async function answerProtocolRequestFromNotice(entry) {
    if (!protocolWaitRequestMatchesNotice(entry, entry.requestPayload)) return;
    const answererName = PROTOCOL_WAIT_REQUEST_ANSWERERS[String(entry.requestType || "")];
    const answerer = answererName ? servicesRef.current[answererName] : null;
    if (typeof answerer !== "function") return;
    const requesterPlayer = protocolWaitPlayer(entry.requester);
    const routePeerId = requesterPlayer ? routePeerIdForPlayer(requesterPlayer) : "";
    const requesterPeerId = String(requesterPlayer?.peerId || "");
    if (!routePeerId || !requesterPeerId) return;
    entry.requestHandled = true;
    const conn = {
      peer: requesterPeerId,
      open: true,
      send: (payload) => {
        sendDirectPeerMessage(routePeerId, payload);
        return { bytes: payloadSizeBytes(payload) };
      },
    };
    await answerer(conn, cloneMultiplayerPayload(entry.requestPayload));
  }

  // Responder: wraps the connection a protocol request arrived on so the
  // first response to it is announced (signed) to every peer.
  function protocolResponseConn(conn, request = {}) {
    const requestId = String(request?.requestId || "");
    if (!conn || !requestId || !multiplayerRef.current.matchStarted) return conn;
    if (!protocolWaitSupportsRequestType(request?.type)) return conn;
    // Requests relayed through the host arrive on the host's connection, so
    // prefer the requester seat the request names.
    const requester = normalizePlayerIndex(request?.requesterIndex)
      ?? normalizePlayerIndex(servicesRef.current.playerIndexForPeerId?.(conn.peer));
    if (requester == null || !protocolWaitPlayer(requester)) return conn;
    const entry = protocolWaitPlaceholder(requester, requestId);
    entry.requestHandled = true;
    if (entry.dispatchTimer) {
      window.clearTimeout(entry.dispatchTimer);
      entry.dispatchTimer = null;
    }
    let announced = false;
    const wrapped = Object.create(conn);
    wrapped.send = (payload) => {
      const result = conn.send(payload);
      if (
        !announced
        && String(payload?.requestId || "") === requestId
        && String(payload?.type || "").endsWith("_response")
      ) {
        announced = true;
        void announceProtocolResponse(requester, request, payload).catch(() => {});
      }
      return result;
    };
    return wrapped;
  }

  async function announceProtocolResponse(requester, request, response) {
    const session = multiplayerRef.current;
    const responder = resolveLocalPlayerIndex(session);
    if (responder == null || Number(responder) === Number(requester)) return;
    const answer = protocolWaitAnswerPayload({
      matchId: currentAuditMatchId(),
      requester,
      responder,
      requestType: String(request?.type || ""),
      requestId: String(request?.requestId || ""),
      status: response?.error ? "error" : "answered",
      responseHash: await sha256Hex(canonicalMultiplayerPayload(response || {})),
    });
    const entry = protocolWaitPlaceholder(requester, answer.requestId);
    if (entry.placeholder || (Number(entry.target) === Number(responder)
      && entry.requestType === answer.requestType)) {
      if (entry.placeholder) {
        entry.target = Number(responder);
        entry.requestType = answer.requestType;
      }
      markProtocolWaitAnswered(entry, answer.status);
    }
    const { keyPair } = await ensureAuditIdentity();
    broadcastProtocolWaitMessage({
      type: "protocol_wait_answer",
      protocolVersion: PROTOCOL_VERSION,
      answer,
      signature: await signAuditPayload(keyPair, answer),
    }, [responder]);
  }

  async function handleProtocolWaitAnswerMessage(message) {
    const session = multiplayerRef.current;
    if (!session.matchStarted || !message?.answer) return;
    const answer = protocolWaitAnswerPayload(message.answer);
    if (canonicalMultiplayerPayload(answer) !== canonicalMultiplayerPayload(message.answer)) return;
    if (!protocolWaitSupportsRequestType(answer.requestType)) return;
    if (answer.status !== "answered" && answer.status !== "error") return;
    if (answer.matchId !== currentAuditMatchId() || !answer.requestId) return;
    if (!protocolWaitPlayer(answer.responder) || !protocolWaitPlayer(answer.requester)) return;
    const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(answer.responder));
    if (!await verifyAuditPayload(publicKey, answer, String(message.signature || ""))) {
      throw new Error("Protocol wait answer signature is invalid");
    }
    const entry = protocolWaitPlaceholder(answer.requester, answer.requestId);
    // Only the seat the request was addressed to can close it.
    if (!entry.placeholder && (Number(entry.target) !== Number(answer.responder)
      || entry.requestType !== answer.requestType)) return;
    if (entry.placeholder) {
      entry.target = Number(answer.responder);
      entry.requestType = answer.requestType;
    }
    markProtocolWaitAnswered(entry, answer.status);
  }

  function rememberActionIntentObservation(key, record) {
    if (!key || !record?.intent) return;
    const map = protocolWaitObservationsRef.current;
    const mapKey = `intent:${key}`;
    if (map.get(mapKey)?.record === record) return;
    map.set(mapKey, {
      kind: "intent",
      key,
      matchId: String(record.intent.matchId || currentAuditMatchId()),
      actor: Number(record.intent.actorIndex),
      seq: Number(record.intent.seq || 0),
      record,
      cancelled: false,
    });
    pruneProtocolWaitObservations();
  }

  function markActionIntentObservationCancelled(key, senderIndex) {
    const entry = protocolWaitObservationsRef.current.get(`intent:${key}`);
    if (!entry) return;
    if (senderIndex != null && Number(senderIndex) !== Number(entry.actor)) return;
    entry.cancelled = true;
  }

  // Open waits the given seat has on others at the current transcript head.
  function openProtocolWaitsForRequester(requester, basisSequence = multiplayerRef.current.lastAppliedSequence) {
    const matchId = currentAuditMatchId();
    const out = [];
    for (const entry of protocolWaitObservationsRef.current.values()) {
      if (entry?.kind !== "request" || entry.placeholder || entry.answerStatus) continue;
      if (String(entry.matchId || "") !== matchId) continue;
      if (Number(entry.requester) !== Number(requester)) continue;
      if (Number(entry.basisSequence) !== Number(basisSequence || 0)) continue;
      if (!protocolWaitIsSubstantiated(entry) || protocolWaitExpired(entry)) continue;
      out.push(entry);
    }
    return out;
  }

  // Milliseconds of [sinceMonoMs, nowMonoMs] during which `requester` had an
  // outstanding request to another seat, as observed locally. The match clock
  // pauses for the requester over these intervals (see runtimeMatchClockSnapshot).
  function observedProtocolWaitMs(requester, sinceMonoMs, nowMonoMs = nowMonotonicMs()) {
    if (requester == null || !Number.isFinite(Number(sinceMonoMs))) return 0;
    const since = Number(sinceMonoMs);
    const now = Number(nowMonoMs);
    const matchId = currentAuditMatchId();
    const basis = Number(multiplayerRef.current.lastAppliedSequence || 0);
    const intervals = [];
    for (const entry of protocolWaitObservationsRef.current.values()) {
      if (entry?.kind !== "request" || entry.placeholder) continue;
      if (String(entry.matchId || "") !== matchId) continue;
      if (Number(entry.requester) !== Number(requester) || Number(entry.target) === Number(requester)) continue;
      if (Number(entry.basisSequence) !== basis) continue;
      if (!protocolWaitIsSubstantiated(entry)) continue;
      const observedAt = Number(entry.observedAtMonoMs ?? now);
      const start = Math.max(since, observedAt);
      const end = Math.min(
        now,
        Number(entry.answeredAtMonoMs ?? now),
        observedAt + protocolWaitCreditTimeoutMs(entry)
      );
      if (end > start) intervals.push([start, end]);
    }
    intervals.sort((left, right) => left[0] - right[0]);
    let total = 0;
    let cursor = -Infinity;
    for (const [start, end] of intervals) {
      const from = Math.max(start, cursor);
      if (end > from) total += end - from;
      cursor = Math.max(cursor, end);
    }
    return Math.max(0, Math.floor(total));
  }

  // Voter: throws unless this browser itself observed the timed-out request
  // (or the target's signed action intent) and its own timer has expired.
  async function assertLocalProtocolTimeoutObservation(claim = {}, options = {}) {
    const matchId = currentAuditMatchId();
    const basis = Number(multiplayerRef.current.lastAppliedSequence || 0);
    const forfeitedPlayer = normalizePlayerIndex(claim.forfeitedPlayer);
    const requester = normalizePlayerIndex(claim.requester);
    if (forfeitedPlayer == null || requester == null) {
      throw new Error("Protocol-timeout vote request has invalid seats");
    }
    if (Number(claim.basisSequence) !== basis) {
      throw new Error("Protocol-timeout vote request is not based on the local transcript head");
    }
    const map = protocolWaitObservationsRef.current;
    const direct = map.get(protocolWaitKey(requester, claim.requestId));
    let dueAtMs;
    if (direct && !direct.placeholder) {
      if (
        String(direct.matchId || "") !== matchId
        || Number(direct.target) !== forfeitedPlayer
        || String(direct.requestType || "") !== String(claim.requestType || "")
        || String(direct.requestPayloadHash || "") !== String(claim.requestPayloadHash || "")
        || Number(direct.basisSequence) !== basis
      ) {
        throw new Error("Protocol-timeout claim does not match the locally observed request");
      }
      if (direct.answerStatus) {
        throw new Error("The accused seat answered this protocol request");
      }
      if (!direct.requestPayload && !direct.local) {
        throw new Error("Protocol request was not relayed to the accused seat; cannot attest a timeout");
      }
      dueAtMs = Number(direct.observedAtMs) + Math.max(
        Number(direct.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS),
        Math.floor(Number(claim.responseTimeoutMs || 0))
      );
    } else if (direct?.placeholder && direct.answerStatus) {
      throw new Error("The accused seat answered this protocol request");
    } else {
      const intentKey = String(claim.actionIntentKey || "");
      const observed = intentKey ? map.get(`intent:${intentKey}`) : null;
      if (!observed || String(observed.matchId || "") !== matchId || Number(observed.actor) !== forfeitedPlayer) {
        throw new Error("This peer never observed the timed-out protocol request");
      }
      if (observed.cancelled) {
        throw new Error("The accused seat cancelled the timed-out action intent");
      }
      if (Number(observed.seq) !== basis + 1 || matchingAppliedActionForIntent(observed.record.intent)) {
        throw new Error("The timed-out action intent is not pending at the local transcript head");
      }
      if (openProtocolWaitsForRequester(forfeitedPlayer, basis).length > 0) {
        throw new Error("The accused seat is itself waiting on another seat's protocol response");
      }
      dueAtMs = pendingActionIntentDueAtMs(observed.record);
    }
    if (!Number.isFinite(dueAtMs)) {
      throw new Error("Protocol response timeout has no local deadline");
    }
    const remainingMs = dueAtMs - (Date.now() + MATCH_CLOCK_CLAIM_SKEW_MS);
    if (remainingMs > 0) {
      if (options.deferred || remainingMs > PROTOCOL_WAIT_VOTE_DEFER_MAX_MS) {
        throw new Error("Protocol response timeout has not elapsed locally");
      }
      await sleep(remainingMs + 50);
      return assertLocalProtocolTimeoutObservation(claim, { deferred: true });
    }
  }

  // Target: a reason to reject a protocol-timeout forfeit aimed at this seat.
  function localProtocolTimeoutContradiction(claim = {}) {
    const requester = normalizePlayerIndex(claim.requester);
    const localIndex = resolveLocalPlayerIndex(multiplayerRef.current);
    if (requester == null || localIndex == null) return "";
    const entry = protocolWaitObservationsRef.current.get(protocolWaitKey(requester, claim.requestId));
    if (entry?.answerStatus && (entry.placeholder || Number(entry.target) === Number(localIndex))) {
      return "This seat answered the protocol request named by the timeout claim";
    }
    const waitingOn = openProtocolWaitsForRequester(localIndex)
      .filter((wait) => Number(wait.target) === Number(requester));
    if (waitingOn.length > 0) {
      return "This seat is waiting on the claimant's protocol response";
    }
    if (claim.twoPlayer && !entry && !claim.actionIntentKey) {
      return "This seat never received the protocol request named by the timeout claim";
    }
    return "";
  }

  const resolveZiffleShuffleStep = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = ziffleShuffleWaitersRef.current.get(requestId);
    if (!waiter) return false;
    ziffleShuffleWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve(message.step);
    }
    return true;
  }, []);

  const resolveZiffleRevealToken = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = ziffleRevealWaitersRef.current.get(requestId);
    if (!waiter) return false;
    ziffleRevealWaitersRef.current.delete(requestId);
    if (message.error) {
      const diagnostics = {
        requester: waiter.metadata || null,
        responder: message.diagnostics || null,
      };
      const error = new Error(String(message.error));
      error.ziffleDiagnostics = diagnostics;
      waiter.reject(error);
    } else {
      waiter.resolve(message.tokens || message.token);
    }
    return true;
  }, []);

  const resolveRngCommit = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = rngCommitWaitersRef.current.get(requestId);
    if (!waiter) return false;
    rngCommitWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve(message.commitment);
    }
    return true;
  }, []);

  const resolveRngReveal = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = rngRevealWaitersRef.current.get(requestId);
    if (!waiter) return false;
    rngRevealWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve(message.reveal);
    }
    return true;
  }, []);

  const resolveTimeoutVote = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = timeoutVoteWaitersRef.current.get(requestId);
    if (!waiter) return false;
    timeoutVoteWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve(message.vote);
    }
    return true;
  }, []);

  const resolveActionQuorumVote = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = actionQuorumVoteWaitersRef.current.get(requestId);
    if (!waiter) return false;
    actionQuorumVoteWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve(message.vote);
    }
    return true;
  }, []);

  const resolveCryptoMaterial = useCallback((message) => {
    const requestId = String(message?.requestId || "");
    const waiter = cryptoMaterialWaitersRef.current.get(requestId);
    if (!waiter) return false;
    cryptoMaterialWaitersRef.current.delete(requestId);
    if (message.error) {
      waiter.reject(new Error(String(message.error)));
    } else {
      waiter.resolve({
        openings: message.openings || [],
        privateViewProofs: message.privateViewProofs || [],
      });
    }
    return true;
  }, []);

  const currentAuditMatchId = useCallback(() => {
    const matchPayload = matchStartPayloadRef.current;
    const session = multiplayerRef.current;
    return String(
      matchPayload?.auditMatchId
        || session.lobbyId
        || session.hostPeerId
        || "match"
    );
  }, []);

  const rememberPrivateViewDisclosure = useCallback((disclosure) => {
    if (!disclosure || typeof disclosure !== "object") return;
    const payload = disclosure.payload || disclosure;
    const matchId = String(disclosure.matchId || payload?.matchId || currentAuditMatchId());
    const plaintextHash = String(disclosure.plaintextHash || "");
    const requirementId = String(disclosure.requirementId || payload?.requirementId || "");
    const key = [
      matchId,
      Number(disclosure.seq ?? payload?.seq ?? 0),
      requirementId,
      Number(disclosure.owner ?? payload?.owner ?? -1),
      Number(disclosure.viewer ?? payload?.viewer ?? -1),
      Number(disclosure.objectId ?? payload?.objectId ?? -1),
      plaintextHash,
    ].join(":");
    privateViewDisclosuresRef.current.set(key, cloneMultiplayerPayload({
      ...disclosure,
      matchId,
      type: String(disclosure.type || "private_view_opening_disclosure"),
      payload: cloneMultiplayerPayload(payload),
    }));
  }, [currentAuditMatchId]);

  const resolveLocalCryptoPlayerIndex = useCallback((payload = matchStartPayloadRef.current) => {
    const session = multiplayerRef.current;
    return (
      resolveLocalPlayerIndexFromPeer(session, payload?.players)
      ?? resolveLocalPlayerIndex(session)
    );
  }, []);

	  const rememberPrivateDeckManifest = useCallback((manifest) => {
	    if (!manifest || !Array.isArray(manifest.slotSecrets)) return;
	    const key = `${manifest.matchId}:${Number(manifest.owner)}`;
	    privateDeckManifestsRef.current.set(key, manifest);
	    writeStoredPrivateDeckManifest(manifest);
      preloadPrivateDeckManifestArt(manifest);
	  }, []);

	  const privateDeckManifestForOwner = useCallback((owner, matchId = currentAuditMatchId()) => {
	    const key = `${matchId}:${Number(owner)}`;
	    const normalizedOwner = Number(owner);
	    // The match-start payload's public manifest is the source of truth for
	    // which deck sits at a seat. A locally stored private manifest can claim
	    // the wrong owner (e.g. a guest's provisional join-time manifest built
	    // before seat assignment), so reject any local copy that does not match
	    // the published commitments before trusting its slot secrets.
	    const payload = matchStartPayloadRef.current;
	    const payloadMatches = payload && String(payload.auditMatchId || "") === String(matchId || "");
	    const payloadPlayer = payloadMatches
	      ? reindexPlayers(payload.players || []).find(
	        (entry) => Number(entry.index) === normalizedOwner
	      )
	      : null;
	    const payloadManifests = payloadMatches && Array.isArray(payload.deckAuditManifests)
	      ? payload.deckAuditManifests
	      : [];
	    const sharedManifest = payloadMatches
	      ? publicDeckManifest(
	        payloadManifests.find((entry) => Number(entry?.owner) === normalizedOwner)
	          || payloadPlayer?.deckAuditManifest
	      )
	      : null;
	    const matchesPublishedManifest = (candidate) =>
	      !sharedManifest
	      || (
	        String(candidate?.commitmentRoot || "") === String(sharedManifest.commitmentRoot || "")
	        && String(candidate?.decklistCommitment || "") === String(sharedManifest.decklistCommitment || "")
	      );
	    const cached = privateDeckManifestsRef.current.get(key);
	    if (cached) {
	      if (matchesPublishedManifest(cached)) return cached;
	      privateDeckManifestsRef.current.delete(key);
	    }
	    const stored = readStoredPrivateDeckManifest(matchId, owner);
	    if (stored?.slotSecrets) {
	      if (matchesPublishedManifest(stored)) {
	        privateDeckManifestsRef.current.set(key, stored);
	        preloadPrivateDeckManifestArt(stored);
	        return stored;
	      }
	      // Self-heal: drop the mislabeled manifest so future reads go straight
	      // to the published payload reconstruction.
	      try {
	        getPeerSessionStorage()?.removeItem(privateDeckManifestStorageKey(matchId, owner));
	      } catch {
	        // Ignore storage failures.
	      }
	    }
	    // Open-decklist matches publish every player's slot openings in the
	    // match-start payload, so any seat can reconstruct any owner's manifest.
	    if (payloadMatches) {
	      const slotSecrets = sanitizeDeckSlotOpenings(payloadPlayer?.deckSlotOpenings);
	      if (
	        sharedManifest
	        && Number(sharedManifest.owner) === normalizedOwner
	        && slotSecrets.length > 0
	        && slotSecrets.length === Number(sharedManifest.deckCount || 0)
	          + Number(sharedManifest.sideboardCount || 0)
	      ) {
	        const shared = { ...sharedManifest, slotSecrets };
	        privateDeckManifestsRef.current.set(key, shared);
	        return shared;
	      }
	    }
	    return null;
	  }, [currentAuditMatchId]);

  const rememberZiffleOpeningPosition = useCallback((owner, originalSlot, position) => {
    const normalizedOwner = Number(owner);
    const normalizedSlot = Number(originalSlot);
    const normalizedPosition = Number(position);
    if (
      !Number.isSafeInteger(normalizedOwner)
      || normalizedOwner < 0
      || !Number.isSafeInteger(normalizedSlot)
      || normalizedSlot < 0
      || !Number.isSafeInteger(normalizedPosition)
      || normalizedPosition < 0
    ) {
      return;
    }
    ziffleOpeningPositionsRef.current.set(
      `${normalizedOwner}:${normalizedSlot}`,
      normalizedPosition
    );
  }, []);

  const ziffleOpeningPositionForSlot = useCallback((owner, originalSlot) => {
    const normalizedOwner = Number(owner);
    const normalizedSlot = Number(originalSlot);
    if (
      !Number.isSafeInteger(normalizedOwner)
      || normalizedOwner < 0
      || !Number.isSafeInteger(normalizedSlot)
      || normalizedSlot < 0
    ) {
      return null;
    }
    const position = ziffleOpeningPositionsRef.current.get(`${normalizedOwner}:${normalizedSlot}`);
    return Number.isSafeInteger(position) && position >= 0 ? position : null;
  }, []);

	  const clearOwnerZiffleOpeningCache = useCallback((owner, matchId = currentAuditMatchId(), options = {}) => {
	    const normalizedOwner = Number(owner);
	    if (!Number.isSafeInteger(normalizedOwner)) return;
	    if (normalizedOwner === Number(resolveLocalPlayerIndex(multiplayerRef.current))) {
	      ziffleHandRevealKeyRef.current = "";
	      ziffleHandRevealQuickKeyRef.current = "";
	    }
	    for (const key of [...ziffleOpeningPositionsRef.current.keys()]) {
	      if (key.startsWith(`${normalizedOwner}:`)) {
	        ziffleOpeningPositionsRef.current.delete(key);
      }
    }
    for (const key of [...ziffleRevealTokenCacheRef.current.keys()]) {
      if (!options.preserveCiphertextTokens && key.startsWith(`${normalizedOwner}:`)) {
        ziffleRevealTokenCacheRef.current.delete(key);
      }
	    }
	    const normalizedMatchId = String(matchId || "");
	    const handledIndexKeys = new Set();
	    for (const [key, opening] of [...localRevealedOpeningsRef.current.entries()]) {
	      if (
	        key.startsWith(`${normalizedMatchId}:`)
	        && Number(opening?.owner) === normalizedOwner
	      ) {
	        handledIndexKeys.add(key);
	        if (
	          key.startsWith(`${normalizedMatchId}:object:`)
	          || key.startsWith(`${normalizedMatchId}:owner:${normalizedOwner}:position:`)
	        ) {
	          localRevealedOpeningsRef.current.delete(key);
	          removeStoredRevealedOpening(key);
	        } else {
	          const stripped = stripTransientZifflePositionOpeningFields(opening);
	          localRevealedOpeningsRef.current.set(key, stripped);
	          writeStoredRevealedOpening(key, stripped);
	        }
	      }
	    }
	    // Entries persisted before a refresh/reconnect that were never loaded
	    // back into memory carry the same pre-shuffle positions; purge them too so
	    // readEntry cannot resurrect them.
	    purgeStoredZifflePositionOpeningsForMatchOwner(
	      normalizedMatchId,
	      normalizedOwner,
	      handledIndexKeys
	    );
	  }, [currentAuditMatchId]);

  const rememberLocalRevealedOpening = useCallback((opening, details = {}) => {
    if (!opening || opening.owner == null || opening.slot == null || !opening.card) return;
    const matchId = String(details.matchId || currentAuditMatchId());
    const writeEntry = (indexKey, entry) => {
      localRevealedOpeningsRef.current.set(indexKey, entry);
      writeStoredRevealedOpening(indexKey, entry);
    };
    const entryPositionCommitment = String(
      details.positionCommitment
      || details.publicCommitment
      || details.public_commitment
      || opening.positionCommitment
      || opening.position_commitment
      || opening.publicCommitment
      || opening.public_commitment
      || ""
    );
    const entryPosition = zifflePositionFromCommitment(entryPositionCommitment);
    const entryPublicSlot = Number(opening.publicSlot ?? opening.public_slot);
    const entry = {
      ...cloneMultiplayerPayload(opening),
      matchId,
      objectId:
        details.objectId != null
          ? Number(details.objectId)
          : opening.objectId != null
            ? Number(opening.objectId)
            : null,
      position:
        entryPosition != null
          ? entryPosition
          : details.position != null
            ? Number(details.position)
            : opening.position != null
              ? Number(opening.position)
              : Number.isSafeInteger(entryPublicSlot) && entryPublicSlot >= 0
                ? entryPublicSlot
                : null,
      positionCommitment: entryPositionCommitment,
      ziffleContext: String(details.ziffleContext || ziffleContextFromOpening(opening) || ""),
    };
    if (entry.objectId != null) {
      writeEntry(`${matchId}:object:${entry.objectId}`, entry);
    }
    writeEntry(
      `${matchId}:owner:${Number(entry.owner)}:slot:${Number(entry.slot)}`,
      entry
    );
    if (entry.commitment) {
      writeEntry(
        `${matchId}:owner:${Number(entry.owner)}:commitment:${entry.commitment}`,
        entry
      );
    }
    if (entry.positionCommitment) {
      writeEntry(
        `${matchId}:owner:${Number(entry.owner)}:position:${entry.positionCommitment}`,
        entry
      );
      if (entry.ziffleContext) {
        writeEntry(
          `${matchId}:owner:${Number(entry.owner)}:position:${entry.positionCommitment}:context:${entry.ziffleContext}`,
          entry
        );
      }
    }
  }, [currentAuditMatchId]);

  const localRevealedOpeningForExport = useCallback((exported) => {
    if (!exported || exported.owner == null) return null;
    const matchId = currentAuditMatchId();
    const objectId = exported.object_id ?? exported.objectId;
    const owner = Number(exported.owner);
    const commitment = String(exported.commitment || "");
    const readEntry = (indexKey) => {
      const cached = localRevealedOpeningsRef.current.get(indexKey);
      if (cached) return cached;
      const stored = readStoredRevealedOpening(indexKey);
      if (stored) {
        localRevealedOpeningsRef.current.set(indexKey, stored);
      }
      return stored;
    };
    const candidates = [];
    if (objectId != null) {
      candidates.push(readEntry(`${matchId}:object:${Number(objectId)}`));
    }
    if (commitment) {
      candidates.push(
        readEntry(`${matchId}:owner:${owner}:commitment:${commitment}`),
        readEntry(`${matchId}:owner:${owner}:position:${commitment}`)
      );
    }
    for (const candidate of candidates) {
      if (!candidate) continue;
      if (Number(candidate.owner) !== owner) continue;
      if (exported.card && String(candidate.card || "") !== String(exported.card || "")) continue;
      if (
        commitment
        && String(candidate.commitment || "") !== commitment
        && String(candidate.positionCommitment || "") !== commitment
      ) {
        continue;
      }
      return cloneMultiplayerPayload(candidate);
    }
    return null;
  }, [currentAuditMatchId]);

  const localRevealedOpeningForRequirement = useCallback((requirement) => {
    if (!requirement || requirement.owner == null) return null;
    const matchId = currentAuditMatchId();
    const objectId = requirement.objectId ?? requirement.object_id;
    const requirementObjectId = Number(objectId);
    const owner = Number(requirement.owner);
    const slot = requirement.slot == null ? null : Number(requirement.slot);
    const commitment = String(requirement.commitment || "");
    const positionCommitment = String(
      requirement.positionCommitment
      || requirement.position_commitment
      || requirement.publicCommitment
      || requirement.public_commitment
      || ""
    );
    const slotIsZifflePosition = Boolean(
      ziffleDeckHashFromCommitment(commitment)
      || ziffleDeckHashFromCommitment(positionCommitment)
    );
    const expectedPositionCommitment =
      positionCommitment
      || (ziffleDeckHashFromCommitment(commitment) ? commitment : "");
    const candidateMatchesPosition = (candidate) => {
      if (expectedPositionCommitment) {
        const candidateObjectId = Number(candidate?.objectId ?? candidate?.object_id);
        const objectIdMatches =
          Number.isSafeInteger(requirementObjectId)
          && requirementObjectId > 0
          && Number.isSafeInteger(candidateObjectId)
          && candidateObjectId === requirementObjectId;
        const commitmentMatches =
          commitment
          && String(candidate?.commitment || "") === commitment;
        if (objectIdMatches && commitmentMatches) return true;
        return String(candidate?.positionCommitment || "") === expectedPositionCommitment;
      }
      return !candidate?.positionCommitment;
    };
    const readEntry = (indexKey) => {
      const cached = localRevealedOpeningsRef.current.get(indexKey);
      if (cached) return cached;
      const stored = readStoredRevealedOpening(indexKey);
      if (stored) {
        localRevealedOpeningsRef.current.set(indexKey, stored);
      }
      return stored;
    };
    const candidates = [];
    if (commitment) {
      candidates.push(
        readEntry(`${matchId}:owner:${owner}:commitment:${commitment}`),
        readEntry(`${matchId}:owner:${owner}:position:${commitment}`)
      );
    }
    if (positionCommitment) {
      candidates.push(
        readEntry(`${matchId}:owner:${owner}:position:${positionCommitment}`)
      );
    }
    if (objectId != null) {
      candidates.push(readEntry(`${matchId}:object:${Number(objectId)}`));
    }
    if (slot != null && !slotIsZifflePosition) {
      candidates.push(readEntry(`${matchId}:owner:${owner}:slot:${slot}`));
    }
    for (const candidate of candidates) {
      if (!candidate) continue;
      if (Number(candidate.owner) !== owner) continue;
      if (slot != null && !slotIsZifflePosition && Number(candidate.slot) !== slot) continue;
      if (requirement.card && String(candidate.card || "") !== String(requirement.card || "")) {
        continue;
      }
      if (
        commitment
        && String(candidate.commitment || "") !== commitment
        && String(candidate.positionCommitment || "") !== commitment
      ) {
        continue;
      }
      if (!candidateMatchesPosition(candidate)) continue;
      return cloneMultiplayerPayload(candidate);
    }
    if (slot != null && slotIsZifflePosition) {
      for (const candidate of localRevealedOpeningsRef.current.values()) {
        if (!candidate) continue;
        if (Number(candidate.owner) !== owner) continue;
        if (Number(candidate.position) !== slot) continue;
        if (requirement.card && String(candidate.card || "") !== String(requirement.card || "")) {
          continue;
        }
        if (
          commitment
          && String(candidate.commitment || "") !== commitment
          && String(candidate.positionCommitment || "") !== commitment
        ) {
          continue;
        }
        if (!candidateMatchesPosition(candidate)) continue;
        return cloneMultiplayerPayload(candidate);
      }
    }
    return null;
  }, [currentAuditMatchId]);

		  const localRevealedOpeningForZiffleReveal = useCallback(({
		    owner,
		    ceremony,
	    shuffleOriginalSlot,
	    position,
	    card = "",
	    objectId = null,
	  } = {}) => {
	    const normalizedOwner = Number(owner);
	    const expectedCard = String(card || "");
	    const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
	    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
		    const objectIds = [
		      objectId,
		      beforeOrder[Number(shuffleOriginalSlot)],
		      afterOrder[Number(position)],
	    ]
	      .map((entry) => Number(entry))
	      .filter((entry, index, list) =>
	        Number.isSafeInteger(entry)
	        && entry >= 0
	        && list.indexOf(entry) === index
		      );
		    const matchId = currentAuditMatchId();
		    const normalizedPosition = Number(position);
        const parsedShuffleOriginalSlot = Number(shuffleOriginalSlot);
        const normalizedShuffleOriginalSlot =
          Number.isSafeInteger(parsedShuffleOriginalSlot) && parsedShuffleOriginalSlot >= 0
            ? parsedShuffleOriginalSlot
            : null;
		    const expectedPositionCommitment =
		      ceremony?.deckHash && Number.isSafeInteger(normalizedPosition) && normalizedPosition >= 0
		        ? ziffleRuntimeCommitment(ceremony.deckHash, normalizedPosition)
		        : "";
		    const expectedZiffleContext = ziffleContextFromCeremony(ceremony);
			    const candidateMatchesCurrentPosition = (candidate) => {
			      if (!candidate) return false;
			      const candidateZiffleContext = ziffleContextFromOpening(candidate);
			      if (
			        expectedZiffleContext
			        && candidateZiffleContext
			        && candidateZiffleContext !== expectedZiffleContext
			      ) {
			        return false;
			      }
			      const candidatePositionCommitment = String(candidate.positionCommitment || "");
		      if (
		        candidatePositionCommitment
		        && expectedPositionCommitment
		        && candidatePositionCommitment !== expectedPositionCommitment
		      ) {
		        return false;
		      }
			      if (
			        candidate.position != null
			        && Number(candidate.position) !== normalizedPosition
			      ) {
			        return false;
			      }
			      if (ziffleCeremonyHasObjectOrder(ceremony)) {
			        const hasObjectIdentity = [
			          candidate.shuffleObjectId,
			          candidate.shuffle_object_id,
			          candidate.objectId,
			          candidate.object_id,
			        ].some((value) => {
			          const id = Number(value);
			          return Number.isSafeInteger(id) && id >= 0;
			        });
			        if (
			          ziffleObjectOrderLinksOpening(
			            ceremony,
			            shuffleOriginalSlot,
			            position,
			            candidate
			          )
			        ) {
			          return true;
			        }
			        return !hasObjectIdentity && Boolean(candidatePositionCommitment || candidate.position != null);
			      }
			      if (candidatePositionCommitment || candidate.position != null) return true;
			      return ziffleObjectOrderLinksOpening(
			        ceremony,
			        shuffleOriginalSlot,
		        position,
		        candidate
		      );
		    };
		    const candidates = [];
      const readOpeningEntry = (indexKey) => {
        const cached = localRevealedOpeningsRef.current.get(indexKey);
        if (cached) candidates.push(cached);
        const stored = readStoredRevealedOpening(indexKey);
        if (stored) {
          localRevealedOpeningsRef.current.set(indexKey, stored);
          candidates.push(stored);
        }
      };
	    for (const objectId of objectIds) {
        readOpeningEntry(`${matchId}:object:${objectId}`);
	    }
      if (expectedPositionCommitment) {
        if (expectedZiffleContext) {
          readOpeningEntry(
            `${matchId}:owner:${normalizedOwner}:position:${expectedPositionCommitment}:context:${expectedZiffleContext}`
          );
        }
        readOpeningEntry(`${matchId}:owner:${normalizedOwner}:position:${expectedPositionCommitment}`);
      }
      if (normalizedShuffleOriginalSlot != null) {
        readOpeningEntry(`${matchId}:owner:${normalizedOwner}:slot:${normalizedShuffleOriginalSlot}`);
      }
	    for (const cached of localRevealedOpeningsRef.current.values()) {
	      if (
          objectIds.includes(Number(cached?.objectId))
          || (
            expectedPositionCommitment
            && String(cached?.positionCommitment || "") === expectedPositionCommitment
          )
        ) {
	        candidates.push(cached);
	      }
	    }
		    for (const candidate of candidates) {
		      if (!candidate) continue;
		      if (Number(candidate.owner) !== normalizedOwner) continue;
		      if (expectedCard && String(candidate.card || "") !== expectedCard) continue;
		      if (candidate.slot == null || !candidate.card) continue;
		      if (!candidateMatchesCurrentPosition(candidate)) continue;
		      return cloneMultiplayerPayload(candidate);
		    }
	    return null;
	  }, [currentAuditMatchId]);

		  const publicDeckManifestForOwner = useCallback((owner) => {
    const normalized = Number(owner);
    return publicDeckManifest(
      genesisRosterPlayers(matchStartPayloadRef.current, multiplayerRef.current).find(
        (player) => Number(player.index) === normalized
      )?.deckAuditManifest
    );
  }, []);

  // Keys come only from the signed genesis roster once a Verified match has
  // one; host lobby_state / resync currentPlayers cannot substitute them.
  const publicKeyForAuditSigner = useCallback((signerIndex) => {
    const normalized = normalizePlayerIndex(signerIndex);
    if (normalized == null) return "";
    const player = genesisRosterPlayers(matchStartPayloadRef.current, multiplayerRef.current).find(
      (entry) => Number(entry.index) === normalized
    );
    return String(player?.auditPublicKey || "");
  }, []);

  const auditEncryptionPublicKeyForPlayer = useCallback((playerIndex) => {
    const normalized = normalizePlayerIndex(playerIndex);
    if (normalized == null) return "";
    const player = genesisRosterPlayers(matchStartPayloadRef.current, multiplayerRef.current).find(
      (entry) => Number(entry.index) === normalized
    );
    return String(player?.auditEncryptionPublicKey || "");
  }, []);

  const signedZiffleKeysForPayload = useCallback((matchPayload = null) => {
    const payload = matchPayload || matchStartPayloadRef.current;
    if (Array.isArray(payload?.ziffleKeys) && payload.ziffleKeys.length > 0) {
      return cloneMultiplayerPayload(payload.ziffleKeys);
    }
    return zifflePublicKeysForPlayers(
      reindexPlayers(payload?.players || multiplayerRef.current.players || [])
    );
  }, [zifflePublicKeysForPlayers]);

  const matchPayloadCeremoniesForLookup = useCallback((options = {}) => {
    const payloads = [
      options.payload,
      options.payload === matchStartPayloadRef.current ? null : matchStartPayloadRef.current,
    ].filter(Boolean);
    return payloads.flatMap((payload) =>
      Array.isArray(payload?.ziffleCeremonies) ? payload.ziffleCeremonies : []
    );
  }, []);

  const hydrateZiffleCeremonyForLookup = useCallback((ceremony, options = {}) => {
    if (!ceremony || typeof ceremony !== "object") return ceremony;
	    const owner = Number(ceremony.owner);
	    const deckHash = String(ceremony.deckHash || "");
	    const context = String(ceremony.context || "");
	    const explicitFallback = [
	      ...(Array.isArray(options.ziffleCeremonies) ? options.ziffleCeremonies : []),
	      ...(Array.isArray(options.shuffleProofs) ? options.shuffleProofs : []),
	    ].find((entry) =>
	      Number(entry?.owner) === owner
	      && String(entry?.deckHash || "") === deckHash
	      && String(entry?.context || "") === context
	    );
	    const payloadFallback = matchPayloadCeremoniesForLookup(options).find((entry) =>
	      Number(entry?.owner) === owner
	      && String(entry?.deckHash || "") === deckHash
	      && String(entry?.context || "") === context
	    );
	    const keys = Array.isArray(ceremony.keys) && ceremony.keys.length > 0
	      ? ceremony.keys
	      : (
	        Array.isArray(explicitFallback?.keys) && explicitFallback.keys.length > 0
	          ? explicitFallback.keys
	          : (
	            Array.isArray(payloadFallback?.keys) && payloadFallback.keys.length > 0
	              ? payloadFallback.keys
	              : signedZiffleKeysForPayload(options.payload)
	          )
	      );
	    const steps = Array.isArray(ceremony.steps) && ceremony.steps.length > 0
	      ? ceremony.steps
	      : (
	        Array.isArray(explicitFallback?.steps) && explicitFallback.steps.length > 0
	          ? explicitFallback.steps
	          : (Array.isArray(payloadFallback?.steps) ? payloadFallback.steps : [])
	      );
    return {
      ...ceremony,
      keys: cloneMultiplayerPayload(keys || []),
      steps: cloneMultiplayerPayload(steps || []),
      ...ziffleInputDeckFields(ceremony.inputDeck ? ceremony : explicitFallback || payloadFallback),
    };
  }, [matchPayloadCeremoniesForLookup, signedZiffleKeysForPayload]);

	  const rememberLocalZiffleCeremonyForLookup = useCallback((ceremony) => {
	    if (!ceremony || typeof ceremony !== "object") return;
	    const owner = Number(ceremony.owner);
	    const deckHash = String(ceremony.deckHash || "");
	    const context = String(ceremony.context || "");
	    if (!Number.isInteger(owner) || !deckHash) return;
	    const key = `${owner}:${deckHash}:${context}`;
	    if (localZiffleCeremonyLookupRef.current.has(key)) {
	      localZiffleCeremonyLookupRef.current.delete(key);
	    }
	    localZiffleCeremonyLookupRef.current.set(key, cloneMultiplayerPayload(ceremony));
	  }, []);

	  const ziffleCeremonyCandidatesForOwner = useCallback((owner, options = {}) => {
	    const normalizedOwner = Number(owner);
	    const deckHash = String(options.deckHash || ziffleDeckHashFromCommitment(options.commitment) || "");
	    const context = String(options.context || options.ziffleContext || options.ziffle_context || "");
	    const candidates = [];
	    const seen = new Set();
	    const addCandidate = (entry) => {
	      if (!entry || typeof entry !== "object") return;
	      if (Number(entry.owner) !== normalizedOwner) return;
	      if (deckHash && String(entry.deckHash || "") !== deckHash) return;
	      if (context && String(entry.context || "") !== context) return;
	      const key = [
	        Number(entry.owner),
	        String(entry.deckHash || ""),
	        String(entry.context || ""),
	        normalizeShuffleOrder(entry.beforeOrder ?? entry.before_order).join(","),
	        normalizeShuffleOrder(entry.afterOrder ?? entry.after_order).join(","),
	      ].join(":");
	      if (seen.has(key)) return;
	      seen.add(key);
	      candidates.push(hydrateZiffleCeremonyForLookup(entry, options));
	    };
	    for (const entry of [
	      ...(Array.isArray(options.ziffleCeremonies) ? options.ziffleCeremonies : []),
	      ...(Array.isArray(options.shuffleProofs) ? options.shuffleProofs : []),
	    ]) {
	      addCandidate(entry);
	    }
	    addCandidate(liveZiffleCeremoniesRef.current.get(normalizedOwner));
	    [...localZiffleCeremonyLookupRef.current.values()].reverse().forEach(addCandidate);
	    for (const entry of matchPayloadCeremoniesForLookup(options)) {
	      addCandidate(entry);
	    }
	    return candidates;
	  }, [hydrateZiffleCeremonyForLookup, matchPayloadCeremoniesForLookup]);

  const ziffleCeremonyForOwner = useCallback((owner, options = {}) => {
	    const candidates = ziffleCeremonyCandidatesForOwner(owner, options);
	    if (candidates.length === 0) return null;
	    const withSteps = candidates.find((entry) =>
	      Array.isArray(entry?.steps) && entry.steps.length > 0
	    );
	    const withKeys = candidates.find((entry) =>
	      Array.isArray(entry?.keys) && entry.keys.length > 0
	    );
	    const withObjectOrder = candidates.find((entry) => ziffleCeremonyHasObjectOrder(entry));
	    if (withObjectOrder) {
	      return {
	        ...withObjectOrder,
	        keys: cloneMultiplayerPayload(
	          Array.isArray(withObjectOrder.keys) && withObjectOrder.keys.length > 0
	            ? withObjectOrder.keys
	            : withKeys?.keys || []
	        ),
	        steps: cloneMultiplayerPayload(
	          Array.isArray(withObjectOrder.steps) && withObjectOrder.steps.length > 0
	            ? withObjectOrder.steps
	            : withSteps?.steps || []
	        ),
	      };
	    }
	    return withSteps || candidates[0];
	  }, [ziffleCeremonyCandidatesForOwner]);

  function zifflePositionForObjectId(owner, objectId, options = {}) {
    const normalizedObjectId = Number(objectId);
    if (!Number.isSafeInteger(normalizedObjectId) || normalizedObjectId < 0) return null;
    for (const ceremony of ziffleCeremonyCandidatesForOwner(owner, options)) {
      if (!ceremony?.deckHash) continue;
      const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
      const position = afterOrder.findIndex((entry) => Number(entry) === normalizedObjectId);
      if (position < 0) continue;
      return {
        ceremony,
        position,
        positionCommitment: ziffleRuntimeCommitment(ceremony.deckHash, position),
        ziffleContext: ziffleContextFromCeremony(ceremony),
      };
    }
    return null;
  }

	  function zifflePositionForOriginalSlot(owner, originalSlot, options = {}) {
	    const normalizedSlot = Number(originalSlot);
	    if (!Number.isSafeInteger(normalizedSlot) || normalizedSlot < 0) return null;
	    const matches = [];
	    for (const ceremony of ziffleCeremonyCandidatesForOwner(owner, options)) {
	      if (!ceremony?.deckHash) continue;
	      const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
	      const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
        const shuffleObjectId = Number(beforeOrder[normalizedSlot]);
        if (!Number.isSafeInteger(shuffleObjectId) || shuffleObjectId < 0) continue;
        const position = afterOrder.findIndex((entry) => Number(entry) === shuffleObjectId);
        if (position < 0) continue;
        matches.push({
          ceremony,
          position,
          shuffleObjectId,
          positionCommitment: ziffleRuntimeCommitment(ceremony.deckHash, position),
          ziffleContext: ziffleContextFromCeremony(ceremony),
        });
      }
      if (matches.length === 0) return null;
      const scoped = Boolean(
        options.context
        || options.ziffleContext
        || options.ziffle_context
        || options.deckHash
        || ziffleDeckHashFromCommitment(options.commitment)
      );
      if (scoped || matches.length === 1 || options.allowAmbiguousOriginalSlot === true) {
        return matches[0];
      }
      return null;
  }

  function ziffleTokensForPosition(tokens = [], position = null) {
    return (Array.isArray(tokens) ? tokens : [])
      .filter((token) =>
        position == null
        || token?.cardPosition == null
        || Number(token.cardPosition) === Number(position)
      )
      .map((token) => ({
        player: Number(token.player),
        publicKeyHex: String(token.publicKeyHex || ""),
        tokenHex: String(token.tokenHex || ""),
        proofHex: String(token.proofHex || ""),
      }));
  }

  function ziffleRevealTokenCacheKey(ceremony, player, position) {
    return [
      Number(ceremony?.owner ?? -1),
      ziffleKeyContextForCeremony(ceremony),
      String(ceremony?.context || ""),
      String(ceremony?.deckHash || ""),
      Number(player),
      Number(position),
    ].join(":");
  }

  function normalizeZiffleRevealToken(token, fallbackPosition = null) {
    if (!token || typeof token !== "object") return null;
    const position = Number(token.cardPosition ?? fallbackPosition);
    const player = Number(token.player);
    if (!Number.isSafeInteger(position) || position < 0 || !Number.isSafeInteger(player)) {
      return null;
    }
    return {
      player,
      publicKeyHex: String(token.publicKeyHex || ""),
      tokenHex: String(token.tokenHex || ""),
      proofHex: String(token.proofHex || ""),
      cardPosition: position,
    };
  }

  function rememberZiffleRevealTokens(ceremony, tokens = [], fallbackPositions = []) {
    const tokenList = Array.isArray(tokens) ? tokens : [tokens];
    const fallbackPosition = fallbackPositions.length === 1 ? fallbackPositions[0] : null;
    for (const token of tokenList) {
      const normalized = normalizeZiffleRevealToken(token, fallbackPosition);
      if (!normalized) continue;
      ziffleRevealTokenCacheRef.current.set(
        ziffleRevealTokenCacheKey(ceremony, normalized.player, normalized.cardPosition),
        normalized
      );
    }
  }

  function cachedZiffleRevealTokens(ceremony, player, positions = []) {
    const tokens = [];
    for (const position of positions) {
      const token = ziffleRevealTokenCacheRef.current.get(
        ziffleRevealTokenCacheKey(ceremony, player, position)
      );
      if (!token) return null;
      tokens.push({ ...token });
    }
    return tokens;
  }

	  function openingNeedsZiffleProof(opening) {
	    if (!opening) return false;
      if (ziffleOriginAnchorFromOpening(opening)) return true;
	    const proof = opening.ziffleReveal || opening.ziffleProof || opening.positionOpeningProof;
	    const positionCommitment = String(opening.positionCommitment || proof?.positionCommitment || "");
	    if (!ziffleDeckHashFromCommitment(positionCommitment)) return false;
	    if (proof) return true;
    const ceremony = ziffleCeremonyForOwner(opening.owner, {
      commitment: positionCommitment,
      context: ziffleContextFromOpening(opening),
    });
    if (ziffleCeremonyHasObjectOrder(ceremony)) {
      const position = Number(
        zifflePositionFromCommitment(positionCommitment) ?? opening.position
      );
      return !ziffleObjectOrderLinksOpening(ceremony, opening.slot, position, opening);
    }
	    return true;
	  }

  function ziffleCeremonyHasObjectOrder(ceremony) {
    return normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order).length > 0
      || normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order).length > 0;
  }

  function ziffleShuffleObjectIdForPosition(ceremony, position) {
    const normalizedPosition = Number(position);
    if (!Number.isSafeInteger(normalizedPosition) || normalizedPosition < 0) return null;
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    const objectId = Number(afterOrder[normalizedPosition]);
    return Number.isSafeInteger(objectId) && objectId >= 0 ? objectId : null;
  }

  function ziffleShuffleOriginalSlotForPosition(ceremony, position, objectId = null) {
    const normalizedPosition = Number(position);
    if (!Number.isSafeInteger(normalizedPosition) || normalizedPosition < 0) return null;
    const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    const positionObjectId = Number(afterOrder[normalizedPosition]);
    const explicitObjectId = Number(objectId);
    const targetObjectId =
      Number.isSafeInteger(positionObjectId) && positionObjectId >= 0
        ? positionObjectId
        : Number.isSafeInteger(explicitObjectId) && explicitObjectId >= 0
          ? explicitObjectId
          : null;
    if (targetObjectId == null) return null;
    if (beforeOrder.length === 0) {
      return afterOrder.length > 0 ? normalizedPosition : null;
    }
    const beforeIndex = beforeOrder.findIndex((entry) => Number(entry) === targetObjectId);
    return beforeIndex >= 0 ? beforeIndex : null;
  }

  function ziffleObjectOrderLinksOpening(ceremony, shuffleOriginalSlot, position, opening) {
    const proof = opening?.ziffleReveal || opening?.ziffleProof || opening?.positionOpeningProof || {};
    const normalizedShuffleOriginalSlot = Number(shuffleOriginalSlot);
    const hasShuffleOriginalSlot =
      Number.isSafeInteger(normalizedShuffleOriginalSlot) && normalizedShuffleOriginalSlot >= 0;
    const normalizedPosition = Number(position);
    const hasPosition = Number.isSafeInteger(normalizedPosition) && normalizedPosition >= 0;
    const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    if (beforeOrder.length === 0 && afterOrder.length === 0) return false;
    const beforeObjectId = hasShuffleOriginalSlot ? Number(beforeOrder[normalizedShuffleOriginalSlot]) : NaN;
    const afterObjectId = hasPosition ? Number(afterOrder[normalizedPosition]) : NaN;
    if (
      hasShuffleOriginalSlot
      && hasPosition
      && Number.isSafeInteger(beforeObjectId)
      && beforeObjectId >= 0
      && Number.isSafeInteger(afterObjectId)
      && afterObjectId >= 0
      && beforeObjectId === afterObjectId
    ) {
      return true;
    }
    const normalizedId = (value) => {
      const id = Number(value);
      return Number.isSafeInteger(id) && id >= 0 ? id : null;
    };
    const shuffleObjectId = normalizedId(
      proof?.shuffleObjectId
      ?? proof?.shuffle_object_id
      ?? opening?.shuffleObjectId
      ?? opening?.shuffle_object_id
    );
    const objectId = normalizedId(
      proof?.objectId
      ?? proof?.object_id
      ?? opening?.objectId
      ?? opening?.object_id
    );
    const beforeExpectedObjectId = shuffleObjectId ?? objectId;
    const afterExpectedObjectId = objectId ?? shuffleObjectId;
    if (beforeExpectedObjectId == null || afterExpectedObjectId == null) return false;
    // The ids come from the opening itself, so they may only confirm a link
    // the ceremony already implies: one object moving from slot to position.
    // Two different ids would let an opening pair any slot with any position.
    if (beforeExpectedObjectId !== afterExpectedObjectId) return false;
    // With one order missing there is no recorded permutation, so only the
    // identity mapping is implied.
    if (
      (beforeOrder.length === 0 || afterOrder.length === 0)
      && !(hasShuffleOriginalSlot && hasPosition && normalizedShuffleOriginalSlot === normalizedPosition)
    ) {
      return false;
    }
    const beforeMatches =
      beforeOrder.length === 0
      || (
        hasShuffleOriginalSlot
        && beforeObjectId === beforeExpectedObjectId
      );
    const afterMatches =
      afterOrder.length === 0
      || (
        hasPosition
        && afterObjectId === afterExpectedObjectId
      );
    return beforeMatches && afterMatches;
  }

  function ziffleRevealMatchesOpening(ceremony, revealOriginalSlot, position, opening) {
    if (Number(revealOriginalSlot) === Number(opening?.slot)) {
      return true;
    }
    const beforeOrder = normalizeShuffleOrder(ceremony?.beforeOrder ?? ceremony?.before_order);
    const afterOrder = normalizeShuffleOrder(ceremony?.afterOrder ?? ceremony?.after_order);
    if (
      beforeOrder.length === 0
      && afterOrder.length === 0
      && Number(revealOriginalSlot) === Number(opening?.slot)
    ) {
      return true;
    }
    return ziffleObjectOrderLinksOpening(ceremony, revealOriginalSlot, position, opening);
  }

  function ziffleOpeningProofHasAuthenticatedObjectOrder(ceremony, opening, position) {
    return ceremony?.authenticatedOrder === true
      && ziffleCeremonyHasObjectOrder(ceremony)
      && ziffleObjectOrderLinksOpening(ceremony, opening.slot, position, opening);
  }

  async function verifyZiffleOpeningProofForOpening(opening, options = {}) {
    const origin = ziffleOriginAnchorFromOpening(opening);
    const trustedOrigin = await currentZiffleOriginForOpening(opening, options);
    if (!origin && trustedOrigin) {
      throw new Error("Ziffle opening is missing its immutable origin anchor");
    }
    if (origin) {
      assertZiffleOpeningOriginMatchesMetadata(opening, trustedOrigin?.metadata);
    }
    return verifyZiffleOpeningCryptographicProof(opening, options);
  }

  // Envelope checks may run before an action creates its post-action objects.
  // Hydration separately requires the state's current-card origin binding above.
  async function verifyZiffleOpeningCryptographicProof(opening, options = {}) {
    const origin = ziffleOriginAnchorFromOpening(opening);
    const currentCeremony = ziffleCeremonyForOwner(opening.owner, {
      commitment: opening.positionCommitment, shuffleProofs: options.shuffleProofs || [],
      payload: options.payload || matchStartPayloadRef.current,
    });
    if (origin && isPrivateZiffleEpoch(currentCeremony)
      && origin.originPositionCommitment !== String(opening.positionCommitment || "")) {
      throw new Error("Private ciphertext epoch cannot inherit an earlier shuffle origin");
    }
    if (origin) {
      const originCeremony = ziffleCeremonyForOwner(opening.owner, {
        commitment: origin.originPositionCommitment,
        payload: options.payload || matchStartPayloadRef.current,
        shuffleProofs: options.shuffleProofs || [],
      });
      if (!originCeremony || ziffleCeremonyHasObjectOrder(originCeremony)
        || (String(originCeremony.context || "") !== currentAuditMatchId() && !isPrivateZiffleEpoch(originCeremony))) {
        throw new Error("Ziffle origin does not reference an authenticated ciphertext epoch");
      }
      const initialOpening = { ...opening,
        position: origin.originPosition,
        positionCommitment: origin.originPositionCommitment,
        ziffleContext: String(originCeremony.context || ""),
      };
      delete initialOpening.originPosition;
      delete initialOpening.originPositionCommitment;
      delete initialOpening.origin_position;
      delete initialOpening.origin_position_commitment;
      return verifyZiffleOpeningCryptographicProof(initialOpening, options);
    }
    if (!openingNeedsZiffleProof(opening)) return;
    const proof = opening.ziffleReveal || opening.ziffleProof || opening.positionOpeningProof;
    if (!proof || typeof proof !== "object") {
      throw new Error("Ziffle card opening is missing its reveal proof");
    }
    if (String(proof.type || "") !== "ziffle_position_opening_v1") {
      throw new Error("Ziffle card opening proof type is unsupported");
    }
    const position = Number(
      zifflePositionFromCommitment(opening.positionCommitment)
      ?? opening.position
      ?? proof.position
    );
    if (!Number.isSafeInteger(position) || position < 0) {
      throw new Error("Ziffle card opening is missing a valid shuffled position");
    }
    if (Number(proof.owner) !== Number(opening.owner)) {
      throw new Error("Ziffle card opening proof owner mismatch");
    }
    if (Number(proof.position) !== position) {
      throw new Error("Ziffle card opening proof position mismatch");
    }
    if (Number(proof.originalSlot) !== Number(opening.slot)) {
      throw new Error("Ziffle card opening proof slot mismatch");
    }
	    const storedCeremony = ziffleCeremonyForOwner(opening.owner, {
	      payload: options.payload || matchStartPayloadRef.current,
	      shuffleProofs: options.shuffleProofs || [],
	      ziffleCeremonies: options.ziffleCeremonies || [],
	      commitment: opening.positionCommitment || proof.positionCommitment,
	      deckHash: proof.deckHash,
	      context: ziffleContextFromOpening(opening) || proof.context,
	    });
	    const ceremony = hydrateZiffleCeremonyForLookup(
	      ziffleCeremonyForOpeningProof(proof, storedCeremony),
	      {
	        payload: options.payload || matchStartPayloadRef.current,
	        shuffleProofs: options.shuffleProofs || [],
	        ziffleCeremonies: options.ziffleCeremonies || [],
	      }
	    );
	    if (!ceremony) {
	      throw new Error(`Missing ziffle ceremony for opening player ${Number(opening.owner) + 1}`);
	    }
    if (ziffleCeremonyHasObjectOrder(ceremony)) {
      if (ziffleOpeningProofHasAuthenticatedObjectOrder(ceremony, opening, position)) {
        return;
      }
      if (!proof && ziffleObjectOrderLinksOpening(ceremony, opening.slot, position, opening)) {
        return;
      }
      if (!proof) {
        const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
        const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
        const slotObjectId = Number(beforeOrder[Number(opening.slot)]);
        const positionObjectId = Number(afterOrder[position]);
        const openingObjectId = Number(
          opening.shuffleObjectId
          ?? opening.shuffle_object_id
          ?? opening.objectId
          ?? opening.object_id
        );
        if (
          !Number.isSafeInteger(slotObjectId)
          || !Number.isSafeInteger(positionObjectId)
          || !Number.isSafeInteger(openingObjectId)
        ) {
          throw new Error("Ziffle card opening object order does not match reveal");
        }
        throw new Error("Ziffle card opening object order does not match reveal");
      }
    }
    const positionCommitment =
      String(opening.positionCommitment || proof.positionCommitment || "")
      || ziffleRuntimeCommitment(ceremony.deckHash, position);
    if (positionCommitment !== ziffleRuntimeCommitment(ceremony.deckHash, position)) {
      throw new Error("Ziffle card opening position commitment mismatch");
    }
    if (String(proof.positionCommitment || "") !== positionCommitment) {
      throw new Error("Ziffle card opening proof commitment mismatch");
    }
    if (String(proof.context || "") !== String(ceremony.context || "")) {
      throw new Error("Ziffle card opening proof context mismatch");
    }
    if (String(proof.keyContext || proof.context || "") !== ziffleKeyContextForCeremony(ceremony)) {
      throw new Error("Ziffle card opening proof key context mismatch");
    }
    if (String(proof.deckHash || "") !== String(ceremony.deckHash || "")) {
      throw new Error("Ziffle card opening proof deck hash mismatch");
    }
    if (Number(proof.deckCount) !== Number(ceremony.deckCount)) {
      throw new Error("Ziffle card opening proof deck count mismatch");
    }
    const currentGame = gameRef.current;
    if (!currentGame || typeof currentGame.ziffleRevealCard !== "function") {
      throw new Error("Ziffle opening reveal backend is not available");
    }
    const tokens = ziffleTokensForPosition(proof.tokens || [], position);
    if (tokens.length === 0) {
      throw new Error("Ziffle card opening proof is missing reveal tokens");
    }
    const reveal = await currentGame.ziffleRevealCard({
      deckCount: Number(ceremony.deckCount),
      context: String(ceremony.context || ""),
      keyContext: ziffleKeyContextForCeremony(ceremony),
      keys: cloneMultiplayerPayload(ceremony.keys || []),
      steps: cloneMultiplayerPayload(ceremony.steps || []),
      ...ziffleInputDeckFields(ceremony),
      cardPosition: position,
      tokens,
    });
    const revealOriginalSlot = Number(reveal.originalSlot);
    const proofShuffleOriginalSlot = Number(proof.shuffleOriginalSlot ?? proof.originalSlot);
    if (revealOriginalSlot !== proofShuffleOriginalSlot) {
      throw new Error(
        `Ziffle card opening proof reveals a different shuffle slot `
        + `(owner ${Number(opening.owner)}, position ${position}, proof slot ${proofShuffleOriginalSlot}, `
        + `revealed slot ${revealOriginalSlot}, card ${String(opening.card || "")})`
      );
    }
	    if (!ziffleRevealMatchesOpening(ceremony, revealOriginalSlot, position, opening)) {
	      throw new Error(
	        `Ziffle card opening proof reveals a different committed slot `
	        + `(owner ${Number(opening.owner)}, position ${position}, opening slot ${Number(opening.slot)}, `
	        + `revealed slot ${Number(reveal.originalSlot)}, card ${String(opening.card || "")})`
	      );
	    }
  }

	  async function ensureZiffleOpeningProof(opening, options = {}) {
	    const openingPositionCommitment = String(
	      opening?.positionCommitment || opening?.position_commitment || ""
	    );
	    const openingPosition = Number(
	      zifflePositionFromCommitment(openingPositionCommitment) ?? opening?.position
	    );
	    const openingHasZiffleIdentity = Boolean(
	      ziffleDeckHashFromCommitment(openingPositionCommitment)
	      && Number.isSafeInteger(openingPosition)
	      && openingPosition >= 0
	    );
    const claimedOrigin = ziffleOriginAnchorFromOpening(opening);
    const trustedOrigin = openingHasZiffleIdentity
      ? await currentZiffleOriginForOpening(opening, options)
      : null;
    if (claimedOrigin) {
      assertZiffleOpeningOriginMatchesMetadata(opening, trustedOrigin?.metadata);
    }
    if (trustedOrigin) {
      const originCeremony = ziffleCeremonyForOwner(opening.owner, {
        commitment: trustedOrigin.originPositionCommitment,
        shuffleProofs: options.shuffleProofs || [],
      });
      if (!originCeremony || ziffleCeremonyHasObjectOrder(originCeremony)
        || (String(originCeremony.context || "") !== currentAuditMatchId() && !isPrivateZiffleEpoch(originCeremony))) {
        throw new Error("Ziffle origin does not reference an authenticated ciphertext epoch");
      }
      const anchoredOpening = {
        ...opening,
        objectId: trustedOrigin.objectId,
        originPosition: trustedOrigin.originPosition,
        originPositionCommitment: trustedOrigin.originPositionCommitment,
      };
      const oldProof = opening.ziffleReveal || opening.ziffleProof || opening.positionOpeningProof;
      if (oldProof && String(oldProof.positionCommitment || "") === trustedOrigin.originPositionCommitment) {
        // No await or game mutation separates this binding from the trusted
        // lookup above. Validate against that same snapshot, then perform the
        // full cryptographic check without exporting the checkpoint twice.
        assertZiffleOpeningOriginMatchesMetadata(anchoredOpening, trustedOrigin.metadata);
        await verifyZiffleOpeningCryptographicProof(anchoredOpening, options);
        return anchoredOpening;
      }
      const currentGame = gameRef.current;
      if (typeof currentGame?.ziffleRevealCard !== "function") {
        throw new Error("Ziffle opening reveal backend is not available");
      }
      const tokens = await collectZiffleRevealTokens(originCeremony, trustedOrigin.originPosition, options);
      const reveal = await currentGame.ziffleRevealCard({
        deckCount: Number(originCeremony.deckCount),
        context: String(originCeremony.context || ""),
        keyContext: ziffleKeyContextForCeremony(originCeremony),
        keys: cloneMultiplayerPayload(originCeremony.keys || []),
        steps: cloneMultiplayerPayload(originCeremony.steps || []),
        ...ziffleInputDeckFields(originCeremony),
        cardPosition: trustedOrigin.originPosition,
        tokens,
      });
      const originalSlot = Number(reveal.originalSlot);
      if (originalSlot !== Number(opening.slot)) {
        throw new Error("Ziffle immutable origin reveals a different committed slot");
      }
      delete anchoredOpening.ziffleProof;
      delete anchoredOpening.positionOpeningProof;
      anchoredOpening.ziffleReveal = buildZiffleOpeningProof({
        opening: anchoredOpening,
        ceremony: originCeremony,
        position: trustedOrigin.originPosition,
        positionCommitment: trustedOrigin.originPositionCommitment,
        originalSlot,
        shuffleOriginalSlot: originalSlot,
        tokens,
        compact: true,
      });
      return anchoredOpening;
    }
	    if (!options.forceZiffleOpeningProof && !openingNeedsZiffleProof(opening)) return opening;
	    if (options.forceZiffleOpeningProof && !openingHasZiffleIdentity) return opening;
	    const currentGame = gameRef.current;
	    if (!currentGame || typeof currentGame.ziffleRevealCard !== "function") {
	      throw new Error("Ziffle opening reveal backend is not available");
	    }
	    const existingProof = opening.ziffleReveal || opening.ziffleProof || opening.positionOpeningProof;
	    if (existingProof) {
	      if (options.skipFreshZiffleOpeningProofVerification && !options.forceZiffleOpeningProof) {
	        return opening;
	      }
	      try {
	        await verifyZiffleOpeningProofForOpening(opening);
	        return opening;
	      } catch (err) {
	        const message = String(err?.message || err || "");
	        const canRebuildStaleProof =
	          !options._rebuiltStaleZiffleProof
	          && (
	            message.includes("reveals a different committed slot")
	            || message.includes("reveals a different shuffle slot")
	            || message.includes("object order does not match reveal")
	          );
	        if (!canRebuildStaleProof) {
	          throw err;
	        }
	        const rebuiltOpening = { ...opening };
	        delete rebuiltOpening.ziffleReveal;
	        delete rebuiltOpening.ziffleProof;
	        delete rebuiltOpening.positionOpeningProof;
	        return ensureZiffleOpeningProof(rebuiltOpening, {
	          ...options,
	          _rebuiltStaleZiffleProof: true,
	          forceZiffleOpeningProof: true,
	        });
	      }
	    }
	    const position = openingPosition;
    if (!Number.isSafeInteger(position) || position < 0) {
      throw new Error("Ziffle card opening is missing a valid shuffled position");
    }
    const ceremony = ziffleCeremonyForOwner(opening.owner, {
      commitment: opening.positionCommitment,
      context: ziffleContextFromOpening(opening),
    });
    if (!ceremony) {
      throw new Error(`Missing ziffle ceremony for opening player ${Number(opening.owner) + 1}`);
    }
    const tokens = await collectZiffleRevealTokens(ceremony, position, options);
    const reveal = await currentGame.ziffleRevealCard({
      deckCount: Number(ceremony.deckCount),
      context: String(ceremony.context || ""),
      keyContext: ziffleKeyContextForCeremony(ceremony),
      keys: cloneMultiplayerPayload(ceremony.keys || []),
      steps: cloneMultiplayerPayload(ceremony.steps || []),
      ...ziffleInputDeckFields(ceremony),
      cardPosition: position,
      tokens,
    });
    const revealOriginalSlot = Number(reveal.originalSlot);
	    const positionCommitment =
	      String(opening.positionCommitment || "")
	      || ziffleRuntimeCommitment(ceremony.deckHash, position);
	    let proofOpening = opening;
	    let proofOriginalSlot = Number(opening.slot);
	    const manifest = privateDeckManifestForOwner(opening.owner);
	    // An in-game shuffle index can differ from the original committed slot.
	    // Keep the opening when its object is already linked by that shuffle.
	    if (!ziffleRevealMatchesOpening(ceremony, revealOriginalSlot, position, opening)) {
	      const beforeOrder = normalizeShuffleOrder(ceremony.beforeOrder ?? ceremony.before_order);
	      const afterOrder = normalizeShuffleOrder(ceremony.afterOrder ?? ceremony.after_order);
	      const shuffleObjectId = Number(beforeOrder[revealOriginalSlot]);
	      const positionObjectId = Number(afterOrder[position]);
	      const orderLinkedOpening = {
        ...opening,
        ...(Number.isSafeInteger(shuffleObjectId) && shuffleObjectId >= 0
          ? { shuffleObjectId }
          : {}),
      };
	      if (
	        Number(opening.slot) === Number(revealOriginalSlot)
	        && Number.isSafeInteger(shuffleObjectId)
	        && shuffleObjectId >= 0
	        && Number(positionObjectId) === Number(shuffleObjectId)
	        && ziffleRevealMatchesOpening(ceremony, revealOriginalSlot, position, orderLinkedOpening)
	      ) {
	        proofOpening = orderLinkedOpening;
	      } else {
	      const resolvedRevealSlot = await resolveCommittedZiffleRevealSlot({
	        owner: opening.owner,
	        ceremony,
	        shuffleOriginalSlot: revealOriginalSlot,
	        shuffleOriginalSlotIsVerified: true,
        position,
        card: opening.card || "",
        objectId: opening.objectId,
        manifest,
        options,
      });
      if (!resolvedRevealSlot) {
        throw new Error(
          `Ziffle card opening proof reveals a different committed slot `
          + `(owner ${Number(opening.owner)}, position ${position}, opening slot ${Number(opening.slot)}, `
          + `revealed slot ${Number(reveal.originalSlot)}, card ${String(opening.card || "")})`
        );
      }
      proofOriginalSlot = Number(resolvedRevealSlot.slot);
      const rebuiltOpening = await buildDeckSlotOpening({
        manifest,
        slot: proofOriginalSlot,
        card: resolvedRevealSlot.card || opening.card,
      });
      proofOpening = {
        ...opening,
        ...rebuiltOpening,
        ...(resolvedRevealSlot.objectId != null ? { objectId: Number(resolvedRevealSlot.objectId) } : {}),
        ...(resolvedRevealSlot.shuffleObjectId != null || resolvedRevealSlot.objectId != null
          ? { shuffleObjectId: Number(resolvedRevealSlot.shuffleObjectId ?? resolvedRevealSlot.objectId) }
          : {}),
        reportedSlot: Number(opening.slot),
      };
      }
    }
    return {
      ...proofOpening,
      position,
      positionCommitment,
      ziffleContext: ziffleContextFromCeremony(ceremony),
      ziffleReveal: buildZiffleOpeningProof({
        opening: {
          ...proofOpening,
          position,
          positionCommitment,
          ziffleContext: ziffleContextFromCeremony(ceremony),
        },
        ceremony,
        position,
        originalSlot: proofOriginalSlot,
        shuffleOriginalSlot: revealOriginalSlot,
        positionCommitment,
        tokens,
        compact: true,
      }),
    };
  }

  const localZiffleDiagnostics = useCallback((label = "local") => {
    const session = multiplayerRef.current;
    return {
      label,
      localPeerId: String(session.localPeerId || ""),
      role: String(session.role || ""),
      mode: String(session.mode || ""),
      matchStarted: Boolean(session.matchStarted),
      localPlayerIndex:
        session.localPlayerIndex == null ? null : Number(session.localPlayerIndex),
      lobbyId: String(session.lobbyId || ""),
      hostPeerId: String(session.hostPeerId || ""),
      auditMatchId: String(matchStartPayloadRef.current?.auditMatchId || ""),
      matchStartPayloadPresent: Boolean(matchStartPayloadRef.current),
      payloadCeremonies: (matchStartPayloadRef.current?.ziffleCeremonies || [])
        .map(compactZiffleCeremonyForDiagnostics)
        .filter(Boolean),
      liveCeremonies: [...liveZiffleCeremoniesRef.current.values()]
        .map(compactZiffleCeremonyForDiagnostics)
        .filter(Boolean),
      players: (session.players || []).map((player) => ({
        index: Number(player.index),
        name: String(player.name || ""),
        peerId: String(player.peerId || ""),
        connected: player.connected !== false,
        hasZiffleKey: Boolean(player.ziffleKey),
      })),
    };
  }, []);

  const emitZiffleDiagnosticNotice = useCallback((title, err, diagnostics = null) => {
    const message = toErrorMessage(err, "Unknown ziffle ceremony");
    const mergedDiagnostics = {
      message,
      local: localZiffleDiagnostics(title),
      ...(err?.ziffleDiagnostics && typeof err.ziffleDiagnostics === "object"
        ? { error: err.ziffleDiagnostics }
        : {}),
      ...(diagnostics && typeof diagnostics === "object" ? diagnostics : {}),
    };
    const json = compactZiffleDiagnosticsJson(mergedDiagnostics);
    emitSyncFailureNotice(title, {
      body: ziffleDiagnosticNoticeBody(message, mergedDiagnostics),
      copyText: json,
      copyStatusMessage: "Copied Ziffle diagnostics",
      actions: [
        {
          label: "Copy diagnostics",
          copyText: json,
          copyStatusMessage: "Copied Ziffle diagnostics",
        },
      ],
    });
    console.warn("Ironsmith Ziffle diagnostics", mergedDiagnostics);
  }, [localZiffleDiagnostics]);

  const importCachedAuditPublicKey = useCallback(async (rawHex) => {
    const normalized = String(rawHex || "").trim();
    if (!normalized) {
      throw new Error("Missing audit public key");
    }
    if (!auditVerifyKeyCacheRef.current.has(normalized)) {
      auditVerifyKeyCacheRef.current.set(
        normalized,
        await importAuditPublicKey(normalized)
      );
    }
    return auditVerifyKeyCacheRef.current.get(normalized);
  }, []);

  async function signActionIntentForCommand({
    seq,
    actorIndex,
    command,
    prevStateHash,
    preActionPublicCheckpointHash,
  }) {
    const retained = assertPaymentDisclosureIntent({ matchId: currentAuditMatchId(), seq, actorIndex, prevStateHash, command });
    const previous = retained?.signedIntent || retained?.evidence?.actionIntent || retained?.timing?.intent;
    if (previous) {
      if (String(previous.preActionPublicCheckpointHash) !== String(preActionPublicCheckpointHash)) {
        throw new Error("Disclosed payment pre-state changed; recover its accepted prefix before retrying");
      }
      return cloneMultiplayerPayload(previous);
    }
    const { keyPair } = await ensureAuditIdentity();
    const payload = signedActionIntentPayload({
      matchId: currentAuditMatchId(),
      attemptId: randomAuditHex(16),
      seq,
      actorIndex,
      prevStateHash,
      preActionPublicCheckpointHash,
      command,
    });
    return {
      ...payload,
      signatureAlgorithm: "ecdsa-p256-sha256",
      signature: await signAuditPayload(keyPair, payload),
    };
  }

  async function verifySignedActionIntent(intent, expected = {}) {
    if (!intent || typeof intent !== "object") {
      throw new Error("Cryptographic material request is missing a signed action intent");
    }
    const payload = signedActionIntentPayload(intent);
    const expectedPayload = signedActionIntentPayload({
      matchId: expected.matchId ?? currentAuditMatchId(),
      seq: expected.seq ?? payload.seq,
      actorIndex: expected.actorIndex ?? payload.actorIndex,
      prevStateHash: expected.prevStateHash ?? payload.prevStateHash,
      preActionPublicCheckpointHash:
        expected.preActionPublicCheckpointHash
        ?? expected.publicCheckpointHash
        ?? payload.preActionPublicCheckpointHash,
      command: expected.command ?? payload.command,
      attemptId: expected.attemptId ?? payload.attemptId,
    });
    if (
      payload.domain !== ACTION_INTENT_DOMAIN
      || String(payload.attemptId || "") !== String(expectedPayload.attemptId || "")
      || payload.matchId !== expectedPayload.matchId
      || Number(payload.seq) !== Number(expectedPayload.seq)
      || Number(payload.actorIndex) !== Number(expectedPayload.actorIndex)
      || payload.prevStateHash !== expectedPayload.prevStateHash
      || payload.preActionPublicCheckpointHash !== expectedPayload.preActionPublicCheckpointHash
      || canonicalMultiplayerPayload(payload.command) !== canonicalMultiplayerPayload(expectedPayload.command)
    ) {
      throw new Error("Signed action intent does not match the requested action");
    }
    const publicKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(payload.actorIndex));
    const valid = await verifyAuditPayload(publicKey, payload, intent.signature || "");
    if (!valid) {
      throw new Error("Signed action intent signature is invalid");
    }
    return {
      ...payload,
      signatureAlgorithm: "ecdsa-p256-sha256",
      signature: String(intent.signature || ""),
    };
  }

  const IGNORED_ACTION_INTENT_TTL_MS = 10 * 60 * 1000;
  const MAX_IGNORED_ACTION_INTENTS = 256;

  function pruneIgnoredActionIntents(nowMs = Date.now()) {
    for (const [key, record] of ignoredActionIntentKeysRef.current.entries()) {
      if (nowMs - Number(record?.at || 0) > IGNORED_ACTION_INTENT_TTL_MS) {
        ignoredActionIntentKeysRef.current.delete(key);
      }
    }
    while (ignoredActionIntentKeysRef.current.size > MAX_IGNORED_ACTION_INTENTS) {
      const oldestKey = ignoredActionIntentKeysRef.current.keys().next().value;
      if (!oldestKey) break;
      ignoredActionIntentKeysRef.current.delete(oldestKey);
    }
  }

  function rememberIgnoredActionIntentKey(actionIntentKeyValue, reason = "", intent = null) {
    const key = String(actionIntentKeyValue || "");
    if (!key) return false;
    const fingerprint = intent ? actionIntentFingerprint(intent) : "";
    const storageKey = fingerprint ? `${key}:${fingerprint}` : key;
    const nowMs = Date.now();
    ignoredActionIntentKeysRef.current.set(storageKey, { reason: String(reason || ""), at: nowMs });
    pruneIgnoredActionIntents(nowMs);
    const active = pendingActionIntentsRef.current.get(key);
    if (!active || !intent || active.fingerprint === fingerprint) clearPeerWaitForActionIntent(key);
    return true;
  }

  function ignoredActionIntentReason(actionIntentKeyValue, intent = null) {
    const key = String(actionIntentKeyValue || "");
    if (!key) return "";
    pruneIgnoredActionIntents();
    const currentIntent = intent || pendingActionIntentsRef.current.get(key)?.intent;
    const specific = currentIntent
      ? ignoredActionIntentKeysRef.current.get(`${key}:${actionIntentFingerprint(currentIntent)}`) : null;
    return String(specific?.reason || ignoredActionIntentKeysRef.current.get(key)?.reason || "");
  }

  function actionIntentKeyFromProtocolPayload(payload = {}) {
    try {
      const intent =
        payload?.actionIntent
        || payload?.actionAuthorization?.actionIntent
        || payload?.action_authorization?.actionIntent
        || payload?.action_authorization?.action_intent
        || null;
      return intent ? actionIntentKey(intent) : "";
    } catch {
      return "";
    }
  }

  function actionIntentKeyFromProtocolClaim(claim = {}) {
    return actionIntentKeyFromProtocolPayload(claim.requestPayload || claim.request_payload || claim);
  }

  function protocolActionIntentInactiveReason(actionIntentKeyValue = "", intent = null) {
    const ignoredReason = ignoredActionIntentReason(actionIntentKeyValue, intent);
    if (ignoredReason) return ignoredReason;
    const session = multiplayerRef.current;
    if (session.mode === "disputed") return "match_disputed";
    if (!session.matchStarted) return "match_not_started";
    return "";
  }

  function isDirectProtocolMessage(message = {}) {
    return [
      "ziffle_shuffle_step_request",
      "ziffle_shuffle_step_response",
      "ziffle_reveal_token_request",
      "ziffle_reveal_token_response",
      "rng_commit_request",
      "rng_commit_response",
      "rng_reveal_request",
      "rng_reveal_response",
      "timeout_vote_request",
      "timeout_vote_response",
      "disconnect_forfeit_vote_request",
      "disconnect_forfeit_vote_response",
      "protocol_timeout_vote_request",
      "protocol_timeout_vote_response",
      "action_quorum_vote_request",
      "action_quorum_vote_response",
      "crypto_material_request",
      "crypto_material_response",
      "action_intent_progress",
      "action_intent_cancel",
    ].includes(String(message?.type || ""));
  }

  function shouldSuppressProtocolMessageError(err, message = {}) {
    if (!isDirectProtocolMessage(message)) return false;
    const key = actionIntentKeyFromProtocolPayload(message);
    const inactiveReason = protocolActionIntentInactiveReason(key);
    const messageText = toErrorMessage(err);
    if (inactiveReason) {
      recordPeerSyncPerf("protocol_message_error:suppressed", {
        message_type: String(message?.type || ""),
        request_id: String(message?.requestId || ""),
        action_intent_key: key,
        reason: inactiveReason,
        error: messageText,
      });
      return true;
    }
    if (
      key
      && messageText.includes("Match clock hash chain does not match local transcript")
      && ignoredActionIntentReason(key)
    ) {
      recordPeerSyncPerf("protocol_message_error:suppressed", {
        message_type: String(message?.type || ""),
        request_id: String(message?.requestId || ""),
        action_intent_key: key,
        reason: "ignored_action_intent_clock_hash",
        error: messageText,
      });
      return true;
    }
    return false;
  }

  function clearPendingActionIntent(intentOrKey) {
    const key = typeof intentOrKey === "string" ? intentOrKey : actionIntentKey(intentOrKey);
    if (!key) return;
    const timeoutId = pendingActionIntentTimeoutsRef.current.get(key);
    if (timeoutId) {
      window.clearTimeout(timeoutId);
      pendingActionIntentTimeoutsRef.current.delete(key);
    }
    pendingActionIntentsRef.current.delete(key);
    actionIntentOpeningPreviewKeysRef.current.delete(key);
    clearPeerWaitForActionIntent(key);
  }

  function clearAllPendingActionIntents() {
    for (const timeoutId of pendingActionIntentTimeoutsRef.current.values()) {
      window.clearTimeout(timeoutId);
    }
    pendingActionIntentTimeoutsRef.current.clear();
    pendingActionIntentsRef.current.clear();
    actionIntentOpeningPreviewKeysRef.current.clear();
  }

  function ignoreAndClearAllPendingActionIntents(reason = "") {
    for (const key of pendingActionIntentsRef.current.keys()) {
      rememberIgnoredActionIntentKey(key, reason);
    }
    clearAllPendingActionIntents();
  }

  function pendingActionIntentEvidenceTimeoutMs(evidence = {}) {
    return Math.max(1, Number(evidence.responseTimeoutMs || PROTOCOL_RESPONSE_TIMEOUT_MS));
  }

  function pendingActionIntentEvidenceRequestedAtMs(evidence = {}) {
    return Math.max(
      1,
      Number(evidence.requestedAtMs || Date.now() - pendingActionIntentEvidenceTimeoutMs(evidence))
    );
  }

  function pendingActionIntentFirstObservedAtMs(record = {}) {
    return Math.max(
      1,
      Number(
        record.firstObservedAtMs
        || record.evidence?.requestedAtMs
        || Date.now()
      )
    );
  }

  function pendingActionIntentEvidenceDueAtMs(evidence = {}) {
    if (!evidence?.requestPayload) return Infinity;
    return (
      pendingActionIntentEvidenceRequestedAtMs(evidence)
      + pendingActionIntentEvidenceTimeoutMs(evidence)
      + MATCH_CLOCK_CLAIM_SKEW_MS
    );
  }

  function pendingActionIntentHardDueAtMs(record = {}) {
    return (
      pendingActionIntentFirstObservedAtMs(record)
      + MAX_PENDING_ACTION_INTENT_MS
      + MATCH_CLOCK_CLAIM_SKEW_MS
    );
  }

  function pendingActionIntentDueAtMs(record = {}) {
    if (!record?.intent) return Infinity;
    return Math.min(
      pendingActionIntentEvidenceDueAtMs(record.evidence || {}),
      pendingActionIntentHardDueAtMs(record)
    );
  }

  function shouldReplacePendingActionIntentEvidence(record = {}, evidence = {}) {
    if (!evidence?.requestPayload) return false;
    if (!record.evidence?.requestPayload) return true;
    const evidenceDueAtMs = pendingActionIntentEvidenceDueAtMs(evidence);
    const previousDueAtMs = pendingActionIntentEvidenceDueAtMs(record.evidence);
    return evidenceDueAtMs >= previousDueAtMs;
  }

  async function pendingActionIntentHardTimeoutEvidence(key, record = {}) {
    const requestId = String(key || actionIntentKey(record.intent) || "");
    const requestPayload = {
      type: "pending_action_intent",
      protocolVersion: PROTOCOL_VERSION,
      requestId,
      actionIntent: cloneMultiplayerPayload(record.intent || {}),
      firstObservedAtMs: pendingActionIntentFirstObservedAtMs(record),
    };
    return {
      requestType: "pending_action_intent",
      requestId,
      requestPayload,
      requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
      responseTimeoutMs: MAX_PENDING_ACTION_INTENT_MS,
      requestedAtMs: pendingActionIntentFirstObservedAtMs(record),
    };
  }

  function schedulePendingActionIntentTimeout(key, record) {
    if (!key || !record?.intent) return;
    persistPendingPaymentTiming(record);
    const existingTimeoutId = pendingActionIntentTimeoutsRef.current.get(key);
    if (existingTimeoutId) {
      window.clearTimeout(existingTimeoutId);
      pendingActionIntentTimeoutsRef.current.delete(key);
    }
    const dueAtMs = Math.max(
      pendingActionIntentDueAtMs(record),
      Number(record.timeoutConfirmation?.notBeforeMs || 0)
    );
    if (!Number.isFinite(dueAtMs)) return;
    const delayMs = Math.max(1, Math.ceil(dueAtMs - Date.now()));
    const timeoutId = window.setTimeout(() => {
      // A cancelled callback can already be queued when a progress update
      // installs its replacement. It must not remove or act for that timer.
      if (pendingActionIntentTimeoutsRef.current.get(key) !== timeoutId
        || pendingActionIntentsRef.current.get(key) !== record) return;
      pendingActionIntentTimeoutsRef.current.delete(key);
      void handlePendingActionIntentTimeout(key, dueAtMs).catch((err) => {
        emitSyncFailureNotice(
          "Action intent timeout failed",
          err instanceof Error ? err.message : String(err)
        );
        setStatus(`Action intent timeout failed: ${toErrorMessage(err)}`, true);
      });
    }, delayMs);
    pendingActionIntentTimeoutsRef.current.set(key, timeoutId);
  }

  function matchingAppliedActionForIntent(intent) {
    const payload = signedActionIntentPayload(intent);
    const applied = actionHistoryEntryForSequence(payload.seq);
    if (!applied) return null;
    if (Number(applied.actorIndex) !== Number(payload.actorIndex)) return null;
    if (canonicalMultiplayerPayload(applied.command) !== canonicalMultiplayerPayload(payload.command)) {
      return null;
    }
    if (String(applied.audit?.prevStateHash || "") !== payload.prevStateHash) return null;
    return applied;
  }

  async function observedMatchClockElapsedForIntent(intent, record) {
    const payload = signedActionIntentPayload(intent);
    const key = actionIntentKey(payload);
    // Progress is an observation of the existing clock epoch, not a game-state
    // transition. Reading WASM here floods the verification queue when progress
    // arrives faster than snapshots, delaying the very response being awaited.
    if (pendingActionIntentsRef.current.get(key) !== record
      || protocolActionIntentInactiveReason(key)
      || payload.matchId !== currentAuditMatchId()
      || Number(payload.seq) <= Number(multiplayerRef.current.lastAppliedSequence || 0)) return null;
    const snapshot = servicesRef.current.runtimeMatchClockSnapshot?.()
      || multiplayerRef.current.matchClock;
    if (!snapshot?.enabled || Number(snapshot.activePlayerIndex) !== Number(payload.actorIndex)) {
      return null;
    }
    if (snapshot.startedAtMs == null) return 0;
    return Math.max(0, Math.floor(nowMonotonicMs() - Number(snapshot.startedAtMs)));
  }

  function pendingActionIntentHeldForProtocolWork(record = {}) {
    const requestType = String(record?.evidence?.requestType || "");
    if ([
      "action_quorum_vote_request",
      "crypto_material_request",
      "rng_reveal_request",
      "ziffle_reveal_token_request",
      "ziffle_shuffle_step_request",
    ].includes(requestType)) {
      return true;
    }
    if (requestType !== "action_intent_progress") return false;
    const phase = String(record?.evidence?.requestPayload?.phase || "");
    return [
      "payload_generation",
      "engine_work",
      "crypto_material",
      "opening_generation",
      "opening_preview",
      "payload_signing",
      "action_broadcast",
    ].includes(phase);
  }

  function actionBroadcastResponseTimeoutMs(actionPayload) {
    const openingCount = Array.isArray(actionPayload?.audit?.openings)
      ? actionPayload.audit.openings.length
      : 0;
    if (openingCount <= 0) return PROTOCOL_RESPONSE_TIMEOUT_MS;
    return Math.max(
      PROTOCOL_RESPONSE_TIMEOUT_MS,
      ziffleRevealTokenTimeoutMs(openingCount)
    );
  }

  async function rememberPendingActionIntent(intent, evidence = {}) {
    const verifiedIntent = await verifySignedActionIntent(intent);
    const key = actionIntentKey(verifiedIntent);
    // Once this peer revealed fair-random material for the actor's intent at
    // this sequence, no other intent at that sequence is acceptable, even
    // after a cancel (the actor has already seen the randomness).
    if (servicesRef.current.fairRandomRevealLockConflict?.(verifiedIntent)) {
      throw new Error("Refusing conflicting signed action intent for this sequence");
    }
    assertPaymentDisclosureIntent(verifiedIntent);
    const inactiveReason = protocolActionIntentInactiveReason(key, verifiedIntent);
    if (inactiveReason) {
      recordPeerSyncPerf("action_intent:ignored", {
        key,
        reason: inactiveReason,
        request_type: String(evidence?.requestType || ""),
        request_id: String(evidence?.requestId || ""),
      });
      return verifiedIntent;
    }
    const fingerprint = actionIntentFingerprint(verifiedIntent);
    const existing = pendingActionIntentsRef.current.get(key);
    if (existing && existing.fingerprint !== fingerprint) {
      throw new Error("Refusing conflicting signed action intent for this sequence");
    }
    if (matchingAppliedActionForIntent(verifiedIntent)
      || Number(verifiedIntent.seq) <= Number(multiplayerRef.current.lastAppliedSequence || 0)
      || verifiedIntent.matchId !== currentAuditMatchId()) {
      return verifiedIntent;
    }
    const retainedTiming = pinnedPaymentDisclosure(verifiedIntent)?.timing;
    const record = existing || {
      intent: cloneMultiplayerPayload(verifiedIntent),
      fingerprint,
      evidence: cloneMultiplayerPayload(retainedTiming?.evidence || null),
      firstObservedAtMs: retainedTiming?.firstObservedAtMs || Date.now(),
      observedElapsedAtIntentMs: retainedTiming?.observedElapsedAtIntentMs ?? null,
      timeoutConfirmation: cloneMultiplayerPayload(retainedTiming?.timeoutConfirmation || null),
    };
    if (!record.firstObservedAtMs) {
      record.firstObservedAtMs = Date.now();
    }
    if (evidence?.requestPayload) {
      const evidenceRequestedAtMs = Number(evidence.requestedAtMs || Date.now());
      const nextEvidence = cloneMultiplayerPayload({
        ...evidence,
        requestedAtMs: evidenceRequestedAtMs,
      });
      if (shouldReplacePendingActionIntentEvidence(record, nextEvidence)) {
        record.evidence = nextEvidence;
      }
    }
    pendingActionIntentsRef.current.set(key, record);
    rememberActionIntentObservation(key, record);
    schedulePendingActionIntentTimeout(key, record);
    // Publish the shared record and merge its deadline before yielding to the
    // worker. Concurrent progress handlers must see and extend this record.
    const observedElapsed = await observedMatchClockElapsedForIntent(verifiedIntent, record);
    if (pendingActionIntentsRef.current.get(key) !== record
      || protocolActionIntentInactiveReason(key)
      || verifiedIntent.matchId !== currentAuditMatchId()
      || Number(verifiedIntent.seq) <= Number(multiplayerRef.current.lastAppliedSequence || 0)) return verifiedIntent;
    if (observedElapsed != null) {
      record.observedElapsedAtIntentMs = Math.max(
        Number(record.observedElapsedAtIntentMs || 0),
        Number(observedElapsed || 0)
      );
      persistPendingPaymentTiming(record);
    }
    return verifiedIntent;
  }

  async function refreshPendingActionIntentEvidenceForAction(message, evidence = {}) {
    const audit = message?.audit || {};
    const key = actionIntentKey({
      matchId: audit.matchId || currentAuditMatchId(),
      seq: audit.seq ?? message?.seq,
      actorIndex: audit.actor ?? message?.actorIndex,
    });
    const record = pendingActionIntentsRef.current.get(key);
    if (!record || !evidence?.requestPayload) return false;
    if (!record.firstObservedAtMs) {
      record.firstObservedAtMs = Date.now();
    }
    const evidenceRequestedAtMs = Number(evidence.requestedAtMs || Date.now());
    const nextEvidence = cloneMultiplayerPayload({
      ...evidence,
      requestedAtMs: evidenceRequestedAtMs,
    });
    if (shouldReplacePendingActionIntentEvidence(record, nextEvidence)) {
      record.evidence = nextEvidence;
      pendingActionIntentsRef.current.set(key, record);
      schedulePendingActionIntentTimeout(key, record);
    }
    return true;
  }

  function extendZiffleRevealTokenWaitersForActionIntent(intent, timeoutMs) {
    const key = actionIntentKey(intent);
    if (!key) return false;
    let extended = false;
    for (const waiter of ziffleRevealWaitersRef.current.values()) {
      if (!waiter || String(waiter.actionIntentKey || "") !== key) continue;
      if (typeof waiter.extendTimeout === "function") {
        extended = waiter.extendTimeout(timeoutMs) || extended;
      }
    }
    return extended;
  }

  function actionIntentProgressOperation(phase) {
    switch (String(phase || "")) {
      case "payload_generation":
        return "Generating action payload";
      case "engine_work":
        return "Applying engine command";
      case "crypto_material":
        return "Collecting hidden-card material";
      case "opening_generation":
        return "Building reveal proofs";
      case "opening_preview":
        return "Opening revealed card";
      case "payload_signing":
        return "Signing audit payload";
      case "action_broadcast":
        return "Broadcasting verified action";
      default:
        return "Working on action sync";
    }
  }

  function actionIntentProgressExtraFromMessage(message = {}) {
    const extra = {};
    for (const key of ["operation", "detail", "cardName", "card_name", "zone", "title", "description"]) {
      if (message[key] == null) continue;
      const normalizedKey = key === "card_name" ? "cardName" : key;
      extra[normalizedKey] = String(message[key] || "");
    }
    const progressCurrent = Number(message.progressCurrent ?? message.progress_current);
    const progressTotal = Number(message.progressTotal ?? message.progress_total);
    if (Number.isFinite(progressCurrent)) extra.progressCurrent = progressCurrent;
    if (Number.isFinite(progressTotal)) extra.progressTotal = progressTotal;
    const openingPreview = normalizeActionOpeningPreview(
      message.openingPreview || message.opening_preview
    );
    if (openingPreview) {
      extra.openingPreview = openingPreview;
      extra.cardName = extra.cardName || openingPreview.card;
      extra.zone = extra.zone || openingPreview.zone;
    }
    return extra;
  }

  function previewActionIntentOpeningInInspector(actionIntentKeyValue, preview, progress = {}) {
    const normalizedPreview = normalizeActionOpeningPreview(preview);
    const key = String(actionIntentKeyValue || "");
    if (!key || !normalizedPreview) return;
    const previewKey = [
      normalizedPreview.owner,
      normalizedPreview.slot ?? "",
      normalizedPreview.objectId ?? "",
      normalizedPreview.stableId ?? "",
      normalizedPreview.position ?? "",
      normalizedPreview.zone || "",
      normalizedPreview.card || "",
    ].join(":");
    let seen = actionIntentOpeningPreviewKeysRef.current.get(key);
    if (!seen) {
      seen = new Set();
      actionIntentOpeningPreviewKeysRef.current.set(key, seen);
    }
    if (seen.has(previewKey)) return;
    seen.add(previewKey);
    previewAuditOpeningInInspector(normalizedPreview, stateRef.current, {
      previewIndex: progress.progressCurrent == null
        ? undefined
        : Math.max(0, Number(progress.progressCurrent) - 1),
      previewTotal: progress.progressTotal,
      previewZone: normalizedPreview.zone,
    });
  }

  function showActionIntentProgressWait(intent, phase, responseTimeoutMs, extra = {}) {
    if (!intent) return;
    const payload = signedActionIntentPayload(intent);
    const key = actionIntentKey(payload);
    const actorName = playerNameForIndex(multiplayerRef.current.players, payload.actorIndex);
    if (servicesRef.current.isRecoveringSequencedActions?.()) return;
    const operation = actionIntentProgressOperation(phase);
    const requestId = `action-progress:${key}`;
    const patch = {
      kind: "action_progress",
      requestId,
      actionIntentKey: key,
      peerIndex: Number(payload.actorIndex),
      peerName: actorName,
      title: `${actorName} is syncing an action`,
      description:
        `${actorName}'s browser is ${operation.toLowerCase()} for action ${Number(payload.seq)}. `
        + "The game will continue after that payload is verified.",
      phase: String(phase || ""),
      operation,
      responseTimeoutMs,
      ...extra,
    };
    if (updatePeerWaitForActionIntent(key, patch)) return;
    beginPeerWait(patch);
  }

  async function handleActionIntentProgressMessage(message) {
    if (!message?.actionIntent) return;
    const messageIntentKey = actionIntentKeyFromProtocolPayload(message);
    const inactiveReason = protocolActionIntentInactiveReason(messageIntentKey, message.actionIntent);
    if (inactiveReason) {
      recordPeerSyncPerf("action_intent_progress:ignored", {
        request_id: String(message.requestId || ""),
        phase: String(message.phase || ""),
        reason: inactiveReason,
        action_intent_key: messageIntentKey,
      });
      return;
    }
    const requestPayload = cloneMultiplayerPayload(message);
    const phase = String(message.phase || "");
    const actionPayload = message.action || message.applyAction || message.apply_action || null;
    const phaseDefaultTimeoutMs = phase === "action_broadcast"
      ? actionBroadcastResponseTimeoutMs(actionPayload)
      : ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD;
    const advertisedTimeoutMs = Number(message.responseTimeoutMs ?? message.response_timeout_ms);
    const responseTimeoutMs = Number.isFinite(advertisedTimeoutMs) && advertisedTimeoutMs > 0
      ? Math.max(Math.floor(advertisedTimeoutMs), phaseDefaultTimeoutMs)
      : phaseDefaultTimeoutMs;
    const verifiedIntent = await rememberPendingActionIntent(message.actionIntent, {
      requestType: "action_intent_progress",
      requestId: String(message.requestId || ""),
      requestPayload,
      requestPayloadHash: await sha256Hex(canonicalMultiplayerPayload(requestPayload)),
      responseTimeoutMs,
      requestedAtMs: Date.now(),
    });
    if (protocolActionIntentInactiveReason(messageIntentKey)
      || !pendingActionIntentsRef.current.has(messageIntentKey)
      || verifiedIntent.matchId !== currentAuditMatchId()
      || Number(verifiedIntent.seq) <= Number(multiplayerRef.current.lastAppliedSequence || 0)) return;
    if (
      message.senderIndex != null
      && Number(message.senderIndex) !== Number(verifiedIntent.actorIndex)
    ) {
      recordPeerSyncPerf("action_intent_progress:ignored", {
        request_id: String(message.requestId || ""),
        phase,
        reason: "sender_actor_mismatch",
        sender: Number(message.senderIndex),
        actor: Number(verifiedIntent.actorIndex),
      });
      return;
    }
    const extendedRevealWaiters = extendZiffleRevealTokenWaitersForActionIntent(
      message.actionIntent,
      responseTimeoutMs
    );
    const progressExtra = actionIntentProgressExtraFromMessage(message);
    showActionIntentProgressWait(message.actionIntent, phase, responseTimeoutMs, progressExtra);
    if (progressExtra.openingPreview) {
      previewActionIntentOpeningInInspector(messageIntentKey, progressExtra.openingPreview, progressExtra);
    }
    recordPeerSyncPerf("action_intent_progress:received", {
      request_id: String(message.requestId || ""),
      phase,
      sender: message.senderIndex == null ? null : Number(message.senderIndex),
      response_timeout_ms: responseTimeoutMs,
      bytes: payloadSizeBytes(message),
      extended_reveal_waiters: extendedRevealWaiters,
    });
    if (
      phase === "action_broadcast"
      && actionPayload
      && String(actionPayload.type || "") === "apply_action"
    ) {
      await applySequencedActionMessage(cloneMultiplayerPayload(actionPayload));
    }
  }

  async function handleActionIntentCancelMessage(message) {
    if (!message?.actionIntent) return;
    const verifiedIntent = await verifySignedActionIntent(message.actionIntent);
    // Only the intent's actor may withdraw it: the intent itself is visible to
    // every peer, so a cancel must carry the actor's own signature over it.
    const cancelPayload = actionIntentCancelPayload(verifiedIntent, message.reason);
    const actorKey = await importCachedAuditPublicKey(publicKeyForAuditSigner(verifiedIntent.actorIndex));
    if (!(await verifyAuditPayload(actorKey, cancelPayload, message.cancelSignature || ""))) {
      throw new Error("Action intent cancel is not signed by the intent's actor");
    }
    const key = actionIntentKey(verifiedIntent);
    // A cancel only drops the pending record. If this peer already disclosed
    // hidden material for the intent, the disclosure lock (validation.js
    // fairRandomRevealLockKey) survives it and still pins the sequence to the
    // intent's command, so peeking and then substituting another action fails.
    if (pinnedPaymentDisclosure(verifiedIntent)) {
      setStatus("A disclosed payment is awaiting retry of its committed command", true);
      return;
    }
    rememberIgnoredActionIntentKey(key, String(message.reason || "action_intent_cancel"), verifiedIntent);
    const active = pendingActionIntentsRef.current.get(key);
    const hadPending = active?.fingerprint === actionIntentFingerprint(verifiedIntent);
    if (hadPending) {
      markActionIntentObservationCancelled(key, message.senderIndex);
      clearPendingActionIntent(key);
      await servicesRef.current.cancelOptimisticIntent?.(verifiedIntent);
    }
    recordPeerSyncPerf("action_intent_cancel:received", {
      request_id: String(message.requestId || ""),
      sender: message.senderIndex == null ? null : Number(message.senderIndex),
      seq: Number(verifiedIntent.seq || 0),
      actor: Number(verifiedIntent.actorIndex ?? -1),
      cleared: hadPending,
      reason: String(message.reason || ""),
    });
  }

  function broadcastActionIntentProgress(
    actionIntent,
    phase = "payload_generation",
    responseTimeoutMs = null,
    extraPayload = null
  ) {
    if (!actionIntent) return false;
    const session = multiplayerRef.current;
    const advertisedTimeoutMs = Number(responseTimeoutMs);
    const normalizedPhase = String(phase || "payload_generation");
    const localExtensionMs = Number.isFinite(advertisedTimeoutMs) && advertisedTimeoutMs > 0
      ? Math.floor(advertisedTimeoutMs)
      : normalizedPhase === "action_broadcast"
        ? actionBroadcastResponseTimeoutMs(extraPayload?.action || extraPayload?.applyAction)
        : ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD;
    const extendedLocalRevealWaiters = extendZiffleRevealTokenWaitersForActionIntent(
      actionIntent,
      localExtensionMs
    );
    const payload = {
      type: "action_intent_progress",
      protocolVersion: PROTOCOL_VERSION,
      requestId: makeZiffleRequestId("action-progress"),
      senderPeerId: String(session.localPeerId || ""),
      senderIndex: resolveLocalPlayerIndex(session),
      phase: normalizedPhase,
      actionIntent: cloneMultiplayerPayload(actionIntent),
      at: Date.now(),
    };
    if (extraPayload && typeof extraPayload === "object") {
      Object.assign(payload, cloneMultiplayerPayload(extraPayload));
    }
    if (Number.isFinite(advertisedTimeoutMs) && advertisedTimeoutMs > 0) {
      payload.responseTimeoutMs = Math.floor(advertisedTimeoutMs);
    }
    let sent = false;
    for (const player of session.players || []) {
      const peerId = routePeerIdForPlayer(player);
      if (!peerId || peerId === session.localPeerId) continue;
      sent = sendDirectPeerMessage(peerId, payload) || sent;
    }
    recordPeerSyncPerf("action_intent_progress:sent", {
      request_id: payload.requestId,
      phase: payload.phase,
      sent,
      extended_reveal_waiters: extendedLocalRevealWaiters,
    });
    return sent;
  }

  function actionIntentCancelPayload(actionIntent, reason = "") {
    return {
      domain: "ironsmith.action_intent_cancel.v1",
      matchId: String(actionIntent.matchId || ""),
      seq: Number(actionIntent.seq || 0),
      actorIndex: Number(actionIntent.actorIndex ?? -1),
      intentSignature: String(actionIntent.signature || ""),
      reason: String(reason || ""),
    };
  }

  async function broadcastActionIntentCancel(actionIntent, reason = "") {
    if (!actionIntent) return false;
    const session = multiplayerRef.current;
    const { keyPair } = await ensureAuditIdentity();
    const cancelSignature = await signAuditPayload(keyPair, actionIntentCancelPayload(actionIntent, reason));
    const payload = {
      type: "action_intent_cancel",
      cancelSignature,
      protocolVersion: PROTOCOL_VERSION,
      requestId: makeZiffleRequestId("action-cancel"),
      senderPeerId: String(session.localPeerId || ""),
      senderIndex: resolveLocalPlayerIndex(session),
      actionIntent: cloneMultiplayerPayload(actionIntent),
      reason: String(reason || ""),
      at: Date.now(),
    };
    let sent = false;
    for (const player of session.players || []) {
      const peerId = routePeerIdForPlayer(player);
      if (!peerId || peerId === session.localPeerId) continue;
      sent = sendDirectPeerMessage(peerId, payload) || sent;
    }
    recordPeerSyncPerf("action_intent_cancel:sent", {
      request_id: payload.requestId,
      sent,
      reason: payload.reason,
    });
    return sent;
  }

  function startActionIntentProgressBroadcast(
    actionIntent,
    phase = "payload_generation",
    responseTimeoutMs = null,
    extraPayload = null
  ) {
    if (!actionIntent) return null;
    let currentPhase = phase;
    let currentResponseTimeoutMs = responseTimeoutMs;
    let currentExtraPayload = extraPayload;
    const sendCurrent = () => {
      broadcastActionIntentProgress(
        actionIntent,
        currentPhase,
        currentResponseTimeoutMs,
        currentExtraPayload
      );
    };
    sendCurrent();
    const timerId = window.setInterval(() => {
      sendCurrent();
    }, Math.max(250, Math.floor(ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD / 2)));
    const stop = () => window.clearInterval(timerId);
    stop.update = (
      nextPhase = currentPhase,
      nextResponseTimeoutMs = currentResponseTimeoutMs,
      nextExtraPayload = currentExtraPayload
    ) => {
      currentPhase = nextPhase;
      currentResponseTimeoutMs = nextResponseTimeoutMs;
      currentExtraPayload = nextExtraPayload;
      sendCurrent();
    };
    return stop;
  }

  function pendingActionIntentRecordForSequence(seq) {
    const matchId = currentAuditMatchId();
    const targetSeq = Number(seq);
    if (!Number.isSafeInteger(targetSeq) || targetSeq <= 0) return null;
    for (const [key, record] of pendingActionIntentsRef.current.entries()) {
      const intent = record?.intent;
      if (!intent) continue;
      const payload = signedActionIntentPayload(intent);
      if (
        payload.matchId === matchId
        && Number(payload.seq) === targetSeq
      ) {
        return { key, record, payload };
      }
    }
    return null;
  }

  async function waitForPendingActionIntentBeforeLocalSubmit(seq, command = null) {
    if (command && isForfeitCommand(command)) return true;
    const targetSeq = Number(seq);
    if (!Number.isSafeInteger(targetSeq) || targetSeq <= 0) return true;
    const startedAtMs = Date.now();
    while (Date.now() - startedAtMs < MAX_PENDING_ACTION_INTENT_MS + MATCH_CLOCK_CLAIM_SKEW_MS) {
      const currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
      if (currentSequence >= targetSeq) return false;
      const pending = pendingActionIntentRecordForSequence(targetSeq);
      if (!pending) return true;
      if (command && Number(pending.payload.actorIndex) === Number(resolveLocalPlayerIndex(multiplayerRef.current))
        && pinnedPaymentDisclosure(pending.record.intent)) {
        assertPaymentDisclosureIntent({ ...pending.record.intent, command });
        return true;
      }
      if (matchingAppliedActionForIntent(pending.record.intent)) {
        clearPendingActionIntent(pending.key);
        return false;
      }
      const dueAtMs = pendingActionIntentDueAtMs(pending.record);
      const actorName = playerNameForIndex(
        multiplayerRef.current.players,
        pending.payload.actorIndex
      );
      setStatus(`Waiting for ${actorName}'s action payload`);
      if (Date.now() >= dueAtMs) {
        await handlePendingActionIntentTimeout(pending.key);
        return false;
      }
      await sleep(Math.min(250, Math.max(1, Math.ceil(dueAtMs - Date.now()))));
    }
    return false;
  }

  async function handlePendingActionIntentTimeout(key, scheduledAtMs = null) {
    const record = pendingActionIntentsRef.current.get(key);
    if (!record || record.timeoutClaimPending || protocolActionIntentInactiveReason(key)) return;
    const intent = record.intent || {};
    if (intent.matchId !== currentAuditMatchId()) return;
    if (matchingAppliedActionForIntent(intent)) {
      clearPendingActionIntent(key);
      return;
    }
    const seq = Number(intent.seq || 0);
    const currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
    if (seq <= currentSequence) {
      clearPendingActionIntent(key);
      return;
    }
    if (seq !== currentSequence + 1) {
      return;
    }
    const dueAtMs = pendingActionIntentDueAtMs(record);
    if (Date.now() < dueAtMs) {
      record.timeoutConfirmation = null;
      schedulePendingActionIntentTimeout(key, record);
      return;
    }
    const nowMs = Date.now();
    const confirmation = record.timeoutConfirmation;
    const expectedCheckMs = scheduledAtMs ?? confirmation?.notBeforeMs ?? dueAtMs;
    const schedulerWasLate = nowMs - expectedCheckMs > MATCH_CLOCK_CLAIM_SKEW_MS;
    if (!confirmation || confirmation.dueAtMs !== dueAtMs || schedulerWasLate) {
      // Let queued messages reach their verified deadline updates before
      // attributing silence to a peer. Repeated local suspension may postpone
      // observation, but never extends the signed evidence or its hard cap.
      const recoveryMs = schedulerWasLate
        ? ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD + MATCH_CLOCK_CLAIM_SKEW_MS
        : MATCH_CLOCK_CLAIM_SKEW_MS;
      record.timeoutConfirmation = { dueAtMs, notBeforeMs: nowMs + recoveryMs };
      recordPeerSyncPerf("action_intent_timeout:catch_up", {
        seq, actor: intent.actorIndex, scheduler_delay_ms: Math.max(0, nowMs - expectedCheckMs),
        recovery_ms: recoveryMs,
      });
      schedulePendingActionIntentTimeout(key, record);
      return;
    }
    if (nowMs < confirmation.notBeforeMs) {
      schedulePendingActionIntentTimeout(key, record);
      return;
    }
    const evidenceAtCheck = record.evidence;
    record.timeoutClaimPending = true;
    try {
      const hardDueAtMs = pendingActionIntentHardDueAtMs(record);
      const evidenceDueAtMs = pendingActionIntentEvidenceDueAtMs(record.evidence || {});
      const evidence = hardDueAtMs <= evidenceDueAtMs
        ? await pendingActionIntentHardTimeoutEvidence(key, record)
        : (record.evidence || {});
      const timeoutMs = pendingActionIntentEvidenceTimeoutMs(evidence);
      const requestedAtMs = pendingActionIntentEvidenceRequestedAtMs(evidence);
      const targetPlayerIndex = normalizePlayerIndex(intent.actorIndex);
      if (targetPlayerIndex == null) return;
      const target = playerForProtocolResponseTimeout(targetPlayerIndex);
      const requestPayload = cloneMultiplayerPayload(evidence.requestPayload || {});
      const requestPayloadHash = String(
        evidence.requestPayloadHash
        || await sha256Hex(canonicalMultiplayerPayload(requestPayload))
      );
      // Hashing can yield long enough for progress, application, cancellation,
      // or a different match. Do not submit a claim built from stale evidence.
      if (pendingActionIntentsRef.current.get(key) !== record
        || protocolActionIntentInactiveReason(key)
        || intent.matchId !== currentAuditMatchId()
        || Number(multiplayerRef.current.lastAppliedSequence || 0) !== currentSequence
        || matchingAppliedActionForIntent(intent)) return;
      if (record.evidence !== evidenceAtCheck || pendingActionIntentDueAtMs(record) !== dueAtMs) {
        schedulePendingActionIntentTimeout(key, record);
        return;
      }
      if (Date.now() - nowMs > MATCH_CLOCK_CLAIM_SKEW_MS) {
        record.timeoutConfirmation = {
          dueAtMs,
          notBeforeMs: Date.now() + ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS_PER_CARD + MATCH_CLOCK_CLAIM_SKEW_MS,
        };
        schedulePendingActionIntentTimeout(key, record);
        return;
      }
      recordPeerSyncPerf("action_intent_timeout:confirmed", {
        seq, actor: intent.actorIndex, basis_sequence: currentSequence,
        request_type: evidence.requestType, request_id: evidence.requestId,
        response_timeout_ms: timeoutMs, requested_at_ms: requestedAtMs,
      });
      await submitProtocolResponseTimeoutClaim({
        matchId: currentAuditMatchId(),
        basisSequence: currentSequence,
        targetPlayerIndex,
        targetPeerId: String(target?.peerId || evidence.actorPeerId || ""),
        targetName: target?.name || `Player ${targetPlayerIndex + 1}`,
        requesterIndex: resolveLocalPlayerIndex(multiplayerRef.current),
        requestType: String(evidence.requestType || requestPayload.type || "action_intent"),
        requestId: String(evidence.requestId || requestPayload.requestId || ""),
        requestPayloadHash,
        requestPayload,
        responseTimeoutMs: timeoutMs,
        requestedAtMs,
      });
    } finally {
      record.timeoutClaimPending = false;
    }
  }

  async function verifyActionMatchesPendingIntent(message) {
    const audit = message?.audit || {};
    const key = actionIntentKey({
      matchId: audit.matchId || currentAuditMatchId(),
      seq: audit.seq ?? message?.seq,
      actorIndex: audit.actor ?? message?.actorIndex,
    });
    const record = pendingActionIntentsRef.current.get(key);
    if (!record) return;
    const intent = record.intent || {};
    const expectedPreActionPublicCheckpointHash =
      String(intent.preActionPublicCheckpointHash || "");
    if (expectedPreActionPublicCheckpointHash) {
      await verifyCurrentPublicCheckpointHash(
        expectedPreActionPublicCheckpointHash,
        "Signed action intent public checkpoint does not match local state"
      );
    }
    const expected = signedActionIntentPayload(intent);
    if (
      String(audit.matchId || "") !== expected.matchId
      || Number(audit.seq) !== Number(expected.seq)
      || Number(audit.actor) !== Number(expected.actorIndex)
      || String(audit.prevStateHash || "") !== expected.prevStateHash
      || canonicalMultiplayerPayload(message?.command) !== canonicalMultiplayerPayload(expected.command)
      || canonicalMultiplayerPayload(audit.command) !== canonicalMultiplayerPayload(expected.command)
    ) {
      throw new Error("Sequenced action conflicts with an earlier signed action intent");
	    }
	    const observedElapsed = Number(record.observedElapsedAtIntentMs || 0);
	    const clockElapsed = Number(audit.clock?.elapsedMs || 0);
    const intentWasHeldForCryptoMaterial = pendingActionIntentHeldForProtocolWork(record);
	    if (
	      !intentWasHeldForCryptoMaterial
	      && observedElapsed > 0
	      && clockElapsed + MATCH_CLOCK_CLAIM_SKEW_MS < observedElapsed
	    ) {
      throw new Error("Sequenced action match clock is below its signed action intent observation");
    }
    clearPendingActionIntent(key);
    return {
      intentWasHeldForCryptoMaterial,
      observedElapsedAtIntentMs: observedElapsed,
    };
	  }

  async function signReconnectProofForChallenge(challenge) {
    const { keyPair } = await ensureAuditIdentity();
    const payload = reconnectProofPayload({
      matchId: challenge.matchId,
      challengeId: challenge.requestId || challenge.challengeId,
      nonce: challenge.nonce,
      playerIndex: challenge.playerIndex,
      peerId: challenge.peerId || multiplayerRef.current.localPeerId,
      hostPeerId: challenge.hostPeerId,
      transcriptHash: challenge.transcriptHash,
    });
    return {
      ...payload,
      signatureAlgorithm: "ecdsa-p256-sha256",
      signature: await signAuditPayload(keyPair, payload),
    };
  }

  async function verifyReconnectProofForChallenge(proof, challenge, auditPublicKey) {
    if (!proof || typeof proof !== "object") {
      throw new Error("Reconnect response is missing audit-key proof");
    }
    const payload = reconnectProofPayload(proof);
    const expected = reconnectProofPayload({
      matchId: challenge.matchId,
      challengeId: challenge.requestId,
      nonce: challenge.nonce,
      playerIndex: challenge.playerIndex,
      peerId: challenge.peerId,
      hostPeerId: challenge.hostPeerId,
      transcriptHash: challenge.transcriptHash,
    });
    if (canonicalMultiplayerPayload(payload) !== canonicalMultiplayerPayload(expected)) {
      throw new Error("Reconnect proof does not match the host challenge");
    }
    const publicKey = await importCachedAuditPublicKey(auditPublicKey);
    const valid = await verifyAuditPayload(publicKey, payload, proof.signature || "");
    if (!valid) {
      throw new Error("Reconnect proof signature is invalid");
    }
  }


  return { pinBlindExileOpeningIntent, pinVerifiedPaymentEnvelope, restorePaymentDisclosureAtHead, assertPaymentDisclosureIntent, pinnedPaymentDisclosure, pinPaymentDisclosureIntent, acceptPaymentDisclosure, paymentDisclosureForCommand, assertLocalProtocolTimeoutObservation, handleProtocolWaitAnswerMessage, handleProtocolWaitNoticeMessage, localProtocolTimeoutContradiction, observedProtocolWaitMs, openProtocolWaitsForRequester, protocolResponseConn, IGNORED_ACTION_INTENT_TTL_MS, MAX_IGNORED_ACTION_INTENTS, actionBroadcastResponseTimeoutMs, actionIntentKeyFromProtocolClaim, actionIntentKeyFromProtocolPayload, actionIntentProgressExtraFromMessage, actionIntentProgressOperation, auditEncryptionPublicKeyForPlayer, beginPeerWait, broadcastActionIntentCancel, broadcastActionIntentProgress, cachedZiffleRevealTokens, clearAllConnectionHeartbeats, clearAllPendingActionIntents, clearConnectionHeartbeat, clearOwnerZiffleOpeningCache, clearPeerWait, clearPeerWaitForActionIntent, clearPendingActionIntent, currentAuditMatchId, emitZiffleDiagnosticNotice, ensureAuditIdentity, ensureDirectPeerConnections, ensureZiffleIdentity, ensureZiffleOpeningProof, extendZiffleRevealTokenWaitersForActionIntent, handleActionIntentCancelMessage, handleActionIntentProgressMessage, handleConnectionHeartbeatMessage, handlePendingActionIntentTimeout, hydrateZiffleCeremonyForLookup, ignoreAndClearAllPendingActionIntents, ignoredActionIntentReason, importCachedAuditPublicKey, isDirectProtocolMessage, localRevealedOpeningForExport, localRevealedOpeningForRequirement, localRevealedOpeningForZiffleReveal, localZiffleDiagnostics, makeProtocolResponseTimeoutError, makeZiffleRequestId, markConnectionAlive, matchPayloadCeremoniesForLookup, matchingAppliedActionForIntent, normalizeZiffleRevealToken, observedMatchClockElapsedForIntent, openingNeedsZiffleProof, pendingActionIntentDueAtMs, pendingActionIntentEvidenceDueAtMs, pendingActionIntentEvidenceRequestedAtMs, pendingActionIntentEvidenceTimeoutMs, pendingActionIntentFirstObservedAtMs, pendingActionIntentHardDueAtMs, pendingActionIntentHardTimeoutEvidence, pendingActionIntentHeldForProtocolWork, pendingActionIntentRecordForSequence, pendingActionIntentSuppressesHeartbeatStale, previewActionIntentOpeningInInspector, privateDeckManifestForOwner, protocolActionIntentInactiveReason, pruneIgnoredActionIntents, publicDeckManifestForOwner, publicKeyForAuditSigner, publicZiffleKey, refreshPendingActionIntentEvidenceForAction, rememberIgnoredActionIntentKey, rememberLocalRevealedOpening, rememberLocalZiffleCeremonyForLookup, rememberPendingActionIntent, rememberPrivateDeckManifest, rememberPrivateViewDisclosure, rememberZiffleOpeningPosition, rememberZiffleRevealTokens, resolveActionQuorumVote, resolveCryptoMaterial, resolveLocalCryptoPlayerIndex, resolveRngCommit, resolveRngReveal, resolveSubmissionIdleWaiters, resolveTimeoutVote, resolveZiffleRevealToken, resolveZiffleShuffleStep, runtimeManifestForZiffleCeremony, schedulePendingActionIntentTimeout, shouldReplacePendingActionIntentEvidence, shouldSuppressProtocolMessageError, showActionIntentProgressWait, signActionIntentForCommand, signPlayerGenesis, signReconnectProofForChallenge, signedZiffleKeysForPayload, startActionIntentProgressBroadcast, startConnectionHeartbeat, updateMultiplayer, updatePeerWait, updatePeerWaitForActionIntent, verifyActionMatchesPendingIntent, verifyReconnectProofForChallenge, verifySignedActionIntent, verifyZiffleOpeningCryptographicProof, verifyZiffleOpeningProofForOpening, waitForActionQuorumVote, waitForCryptoMaterial, waitForPendingActionIntentBeforeLocalSubmit, waitForProtocolResponse, waitForRngCommit, waitForRngReveal, waitForSubmissionIdle, waitForTimeoutVote, waitForZiffleRevealToken, waitForZiffleShuffleStep, ziffleCeremonyCandidatesForOwner, ziffleCeremonyForOwner, ziffleCeremonyHasObjectOrder, ziffleObjectOrderLinksOpening, ziffleOpeningPositionForSlot, ziffleOpeningProofHasAuthenticatedObjectOrder, zifflePositionForObjectId, zifflePositionForOriginalSlot, zifflePublicKeysForPlayers, ziffleRevealMatchesOpening, ziffleRevealTokenCacheKey, ziffleShuffleObjectIdForPosition, ziffleShuffleOriginalSlotForPosition, ziffleTokensForPosition };
}
