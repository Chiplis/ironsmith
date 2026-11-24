// Worker-thread entry: one isolated seat (own module graph, wasm engine and
// IndexedDB) whose peers talk to other seats through the hub on the main thread.
import { parentPort, workerData } from 'node:worker_threads';
// Each worker is one browser tab: give it tab-local storage and a window.
for (const name of ['localStorage', 'sessionStorage']) {
  const map = new Map();
  globalThis[name] = {
    getItem: key => (map.has(String(key)) ? map.get(String(key)) : null),
    setItem: (key, value) => { map.set(String(key), String(value)); },
    removeItem: key => { map.delete(String(key)); },
    key: index => [...map.keys()][index] ?? null,
    get length() { return map.size; },
    clear: () => map.clear(),
  };
}
globalThis.window = globalThis;
const { createSeat } = await import('./headless-seat.mjs');

class Events {
  listeners = new Map();
  on(type, fn) { this.listeners.set(type, [...(this.listeners.get(type) || []), fn]); return this; }
  off(type, fn) { this.listeners.set(type, (this.listeners.get(type) || []).filter(entry => entry !== fn)); return this; }
  emit(type, ...args) { for (const fn of [...(this.listeners.get(type) || [])]) fn(...args); }
}

const connections = new Map(); // connId -> BridgeConnection
const peers = new Map();       // peer id -> BridgePeer
let counter = 0;
const post = message => parentPort.postMessage(message);
// recordPeerSyncPerf logs every protocol step; keep worker output quiet.
console.info = () => {};
console.debug = () => {};

class BridgeConnection extends Events {
  open = false;
  constructor(owner, peer, connId, metadata = {}) {
    super();
    Object.assign(this, { owner, peer, connId, metadata });
    connections.set(connId, this);
  }
  send(payload) {
    if (!this.open) throw new Error('Connection closed');
    post({ t: 'send', connId: this.connId, payload: JSON.parse(JSON.stringify(payload)) });
    return { bytes: 0 };
  }
  close() {
    if (!this.open) return;
    this.open = false;
    post({ t: 'close', connId: this.connId });
    this.emit('close');
  }
}

class BridgePeer extends Events {
  open = false;
  disconnected = true;
  destroyed = false;
  constructor(id, options = {}) {
    super();
    Object.assign(this, { id, options });
    peers.set(id, this);
    post({ t: 'peer-open', id });
    queueMicrotask(() => {
      if (this.destroyed) return;
      this.open = true; this.disconnected = false;
      this.emit('open', id);
    });
  }
  connect(targetId, options = {}) {
    const connId = `${this.id}>${targetId}#${++counter}`;
    const conn = new BridgeConnection(this, targetId, connId, options.metadata);
    post({ t: 'connect', connId, from: this.id, to: targetId, metadata: options.metadata || {} });
    return conn;
  }
  reconnect() {}
  advertise() {}
  send() {}
  destroy() {
    if (this.destroyed) return;
    this.destroyed = true; this.open = false; this.disconnected = true;
    for (const conn of [...connections.values()]) if (conn.owner === this) conn.close();
    peers.delete(this.id);
    post({ t: 'peer-destroy', id: this.id });
    this.emit('close');
  }
}

const network = { factory: label => (requested, options) => new BridgePeer(requested || `${label}-${++counter}`, options) };

function onHub(message) {
  if (message.t === 'incoming') {
    const peer = peers.get(message.to);
    if (!peer || peer.destroyed) { post({ t: 'refuse', connId: message.connId }); return; }
    const conn = new BridgeConnection(peer, message.from, message.connId, message.metadata);
    conn.open = true;
    peer.emit('connection', conn);
    conn.emit('open');
    post({ t: 'accepted', connId: message.connId });
  } else if (message.t === 'accepted') {
    const conn = connections.get(message.connId);
    if (conn && !conn.open) { conn.open = true; conn.emit('open'); }
  } else if (message.t === 'refuse') {
    const conn = connections.get(message.connId);
    if (conn) { conn.emit('error', new Error(`Could not connect to ${conn.peer}`)); connections.delete(message.connId); }
  } else if (message.t === 'data') {
    const conn = connections.get(message.connId);
    if (conn?.open) conn.emit('data', message.payload, { bytes: 0, transport: 'bridge' });
  } else if (message.t === 'closed') {
    const conn = connections.get(message.connId);
    if (conn?.open) { conn.open = false; conn.emit('close'); }
  }
}

let seat = null;
const plain = value => JSON.parse(JSON.stringify(value ?? null, (_k, v) => (typeof v === 'bigint' ? Number(v) : typeof v === 'function' ? undefined : v)));

async function onRpc(message) {
  try {
    let result;
    if (message.op === 'create') {
      seat = await createSeat({ name: workerData.name, network, deckText: message.deckText || '' });
      result = true;
    } else if (message.op === 'call') {
      const target = message.path === 'lobby' ? seat.lobby : seat[message.path];
      result = await target[message.method](...(message.args || []));
    } else if (message.op === 'snapshot') {
      const m = seat.multiplayer || {};
      const s = seat.state || {};
      result = {
        multiplayer: plain({ mode: m.mode, role: m.role, lobbyId: m.lobbyId, matchStarted: m.matchStarted, players: (m.players || []).map(p => ({ index: p.index, name: p.name, ready: p.ready, connected: p.connected })), lastAppliedSequence: m.lastAppliedSequence, submittingAction: m.submittingAction, disputed: m.disputed, matchDisputed: m.matchDisputed, peerWait: m.peerWait, localPlayerIndex: m.localPlayerIndex, desiredPlayers: m.desiredPlayers }),
        canStart: Boolean(seat.lobby?.canStartHostedMatch),
        state: plain({ decision: s.decision, turn: s.turn_number, phase: s.phase, step: s.step, active_player: s.active_player, priority_player: s.priority_player, game_over: s.game_over, players: (s.players || []).map(p => ({ id: p.id, life: p.life, hand_size: p.hand_size, library_size: p.library_size })), perspective: s.perspective }),
        errors: seat.statuses.filter(x => x.isError).slice(-6).map(x => x.text),
        lastStatus: seat.statuses.at(-1)?.text,
      };
    } else if (message.op === 'dispose') {
      seat?.dispose();
      result = true;
    }
    post({ t: 'rpc-result', id: message.id, ok: true, result: plain(result) });
  } catch (error) {
    post({ t: 'rpc-result', id: message.id, ok: false, error: String(error?.stack || error) });
  }
}

parentPort.on('message', message => { if (message.t === 'rpc') void onRpc(message); else onHub(message); });
post({ t: 'ready' });
