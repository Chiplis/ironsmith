import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const messaging = readFileSync(new URL('../src/hooks/peer-lobby/messaging.js', import.meta.url), 'utf8');
const shared = readFileSync(new URL('../src/hooks/peer-lobby/shared.js', import.meta.url), 'utf8');
const enqueueStart = shared.indexOf('export function enqueueAsync(');
const enqueueEnd = shared.indexOf('\nexport ', enqueueStart + 1);
const enqueueAsync = new Function(`${shared.slice(enqueueStart, enqueueEnd).replace('export ', '')}; return enqueueAsync;`)();

// Exercise the production transport callbacks; the action handler models a
// relayed action waiting for the verifier, which is awaiting crypto material.
function transportHarness(marker) {
  const start = messaging.indexOf('conn.on("data",', messaging.indexOf(marker));
  const end = messaging.indexOf('conn.on("close",', start);
  assert.ok(start >= 0 && end > start, marker);
  let receive, release;
  const dependency = new Promise(resolve => { release = resolve; });
  const events = [];
  const queue = { current: Promise.resolve() };
  const conn = { peer: 'remote', on: (type, handler) => { assert.equal(type, 'data'); receive = handler; } };
  const handle = async message => {
    if (message.type === 'apply_action') {
      events.push('action waiting');
      await dependency;
      events.push('action finished');
    } else {
      events.push(message.type);
      if (message.type.startsWith('crypto_material_')) release();
    }
  };
  const noop = () => {};
  const context = {
    conn, heartbeatKey: 'peer', hostConnectionRef: { current: conn },
    peerMessageQueueRef: queue, clientMessageQueueRef: queue, hostMessageQueueRef: queue,
    enqueueAsync, handleHostMessage: handle,
    handlePeerMessage: (_conn, message) => handle(message),
    handleClientMessage: (_conn, message) => handle(message),
    markConnectionAlive: noop, recordPeerMessage: noop, approximateMessageBytes: () => 0,
    handleConnectionHeartbeatMessage: () => false, PROTOCOL_VERSION: 14,
    servicesRef: { current: { receiveOptimisticCanonicalAction: async () => {} } },
    recordPeerSyncPerf: noop, recordDiagnosticEvent: noop, shouldSuppressProtocolMessageError: () => false,
    toErrorMessage: String, safeSend: noop,
    setStatus: text => events.push(`error: ${text}`),
    emitSyncFailureNotice: text => events.push(`error: ${text}`), emitZiffleDiagnosticNotice: noop,
  };
  new Function(...Object.keys(context), messaging.slice(start, end))(...Object.values(context));
  return { receive, release, events, queue };
}

for (const [route, marker] of [
  ['direct peer', 'const configurePeerConnection ='],
  ['client to host', 'const configureHostConnection ='],
  ['host to client', 'const joinLobby ='],
]) {
  for (const type of ['crypto_material_request', 'crypto_material_response']) {
    test(`${route}: ${type} unblocks verification ahead of a queued action`, async () => {
      const h = transportHarness(marker);
      try {
        h.receive({ type: 'apply_action', protocolVersion: 14, seq: 48 });
        await new Promise(setImmediate);
        assert.deepEqual(h.events, ['action waiting']);
        h.receive({ type: 'lobby_state' });
        h.receive({ type, protocolVersion: 14, requestId: 'action-49' });
        await new Promise(setImmediate);
        assert.deepEqual([...h.events], ['action waiting', type, 'action finished', 'lobby_state']);
      } finally {
        h.release();
        await h.queue.current;
      }
    });
  }
}
