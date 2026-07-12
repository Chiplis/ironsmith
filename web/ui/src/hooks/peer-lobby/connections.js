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
} from "./shared.js";

export function usePeerLobbyConnections(base, servicesRef) {
  const { actionIntentOpeningPreviewKeysRef, actionQuorumVoteWaitersRef, actionSubmissionStartedAtMsRef, auditEncryptionKeyPairRef, auditEncryptionPublicKeyRef, auditKeyPairRef, auditPublicKeyRef, auditVerifyKeyCacheRef, connectionHeartbeatsRef, cryptoMaterialWaitersRef, ensureDirectPeerConnectionsRef, gameRef, ignoredActionIntentKeysRef, liveZiffleCeremoniesRef, localRevealedOpeningsRef, localZiffleCeremonyLookupRef, matchClockConfigRef, matchStartPayloadRef, multiplayerRef, peerHeartbeatConfigRef, pendingActionIntentTimeoutsRef, pendingActionIntentsRef, privateDeckManifestsRef, privateViewDisclosuresRef, rngCommitWaitersRef, rngRevealWaitersRef, setMultiplayer, setStatus, stateRef, submissionIdleWaitersRef, timeoutVoteWaitersRef, ziffleHandRevealKeyRef, ziffleHandRevealQuickKeyRef, ziffleKeyPairsRef, ziffleOpeningPositionsRef, ziffleRevealTokenCacheRef, ziffleRevealWaitersRef, ziffleShuffleWaitersRef } = base;
  const actionHistoryEntryForSequence = useCallback((...args) => servicesRef.current.actionHistoryEntryForSequence(...args), [servicesRef]);
  const applySequencedActionMessage = useCallback((...args) => servicesRef.current.applySequencedActionMessage(...args), [servicesRef]);
  const collectZiffleRevealTokens = useCallback((...args) => servicesRef.current.collectZiffleRevealTokens(...args), [servicesRef]);
  const playerForProtocolResponseTimeout = useCallback((...args) => servicesRef.current.playerForProtocolResponseTimeout(...args), [servicesRef]);
  const previewAuditOpeningInInspector = useCallback((...args) => servicesRef.current.previewAuditOpeningInInspector(...args), [servicesRef]);
  const resolveCommittedZiffleRevealSlot = useCallback((...args) => servicesRef.current.resolveCommittedZiffleRevealSlot(...args), [servicesRef]);
  const routePeerIdForPlayer = useCallback((...args) => servicesRef.current.routePeerIdForPlayer(...args), [servicesRef]);
  const sendDirectPeerMessage = useCallback((...args) => servicesRef.current.sendDirectPeerMessage(...args), [servicesRef]);
  const submitProtocolResponseTimeoutClaim = useCallback((...args) => servicesRef.current.submitProtocolResponseTimeoutClaim(...args), [servicesRef]);
  const updateMatchClockForState = useCallback((...args) => servicesRef.current.updateMatchClockForState(...args), [servicesRef]);
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

  const startConnectionHeartbeat = useCallback((key, conn, onStale) => {
    clearConnectionHeartbeat(key);
    const { intervalMs, timeoutMs } = peerHeartbeatConfigRef.current;
    if (!intervalMs || !timeoutMs) return;

    const heartbeat = {
      lastSeen: Date.now(),
      timer: window.setInterval(() => {
        if (!conn || conn.open === false) {
          clearConnectionHeartbeat(key);
          onStale?.("Connection closed");
          return;
        }

        const nowMs = Date.now();
        if (
          nowMs - heartbeat.lastSeen > timeoutMs
          && !pendingActionIntentSuppressesHeartbeatStale(nowMs)
        ) {
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
        });
      }, intervalMs),
    };
    connectionHeartbeatsRef.current.set(key, heartbeat);
  }, [clearConnectionHeartbeat]);

  const handleConnectionHeartbeatMessage = useCallback((conn, message) => {
    if (message?.type === "peer_heartbeat_ack") {
      return message.protocolVersion === PROTOCOL_VERSION;
    }
    if (message?.type !== "peer_heartbeat") return false;
    if (message.protocolVersion !== PROTOCOL_VERSION) return false;
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
    setMultiplayer(normalized);
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

  const runtimeManifestForZiffleCeremony = useCallback((manifest, ceremony) => {
    const deckCount = Number(ceremony?.deckCount || manifest?.deckCount || 0);
    const baseManifest = publicDeckManifest(manifest) || {};
    return {
      ...baseManifest,
      deckCount,
      commitmentRoot: `ziffle:${String(ceremony?.deckHash || "")}`,
      slotCommitments: Array.from({ length: deckCount }, (_, position) => ({
        slot: position,
        commitment: ziffleRuntimeCommitment(ceremony.deckHash, position),
      })),
    };
  }, []);

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
    try {
      return await waiter;
    } catch (err) {
      if (!isProtocolResponseWaitTimeout(err)) throw err;
      const timing = err?.protocolResponseTimeoutTiming || {};
      throw await makeProtocolResponseTimeoutError(err, {
        ...claim,
        ...(timing.responseTimeoutMs != null ? { responseTimeoutMs: timing.responseTimeoutMs } : {}),
        ...(timing.requestedAtMs != null ? { requestedAtMs: timing.requestedAtMs } : {}),
      });
    }
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

	  const clearOwnerZiffleOpeningCache = useCallback((owner, matchId = currentAuditMatchId()) => {
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
      if (key.startsWith(`${normalizedOwner}:`)) {
        ziffleRevealTokenCacheRef.current.delete(key);
      }
	    }
	    const normalizedMatchId = String(matchId || "");
	    for (const [key, opening] of [...localRevealedOpeningsRef.current.entries()]) {
	      if (
	        key.startsWith(`${normalizedMatchId}:`)
	        && Number(opening?.owner) === normalizedOwner
	      ) {
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
      multiplayerRef.current.players.find(
        (player) => Number(player.index) === normalized
      )?.deckAuditManifest
    );
  }, []);

  const publicKeyForAuditSigner = useCallback((signerIndex) => {
    const normalized = normalizePlayerIndex(signerIndex);
    if (normalized == null) return "";
    const player = multiplayerRef.current.players.find(
      (entry) => Number(entry.index) === normalized
    );
    return String(player?.auditPublicKey || "");
  }, []);

  const auditEncryptionPublicKeyForPlayer = useCallback((playerIndex) => {
    const normalized = normalizePlayerIndex(playerIndex);
    if (normalized == null) return "";
    const player = (matchStartPayloadRef.current?.players || multiplayerRef.current.players || []).find(
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
	    if (
	      Number(opening.slot) !== Number(revealOriginalSlot)
	      || !ziffleRevealMatchesOpening(ceremony, revealOriginalSlot, position, opening)
	    ) {
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
    const { keyPair } = await ensureAuditIdentity();
    const payload = signedActionIntentPayload({
      matchId: currentAuditMatchId(),
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
    });
    if (
      payload.domain !== ACTION_INTENT_DOMAIN
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

  function rememberIgnoredActionIntentKey(actionIntentKeyValue, reason = "") {
    const key = String(actionIntentKeyValue || "");
    if (!key) return false;
    const nowMs = Date.now();
    ignoredActionIntentKeysRef.current.set(key, {
      reason: String(reason || ""),
      at: nowMs,
    });
    pruneIgnoredActionIntents(nowMs);
    clearPeerWaitForActionIntent(key);
    return true;
  }

  function ignoredActionIntentReason(actionIntentKeyValue) {
    const key = String(actionIntentKeyValue || "");
    if (!key) return "";
    pruneIgnoredActionIntents();
    return String(ignoredActionIntentKeysRef.current.get(key)?.reason || "");
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

  function protocolActionIntentInactiveReason(actionIntentKeyValue = "") {
    const ignoredReason = ignoredActionIntentReason(actionIntentKeyValue);
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
    const evidenceRequestedAtMs = Number(evidence.requestedAtMs || Date.now());
    const previousRequestedAtMs = Number(record.evidence.requestedAtMs || 0);
    if (evidenceRequestedAtMs < previousRequestedAtMs) return false;
    const evidenceDueAtMs = pendingActionIntentEvidenceDueAtMs(evidence);
    const previousDueAtMs = pendingActionIntentEvidenceDueAtMs(record.evidence);
    return evidenceDueAtMs >= previousDueAtMs || Date.now() >= previousDueAtMs;
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
    const existingTimeoutId = pendingActionIntentTimeoutsRef.current.get(key);
    if (existingTimeoutId) {
      window.clearTimeout(existingTimeoutId);
      pendingActionIntentTimeoutsRef.current.delete(key);
    }
    const dueAtMs = pendingActionIntentDueAtMs(record);
    if (!Number.isFinite(dueAtMs)) return;
    const delayMs = Math.max(1, Math.ceil(dueAtMs - Date.now()));
    const timeoutId = window.setTimeout(() => {
      pendingActionIntentTimeoutsRef.current.delete(key);
      void handlePendingActionIntentTimeout(key).catch((err) => {
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

  async function observedMatchClockElapsedForIntent(intent) {
    const payload = signedActionIntentPayload(intent);
    const liveState = gameRef.current && typeof gameRef.current.uiState === "function"
      ? await gameRef.current.uiState()
      : stateRef.current;
    const snapshot = updateMatchClockForState(liveState);
    if (!snapshot.enabled || Number(snapshot.activePlayerIndex) !== Number(payload.actorIndex)) {
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
    const inactiveReason = protocolActionIntentInactiveReason(key);
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
    if (matchingAppliedActionForIntent(verifiedIntent)) {
      return verifiedIntent;
    }
    const record = existing || {
      intent: cloneMultiplayerPayload(verifiedIntent),
      fingerprint,
      evidence: null,
      firstObservedAtMs: Date.now(),
      observedElapsedAtIntentMs: null,
    };
    if (!record.firstObservedAtMs) {
      record.firstObservedAtMs = Date.now();
    }
    const observedElapsed = await observedMatchClockElapsedForIntent(verifiedIntent);
    if (observedElapsed != null) {
      record.observedElapsedAtIntentMs = Math.max(
        Number(record.observedElapsedAtIntentMs || 0),
        Number(observedElapsed || 0)
      );
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
    schedulePendingActionIntentTimeout(key, record);
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
    const inactiveReason = protocolActionIntentInactiveReason(messageIntentKey);
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
    const key = actionIntentKey(verifiedIntent);
    rememberIgnoredActionIntentKey(key, String(message.reason || "action_intent_cancel"));
    const hadPending = pendingActionIntentsRef.current.has(key);
    if (hadPending || matchingAppliedActionForIntent(verifiedIntent)) {
      clearPendingActionIntent(key);
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

  function broadcastActionIntentCancel(actionIntent, reason = "") {
    if (!actionIntent) return false;
    const session = multiplayerRef.current;
    const payload = {
      type: "action_intent_cancel",
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

  async function waitForPendingActionIntentBeforeLocalSubmit(seq) {
    const targetSeq = Number(seq);
    if (!Number.isSafeInteger(targetSeq) || targetSeq <= 0) return true;
    const startedAtMs = Date.now();
    while (Date.now() - startedAtMs < MAX_PENDING_ACTION_INTENT_MS + MATCH_CLOCK_CLAIM_SKEW_MS) {
      const currentSequence = Number(multiplayerRef.current.lastAppliedSequence || 0);
      if (currentSequence >= targetSeq) return false;
      const pending = pendingActionIntentRecordForSequence(targetSeq);
      if (!pending) return true;
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

  async function handlePendingActionIntentTimeout(key) {
    const record = pendingActionIntentsRef.current.get(key);
    if (!record) return;
    const intent = record.intent || {};
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
      schedulePendingActionIntentTimeout(key, record);
      return;
    }
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


  return { IGNORED_ACTION_INTENT_TTL_MS, MAX_IGNORED_ACTION_INTENTS, actionBroadcastResponseTimeoutMs, actionIntentKeyFromProtocolClaim, actionIntentKeyFromProtocolPayload, actionIntentProgressExtraFromMessage, actionIntentProgressOperation, auditEncryptionPublicKeyForPlayer, beginPeerWait, broadcastActionIntentCancel, broadcastActionIntentProgress, cachedZiffleRevealTokens, clearAllConnectionHeartbeats, clearAllPendingActionIntents, clearConnectionHeartbeat, clearOwnerZiffleOpeningCache, clearPeerWait, clearPeerWaitForActionIntent, clearPendingActionIntent, currentAuditMatchId, emitZiffleDiagnosticNotice, ensureAuditIdentity, ensureDirectPeerConnections, ensureZiffleIdentity, ensureZiffleOpeningProof, extendZiffleRevealTokenWaitersForActionIntent, handleActionIntentCancelMessage, handleActionIntentProgressMessage, handleConnectionHeartbeatMessage, handlePendingActionIntentTimeout, hydrateZiffleCeremonyForLookup, ignoreAndClearAllPendingActionIntents, ignoredActionIntentReason, importCachedAuditPublicKey, isDirectProtocolMessage, localRevealedOpeningForExport, localRevealedOpeningForRequirement, localRevealedOpeningForZiffleReveal, localZiffleDiagnostics, makeProtocolResponseTimeoutError, makeZiffleRequestId, markConnectionAlive, matchPayloadCeremoniesForLookup, matchingAppliedActionForIntent, normalizeZiffleRevealToken, observedMatchClockElapsedForIntent, openingNeedsZiffleProof, pendingActionIntentDueAtMs, pendingActionIntentEvidenceDueAtMs, pendingActionIntentEvidenceRequestedAtMs, pendingActionIntentEvidenceTimeoutMs, pendingActionIntentFirstObservedAtMs, pendingActionIntentHardDueAtMs, pendingActionIntentHardTimeoutEvidence, pendingActionIntentHeldForProtocolWork, pendingActionIntentRecordForSequence, pendingActionIntentSuppressesHeartbeatStale, previewActionIntentOpeningInInspector, privateDeckManifestForOwner, protocolActionIntentInactiveReason, pruneIgnoredActionIntents, publicDeckManifestForOwner, publicKeyForAuditSigner, publicZiffleKey, refreshPendingActionIntentEvidenceForAction, rememberIgnoredActionIntentKey, rememberLocalRevealedOpening, rememberLocalZiffleCeremonyForLookup, rememberPendingActionIntent, rememberPrivateDeckManifest, rememberPrivateViewDisclosure, rememberZiffleOpeningPosition, rememberZiffleRevealTokens, resolveActionQuorumVote, resolveCryptoMaterial, resolveLocalCryptoPlayerIndex, resolveRngCommit, resolveRngReveal, resolveSubmissionIdleWaiters, resolveTimeoutVote, resolveZiffleRevealToken, resolveZiffleShuffleStep, runtimeManifestForZiffleCeremony, schedulePendingActionIntentTimeout, shouldReplacePendingActionIntentEvidence, shouldSuppressProtocolMessageError, showActionIntentProgressWait, signActionIntentForCommand, signPlayerGenesis, signReconnectProofForChallenge, signedZiffleKeysForPayload, startActionIntentProgressBroadcast, startConnectionHeartbeat, updateMultiplayer, updatePeerWait, updatePeerWaitForActionIntent, verifyActionMatchesPendingIntent, verifyReconnectProofForChallenge, verifySignedActionIntent, verifyZiffleOpeningProofForOpening, waitForActionQuorumVote, waitForCryptoMaterial, waitForPendingActionIntentBeforeLocalSubmit, waitForProtocolResponse, waitForRngCommit, waitForRngReveal, waitForSubmissionIdle, waitForTimeoutVote, waitForZiffleRevealToken, waitForZiffleShuffleStep, ziffleCeremonyCandidatesForOwner, ziffleCeremonyForOwner, ziffleCeremonyHasObjectOrder, ziffleObjectOrderLinksOpening, ziffleOpeningPositionForSlot, ziffleOpeningProofHasAuthenticatedObjectOrder, zifflePositionForObjectId, zifflePositionForOriginalSlot, zifflePublicKeysForPlayers, ziffleRevealMatchesOpening, ziffleRevealTokenCacheKey, ziffleShuffleObjectIdForPosition, ziffleShuffleOriginalSlotForPosition, ziffleTokensForPosition };
}
