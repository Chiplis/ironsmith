import { createOptimisticMatch } from '../../lib/optimistic-match.js';
import { calculateOptimisticAction } from '../../lib/optimistic-game-runtime.js';
import { wireStablePayload } from '../../lib/accepted-actions.js';
import { isMatchDisputed } from './match-lifecycle.js';
import {
  actionRefObjectId, canonicalMultiplayerPayload, cryptoRequirementsFromState,
  isDecisionCommandCompatible, isForfeitCommand, isNonDispatchSyncCommand,
  isTrustedMultiplayerSecurityMode, publicCheckpointHash, randomAuditHex,
  recordPeerSyncPerf, safeSend, selectObjectCandidateForId,
  selectObjectCandidateRevealPolicy, sessionSecurityMode, toErrorMessage,
  useEffect, useRef, useState, PROTOCOL_VERSION,
} from './shared.js';

// Sync metadata can be added by the signing path after calculation. It does
// not change the selected action; its meaning is checked by canonical replay.
function commandKey(command) {
  const result = { ...command };
  for (const key of ['object_id', 'object_stable_id', 'object_hidden_ref',
    'object_stable_ids', 'object_hidden_refs']) delete result[key];
  return canonicalMultiplayerPayload(result);
}

export function useOptimisticPeerState(base, servicesRef) {
  const liveBaseRef = useRef(base);
  liveBaseRef.current = base;
  const stableBaseRef = useRef(null);
  if (!stableBaseRef.current) stableBaseRef.current = new Proxy({}, {
    get: (_target, key) => liveBaseRef.current[key],
  });
  base = stableBaseRef.current;
  const { gameRef, stateRef, multiplayerRef, actionHistoryRef } = base;
  const runtimeRef = useRef(null);
  const initializingRef = useRef(null);
  const resettingRef = useRef(null);
  const verifierQueueRef = useRef(Promise.resolve());
  const controllerRef = useRef(null);
  const preparedRuntimeTokenRef = useRef(Symbol('prepared runtime'));
  const pendingTimersRef = useRef(new Map());
  const [optimistic, setOptimistic] = useState({ pending: 0, calculating: false, closed: true });
  const [waitingForMaterial, setWaitingForMaterial] = useState(false);
  const waitingForMaterialRef = useRef(false);
  const blockingCommandRef = useRef(null);
  const foregroundReadyRef = useRef(new Map());
  const visibleGame = () => base.game;
  const publish = state => { if (state) base.setState(state, { replacePresentation: true }); };
  const restoreVerified = async () => {
    const runtime = runtimeRef.current;
    if (!runtime) return;
    const restored = await runtime.copyToVisible();
    publish(restored);
  };
  if (!controllerRef.current) {
    controllerRef.current = createOptimisticMatch({
      calculate: async candidate => {
        if (candidate.preparedRuntime === preparedRuntimeTokenRef.current) {
          const state = await runtimeRef.current.copyToVisible();
          return { state, publicCheckpointHash: candidate.publicCheckpointHash };
        }
        const current = visibleGame();
        const state = await current.uiState();
        if (Number(state?.decision?.player) !== Number(candidate.actorIndex)
          || !isDecisionCommandCompatible(state?.decision, candidate.command)
          || isForfeitCommand(candidate.command)) return null;
        return calculateOptimisticAction(current, candidate, {
          prepare: async (game, entry) => {
            const allowed = commandClaimIds(entry.command, state);
            const claims = entry.publicClaims || [];
            if (!Array.isArray(claims) || claims.length > 128) throw new Error('Invalid provisional public claims');
            const canonicalClaims = (entry.calculationAudit?.openings || []).filter(opening =>
              allowed.has(Number(opening.objectId ?? opening.object_id))).map(opening => ({
              ...opening, objectId: Number(opening.objectId ?? opening.object_id),
            }));
            for (const claim of [...claims, ...canonicalClaims]) {
              if (!allowed.has(Number(claim.objectId))) throw new Error('Unrequested provisional card identity');
              const hidden = await game.hiddenCardOpenState(BigInt(claim.objectId));
              if (!hidden?.tracked || Number(hidden.owner) !== Number(claim.owner)) throw new Error('Provisional claim owner mismatch');
              await game.revealHiddenObject({ objectId: Number(claim.objectId), cardName: String(claim.card),
                commitment: String(claim.commitment), recomputeDecision: true });
            }
          },
          dispatch: (command, preState) => base.applySyncedCommand(command, '', {
            runtimeGame: current, preState, provisional: true, publishState: false,
          }),
          requirementsFromState: cryptoRequirementsFromState,
          checkpointHash: publicCheckpointHash,
          localPlayerIndex: multiplayerRef.current.localPlayerIndex,
        });
      },
      restoreVerified,
      publish,
      equivalent: (a, b) => Number(a.actorIndex) === Number(b.actorIndex)
        && commandKey(a.command) === commandKey(b.command),
      onChange: status => {
        for (const [seq, timer] of pendingTimersRef.current) {
          if (seq <= status.acceptedSequence || status.pending === 0) {
            clearTimeout(timer); pendingTimersRef.current.delete(seq);
          }
        }
        setOptimistic(status);
      },
    });
  }
  const controller = controllerRef.current;
  const resetRef = useRef(null);

  function commandClaimIds(command, state) {
    const ids = new Set();
    const ref = command?.action_ref;
    // Only identities the action is announcing publicly are transported.
    if (command?.type === 'priority_action' && ['cast_spell', 'play_land'].includes(ref?.kind)) {
      const id = Number(command.object_id ?? actionRefObjectId(ref));
      if (Number.isSafeInteger(id) && id > 0) ids.add(id);
    }
    if (command?.type === 'select_objects') {
      for (const id of command.object_ids || []) {
        const candidate = selectObjectCandidateForId(state?.decision, id);
        if (selectObjectCandidateRevealPolicy(state?.decision, candidate) === 'public') ids.add(Number(id));
      }
    }
    return ids;
  }

  async function ensureRuntime() {
    if (resettingRef.current) await resettingRef.current;
    if (base.awaitingStateResyncRef.current || base.resyncInProgressRef?.current) return false;
    if (runtimeRef.current) return true;
    if (initializingRef.current) return initializingRef.current;
    if (!multiplayerRef.current.matchStarted || isTrustedMultiplayerSecurityMode(sessionSecurityMode(multiplayerRef.current))
      || !visibleGame()?.supportsRuntimeBranches) return false;
    initializingRef.current = (async () => {
      const runtime = await visibleGame().forkRuntimeBranch();
      try {
        runtimeRef.current = runtime;
        gameRef.current = runtime;
        stateRef.current = await runtime.uiState();
        await controller.start(servicesRef.current.currentAuditMatchId(), multiplayerRef.current.lastAppliedSequence || 0);
        return true;
      } catch (error) {
        runtimeRef.current = null;
        gameRef.current = visibleGame();
        await runtime.release();
        throw error;
      }
    })();
    try { return await initializingRef.current; }
    finally { initializingRef.current = null; }
  }

  function runVerifiedTask(task) {
    const result = verifierQueueRef.current.then(task, task);
    verifierQueueRef.current = result.catch(() => {});
    return result;
  }

  function deferUntilVerifiedIdle(task) {
    const runtime = runtimeRef.current;
    // Recovery can submit another verified task. It must run after the current
    // task's catch/finally has unwound, without becoming part of that queue tail.
    return verifierQueueRef.current.then(async () => {
      // The foreground caller can have its own promise-race/finally gate. Let
      // those completion handlers run before recovery tries to acquire it.
      await new Promise(resolve => setTimeout(resolve, 0));
      // Discarding a failed provisional suffix is normal timeout cleanup. Only
      // replacement of the owning runtime makes this deferred recovery stale.
      if (runtime !== runtimeRef.current) return;
      return task();
    });
  }

  async function verifiedState(state) {
    stateRef.current = state;
    if (!runtimeRef.current) { publish(state); return; }
    const head = Number(multiplayerRef.current.lastAppliedSequence || 0);
    if (head > controller.status().acceptedSequence) {
      await controller.accept(actionHistoryRef.current.find(action => Number(action.seq) === head) || { seq: head });
    } else if (!controller.status().pending && !controller.status().calculating) await restoreVerified();
  }

  async function reset(reason = 'Match recovery') {
    if (resettingRef.current) return resettingRef.current;
    const pendingInitialization = initializingRef.current;
    resettingRef.current = (async () => {
      try { await pendingInitialization; } catch { /* No branch was installed. */ }
      for (const timer of pendingTimersRef.current.values()) clearTimeout(timer);
      pendingTimersRef.current.clear();
      for (const waiting of foregroundReadyRef.current.values()) waiting.resolve();
      foregroundReadyRef.current.clear();
      await controller.close(reason);
      await runtimeRef.current?.release();
      runtimeRef.current = null;
      gameRef.current = visibleGame();
    })();
    try { await resettingRef.current; }
    finally { resettingRef.current = null; }
  }

  async function stageLocal(command, label, submitVerified) {
    if (waitingForMaterialRef.current) return;
    if (base.awaitingStateResyncRef.current || !multiplayerRef.current.matchStarted
      || multiplayerRef.current.localPlayerIndex == null) return submitVerified(command, label);
    if (!await ensureRuntime()) return submitVerified(command, label);
    const session = multiplayerRef.current;
    if (isMatchDisputed(session)) throw new Error('Match is disputed');
    if (isForfeitCommand(command)) {
      await controller.fail(controller.status().acceptedSequence + 1, 'Forfeit superseded provisional choices');
      waitingForMaterialRef.current = true; setWaitingForMaterial(true);
      try { return await runVerifiedTask(() => submitVerified(command, label)); }
      finally { waitingForMaterialRef.current = false; setWaitingForMaterial(false); }
    }
    const state = await visibleGame().uiState();
    command = wireStablePayload(command);
    const disclosure = isNonDispatchSyncCommand(command)
      ? null : await visibleGame().getPaymentDisclosureForCommand(command);
    if (disclosure?.required || disclosure?.active) {
      // Provisional publicClaims are cancelable and precede the signed
      // disclosure commitment. Keep these payments on the verified branch.
      waitingForMaterialRef.current = true; setWaitingForMaterial(true);
      try { return await runVerifiedTask(() => submitVerified(command, label)); }
      finally { waitingForMaterialRef.current = false; setWaitingForMaterial(false); }
    }
    const publicClaims = [];
    for (const objectId of commandClaimIds(command, state)) {
      const open = await visibleGame().hiddenCardOpenState(BigInt(objectId));
      if (open?.tracked && open.open) {
        const claim = await visibleGame().exportHiddenCardOpening(BigInt(objectId));
        publicClaims.push({ objectId, owner: Number(claim.owner), card: String(claim.card),
          commitment: String(claim.commitment) });
      }
    }
    let cancelled = false;
    const dependencyAbort = new AbortController();
    const candidate = { type: 'provisional_action', protocolVersion: PROTOCOL_VERSION,
      id: randomAuditHex(16), matchId: servicesRef.current.currentAuditMatchId(),
      basisSequence: Number(session.lastAppliedSequence || 0),
      actorIndex: Number(session.localPlayerIndex), command, label, publicClaims,
      cancel: () => {
        cancelled = true;
        dependencyAbort.abort();
        if (entry) for (const conn of connections()) safeSend(conn, {
          type: 'provisional_cancel', protocolVersion: PROTOCOL_VERSION,
          matchId: candidate.matchId, actorIndex: candidate.actorIndex, seq: entry.seq, id: entry.id,
        });
      } };
    let entry;
    try { entry = await controller.stage(candidate); }
    catch (error) { recordPeerSyncPerf('optimistic:calculation_blocked', { error: toErrorMessage(error) }); }
    if (!entry) {
      waitingForMaterialRef.current = true; setWaitingForMaterial(true);
      const blockingToken = {};
      blockingCommandRef.current = blockingToken;
      const generation = controller.status().generation;
      const seq = controller.status().provisionalSequence + 1;
      try {
        await servicesRef.current.waitForProtocolActionHead({ seq, matchId: candidate.matchId }, 'Local action dependency');
        if (generation !== controller.status().generation || controller.status().closed) return;
        const foregroundReady = new Promise(resolve => foregroundReadyRef.current.set(seq, { resolve, blockingToken }));
        const completion = runVerifiedTask(async () => {
          const result = await submitVerified(command, label);
          if (generation === controller.status().generation && Number(multiplayerRef.current.lastAppliedSequence || 0) < seq) {
            await controller.fail(seq, 'Action was not accepted');
          }
          return result;
        }).catch(async error => {
          if (generation !== controller.status().generation) return;
          await controller.fail(seq, toErrorMessage(error));
          base.setStatus(toErrorMessage(error), true);
          throw error;
        });
        return await Promise.race([completion, foregroundReady]);
      } finally {
        foregroundReadyRef.current.delete(seq);
        if (blockingCommandRef.current === blockingToken) {
          blockingCommandRef.current = null;
          waitingForMaterialRef.current = false; setWaitingForMaterial(false);
        }
      }
    }
    observeDeadline(entry);
    const { state: _state, generation: _generation, cancel: _cancel, ...message } = entry;
    // Temporary peer claims drive presentation only. The signed canonical
    // envelope must still pass all checks before any transcript append.
    for (const conn of connections()) safeSend(conn, message);
    const completion = (async () => {
      await servicesRef.current.waitForProtocolActionHead(entry, 'Local action dependency', { signal: dependencyAbort.signal });
      if (cancelled || controller.status().generation !== entry.generation) return;
      await runVerifiedTask(async () => {
        if (cancelled || controller.status().generation !== entry.generation) return;
        await submitVerified(command, label);
        if (controller.status().generation === entry.generation && Number(multiplayerRef.current.lastAppliedSequence || 0) < entry.seq) {
          await controller.fail(entry.seq, 'Action was not accepted');
        }
      });
    })().catch(async error => {
      if (cancelled || controller.status().generation !== entry.generation) return;
      await controller.fail(entry.seq, toErrorMessage(error));
      base.setStatus(toErrorMessage(error), true);
    });
    // Completion is deliberately detached from the click's interaction gate.
    void completion;
    return { provisional: true, sequence: entry.seq };
  }

  function connections() {
    return [...new Map([base.hostConnectionRef.current,
      ...base.clientConnectionsRef.current.values(), ...base.peerConnectionsRef.current.values()]
      .filter(conn => conn?.open).map(conn => [conn.peer, conn])).values()];
  }

  async function receive(conn, message) {
    if (Number(servicesRef.current.playerIndexForPeerId(conn?.peer)) !== Number(message.actorIndex)
      || message.matchId !== servicesRef.current.currentAuditMatchId() || !await ensureRuntime()) return;
    const { id, matchId, seq, parentId, basisSequence, actorIndex, command, label,
      publicClaims, publicMaterial, prePublicCheckpointHash, publicCheckpointHash } = message;
    const entry = await controller.stage({ id, matchId, seq, parentId, basisSequence, actorIndex,
      command, label, publicClaims, prePublicCheckpointHash, publicCheckpointHash,
      calculationAudit: publicMaterial ? { openings: publicMaterial.openings,
        rngReveals: publicMaterial.rngReveals, shuffleProofs: publicMaterial.shuffleProofs } : undefined });
    if (entry) observeDeadline(entry);
  }

  async function receiveCanonical(message) {
    if (!await ensureRuntime() || !message.audit) return;
    const entry = await controller.stage({ ...message, matchId: message.audit.matchId,
      id: `canonical:${message.audit.nextStateHash}`, publicClaims: [], calculationAudit: message.audit,
      publicCheckpointHash: message.audit.publicCheckpointHash });
    if (entry) observeDeadline(entry);
  }

  async function stagePreparedLocalAction({ seq, actorIndex, command, label, publicCheckpointHash,
    openings = [], rngReveals = [], shuffleProofs = [], paymentDisclosure = false }) {
    if (paymentDisclosure) return;
    if (!runtimeRef.current || controller.status().closed || isMatchDisputed(multiplayerRef.current) || isForfeitCommand(command)) return;
    if (controller.entries().some(entry => entry.seq === Number(seq))) return;
    // A previously blocked action now has its actual random result and private
    // identities. Copy its completed calculation before payload signing/quorum.
    // Never replace a visible suffix with this older canonical branch.
    if (controller.status().pending || Number(seq) !== controller.status().acceptedSequence + 1) return;
    const candidate = { type: 'provisional_action', protocolVersion: PROTOCOL_VERSION,
      id: randomAuditHex(16), matchId: servicesRef.current.currentAuditMatchId(), seq: Number(seq),
      basisSequence: controller.status().acceptedSequence, actorIndex: Number(actorIndex),
      command: wireStablePayload(command), label, publicCheckpointHash,
      publicClaims: [], publicMaterial: { openings, rngReveals, shuffleProofs },
      preparedRuntime: preparedRuntimeTokenRef.current };
    candidate.cancel = () => {
      for (const conn of connections()) safeSend(conn, { type: 'provisional_cancel', protocolVersion: PROTOCOL_VERSION,
        matchId: candidate.matchId, actorIndex: candidate.actorIndex, seq: candidate.seq, id: candidate.id });
    };
    const entry = await controller.stage(candidate);
    if (!entry) return;
    waitingForMaterialRef.current = false; setWaitingForMaterial(false);
    const waiting = foregroundReadyRef.current.get(entry.seq);
    waiting?.resolve({ provisional: true, sequence: entry.seq });
    observeDeadline(entry);
    const { state: _state, generation: _generation, cancel: _cancel, preparedRuntime: _prepared, ...message } = entry;
    for (const conn of connections()) safeSend(conn, message);
  }

  function observeDeadline(entry) {
    if (pendingTimersRef.current.has(entry.seq)) return;
    const timer = setTimeout(() => {
      pendingTimersRef.current.delete(entry.seq);
      void controller.fail(entry.seq, 'Provisional action was not verified in time').then(() => {
        base.setStatus('Action verification timed out; restored the verified game state', true);
        servicesRef.current.requestResync?.('Recovering unverified actions');
      });
    }, 120000);
    pendingTimersRef.current.set(entry.seq, timer);
  }

  resetRef.current = reset;
  useEffect(() => () => { void resetRef.current('Lobby closed'); }, []);
  return { optimistic, waitingForMaterial, ensureOptimisticRuntime: ensureRuntime, runVerifiedTask, deferUntilVerifiedIdle,
    setVerifiedState: verifiedState, resetOptimisticState: reset,
    stageOptimisticLocalCommand: stageLocal, receiveProvisionalAction: receive,
    stagePreparedLocalAction,
    receiveOptimisticCanonicalAction: receiveCanonical,
    cancelProvisionalAction: (conn, message) => {
      if (Number(servicesRef.current.playerIndexForPeerId(conn?.peer)) !== Number(message.actorIndex)
        || message.matchId !== servicesRef.current.currentAuditMatchId()) return;
      const entry = controller.entries().find(candidate => candidate.id === message.id
        && candidate.seq === Number(message.seq) && Number(candidate.actorIndex) === Number(message.actorIndex));
      return entry ? controller.fail(entry.seq, 'Acting player cancelled the provisional action') : undefined;
    },
    failOptimisticAction: (seq, reason) => controller.fail(seq, reason),
    cancelOptimisticIntent: intent => {
      const entry = controller.entries().find(candidate => Number(candidate.seq) === Number(intent.seq)
        && Number(candidate.actorIndex) === Number(intent.actorIndex)
        && commandKey(candidate.command) === commandKey(intent.command));
      return entry ? controller.fail(entry.seq, 'Acting player cancelled the action') : undefined;
    },
    hasPendingOptimisticActions: () => controller.status().pending > 0,
    hasOptimisticRuntime: () => Boolean(runtimeRef.current) };
}
