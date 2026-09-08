import { recordDiagnosticEvent } from "../../lib/action-diagnostics.js";
import { useRef, useEffect } from 'react';
import { relayMatchId } from '../../lib/relay/session.js';
import { canonicalMultiplayerPayload, enqueueAsync, safeSend, PROTOCOL_VERSION,
  MULTIPLAYER_SECURITY_TRUSTED, sessionSecurityMode, isTrustedMultiplayerSecurityMode } from './shared.js';

// One host queue owns sequence allocation. Retries retain the same command ID;
// the durable accepted transcript is the source of deduplication after a reload.
export function useTrustedSequencer(base, servicesRef) {
  const { multiplayerRef, actionHistoryRef, matchStartPayloadRef, clientMessageQueueRef,
    hostConnectionRef, clientConnectionsRef, gameRef } = base;
  const pending = useRef(new Map());
  const deliveries = useRef(new Map());
  const index = useRef({ history: null, length: 0, commands: new Map() });
  const matchId = () => relayMatchId(matchStartPayloadRef.current || {});
  const envelope = fields => ({ protocolVersion: PROTOCOL_VERSION,
    securityMode: MULTIPLAYER_SECURITY_TRUSTED, matchId: matchId(), ...fields });
  const sendAccepted = (peer, entry) => {
    const conn = clientConnectionsRef.current.get(peer);
    return safeSend(conn, envelope({ ...entry, type: 'apply_action' }));
  };
  const commandIndex = () => {
    const history = actionHistoryRef.current;
    if (index.current.history !== history || index.current.length > history.length) {
      index.current = { history, length: 0, commands: new Map() };
    }
    while (index.current.length < history.length) {
      const entry = history[index.current.length++];
      if (entry.commandId) index.current.commands.set(entry.commandId, entry);
    }
    return index.current.commands;
  };
  function publishTrustedAction(message) {
    const session = multiplayerRef.current;
    if (session.role !== 'host') return;
    const entry = actionHistoryRef.current[Number(message.seq) - 1];
    if (!entry) return;
    for (const [peer, conn] of clientConnectionsRef.current) {
      if (!session.players.some(p => p.peerId === peer || p.currentPeerId === peer)) continue;
      // At most one outstanding accepted entry per peer. A gap uses the bounded
      // suffix recovery path instead of retaining an unbounded retry transcript.
      deliveries.current.set(peer, { matchId: matchId(), entry, until: Date.now() + 30_000 });
      if (conn.open) sendAccepted(peer, entry);
    }
  }
  async function acceptTrustedCommand(conn, intent) {
    const started = performance.now();
    const session = multiplayerRef.current;
    if (session.role !== 'host' || !session.matchStarted
        || !isTrustedMultiplayerSecurityMode(sessionSecurityMode(session))) throw new Error('Trusted host is unavailable');
    if (intent.matchId !== matchId()) throw new Error('Command belongs to another match');
    const seat = conn ? session.players.find(p => p.peerId === conn.peer || p.currentPeerId === conn.peer)?.index : session.localPlayerIndex;
    if (seat == null || Number(seat) !== Number(intent.actorIndex)) throw new Error('Command seat does not match connection');
    if (typeof intent.commandId !== 'string' || intent.commandId.length > 160 || !intent.commandId) throw new Error('Invalid command ID');
    const previous = commandIndex().get(intent.commandId);
    if (previous) {
      if (previous.actorIndex !== Number(seat) || canonicalMultiplayerPayload(previous.command) !== canonicalMultiplayerPayload(intent.command)) throw new Error('Command ID reused with different content');
      if (conn) sendAccepted(conn.peer, previous);
      return previous;
    }
    const sequence = Number(session.lastAppliedSequence || 0) + 1;
    if (Number(intent.expectedSequence) !== sequence - 1) throw new Error('Command is based on an outdated prompt');
    const state = await gameRef.current.uiState();
    const clock = await servicesRef.current.buildMatchClockAuditForCommand({ command: intent.command, seq: sequence, actorIndex: seat, uiState: state });
    await servicesRef.current.applySequencedActionMessage(envelope({ type: 'apply_action',
      seq: sequence, actorIndex: Number(seat), commandId: intent.commandId,
      command: intent.command, label: String(intent.label || ''), clock }),
    { throwOnFailure: true, throwOnOrderMismatch: true });
    const entry = actionHistoryRef.current[sequence - 1];
    if (!entry || entry.commandId !== intent.commandId) throw new Error('Host did not accept command');
    recordDiagnosticEvent('trusted:host_accept', { commandId: intent.commandId, seq: entry.seq, elapsedMs: performance.now() - started });
    return entry;
  }
  function submitTrustedIntent(command, label) {
    const session = multiplayerRef.current;
    const intent = envelope({ type: 'trusted_command', commandId: `${session.localPeerId}:${crypto.randomUUID()}`,
      expectedSequence: Number(session.lastAppliedSequence || 0), actorIndex: session.localPlayerIndex, command, label });
    if (session.role === 'host') return enqueueAsync(clientMessageQueueRef, () => acceptTrustedCommand(null, intent));
    if (pending.current.size) return Promise.reject(new Error('Waiting for host acceptance'));
    return new Promise((resolve, reject) => {
      pending.current.set(intent.commandId, { intent, resolve, reject, started: performance.now(), until: Date.now() + 30_000 });
      safeSend(hostConnectionRef.current, intent);
    });
  }
  function acknowledgeTrustedAction(conn, message, result) {
    if (!result?.trusted && !result?.duplicate) return;
    const entry = actionHistoryRef.current[Number(message.seq) - 1];
    if (!entry || (message.prefixHash && entry.prefixHash !== message.prefixHash)) return;
    safeSend(conn, envelope({ type: 'trusted_action_ack', seq: entry.seq, prefixHash: entry.prefixHash }));
    const waiter = pending.current.get(entry.commandId);
    if (waiter) {
      pending.current.delete(entry.commandId);
      recordDiagnosticEvent('trusted:accepted_and_published', { commandId: entry.commandId, seq: entry.seq, elapsedMs: performance.now() - waiter.started });
      waiter.resolve(entry);
    }
  }
  function receiveTrustedAck(conn, message) {
    const entry = actionHistoryRef.current[Number(message.seq) - 1];
    if (message.matchId !== matchId() || !entry || entry.prefixHash !== message.prefixHash) return;
    const item = deliveries.current.get(conn.peer);
    if (item && item.matchId === message.matchId && item.entry.seq === Number(message.seq)
        && item.entry.prefixHash === message.prefixHash) deliveries.current.delete(conn.peer);
  }
  function rejectTrustedIntent(message) {
    const item = pending.current.get(message.commandId);
    if (item) {
      pending.current.delete(message.commandId);
      servicesRef.current.requestResync?.('Host rejected an outdated command; recovering state');
      item.reject(new Error(message.reason || 'Host rejected command'));
    }
  }
  useEffect(() => {
    const timer = setInterval(() => {
      for (const [id, item] of pending.current) {
        if (item.intent.matchId !== matchId() || Date.now() > item.until) {
          pending.current.delete(id); item.reject(new Error('Host acceptance timed out; recovering accepted state'));
          servicesRef.current.requestResync?.('Recovering host acceptance');
        } else safeSend(hostConnectionRef.current, item.intent);
      }
      for (const [peer, item] of deliveries.current) {
        if (item.matchId !== matchId() || !multiplayerRef.current.players.some(p => p.peerId === peer || p.currentPeerId === peer)) {
          deliveries.current.delete(peer); continue;
        }
        if (Date.now() > item.until) {
          // Ask the recipient for its current prefix; a lost ACK means the
          // host's last observed prefix may itself be stale.
          safeSend(clientConnectionsRef.current.get(peer), envelope({ type: 'trusted_recovery_needed' }));
          item.until = Date.now() + 30_000;
        } else sendAccepted(peer, item.entry);
      }
    }, 1000);
    return () => {
      clearInterval(timer);
      for (const item of pending.current.values()) item.reject(new Error('Session closed'));
      pending.current.clear(); deliveries.current.clear();
    };
  }, []); // Mutable session/service refs are intentional; never restart retry timers on clock ticks.
  return { publishTrustedAction, acceptTrustedCommand, submitTrustedIntent,
    acknowledgeTrustedAction, receiveTrustedAck, rejectTrustedIntent };
}
