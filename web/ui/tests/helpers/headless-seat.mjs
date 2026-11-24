// Runs the real peer-lobby stack (usePeerLobby) outside React and outside a
// browser: a headless hook host, the real wasm engine, and an in-memory
// PeerJS-compatible network. One call to createSeat() is one player's client.
import "fake-indexeddb/auto";
import { readFileSync, existsSync } from "node:fs";
import { cardNamesInArguments, createVerifiedSession, previewCryptoRequirementsWithMaterial } from '../../src/lib/verified-session/index.js';

const PKG = process.env.IRONSMITH_PKG ? new URL('file://' + process.env.IRONSMITH_PKG.replace(/\/?$/, '/')) : new URL('../../../wasm_demo/pkg/', import.meta.url);
const CARDS = new URL('../../public/cards/', import.meta.url);

let enginePromise = null;
export function loadEngine() {
  enginePromise ||= (async () => {
    const mod = await import(new URL('ironsmith.js', PKG).href);
    await mod.default({
      engine: { module_or_path: readFileSync(new URL('engine_bg.wasm', PKG)) },
      compiler: false,
      verifier: { module_or_path: readFileSync(new URL('verifier_bg.wasm', PKG)) },
    });
    return mod;
  })();
  return enginePromise;
}

export function cardRoute(name) {
  return String(name || '').trim().toLocaleLowerCase('en-US').normalize('NFKD')
    .replace(/[̀-ͯ]/g, '').replace(/[^a-z0-9_]+/g, '-').replace(/^-+|-+$/g, '');
}

class Events {
  listeners = new Map();
  on(type, fn) { this.listeners.set(type, [...(this.listeners.get(type) || []), fn]); return this; }
  off(type, fn) { this.listeners.set(type, (this.listeners.get(type) || []).filter(entry => entry !== fn)); return this; }
  emit(type, ...args) { for (const fn of [...(this.listeners.get(type) || [])]) fn(...args); }
}

// PeerJS-compatible subset: enough for the lobby and match protocol.
export class MemoryNetwork {
  peers = new Map();
  counter = 0;
  log = [];
  factory(label) {
    return (requestedId, options) => new MemoryPeer(this, requestedId || `${label}-${++this.counter}`, options);
  }
}

class MemoryConnection extends Events {
  open = false;
  constructor(owner, peer, other = null, metadata = {}) {
    super();
    Object.assign(this, { owner, peer, other, metadata });
  }
  send(payload) {
    if (!this.open) throw new Error('Connection closed');
    // The wire (BinaryPack/JSON) never delivers `undefined`; mimic it.
    const wire = JSON.parse(JSON.stringify(payload));
    const target = this.other;
    this.owner.network.log.push({ from: this.owner.id, to: this.peer, type: wire?.type });
    setTimeout(() => { if (target.open) target.emit('data', wire, { bytes: 0, transport: 'memory' }); }, 0);
    return { bytes: 0 };
  }
  close() {
    if (!this.open) return;
    this.open = false;
    this.emit('close');
    const other = this.other;
    if (other?.open) setTimeout(() => { other.open = false; other.emit('close'); }, 0);
  }
}

class MemoryPeer extends Events {
  open = false;
  disconnected = true;
  destroyed = false;
  connections = new Set();
  constructor(network, id, options = {}) {
    super();
    Object.assign(this, { network, id, options });
    network.peers.set(id, this);
    queueMicrotask(() => {
      if (this.destroyed) return;
      this.open = true; this.disconnected = false;
      this.emit('open', id);
    });
  }
  connect(targetId, options = {}) {
    const mine = new MemoryConnection(this, targetId, null, options.metadata);
    this.connections.add(mine);
    setTimeout(() => {
      const target = this.network.peers.get(targetId);
      if (!target || target.destroyed || !target.open) { mine.emit('error', new Error(`Could not connect to ${targetId}`)); return; }
      const theirs = new MemoryConnection(target, this.id, mine, options.metadata);
      mine.other = theirs;
      target.connections.add(theirs);
      target.emit('connection', theirs);
      mine.open = true; theirs.open = true;
      theirs.emit('open'); mine.emit('open');
    }, 0);
    return mine;
  }
  reconnect() {}
  advertise() {}
  send() {}
  destroy() {
    if (this.destroyed) return;
    this.destroyed = true; this.open = false; this.disconnected = true;
    for (const conn of [...this.connections]) conn.close();
    this.network.peers.delete(this.id);
    this.emit('close');
  }
}

// Card definitions load on demand, like the worker's route registration.
function createCardLoader(game) {
  const registered = new Set();
  return {
    ensure(names) {
      const sources = [];
      for (const name of names) {
        const route = cardRoute(name);
        if (!route || registered.has(route)) continue;
        const file = new URL(`${route}.json`, CARDS);
        if (!existsSync(file)) continue;
        registered.add(route);
        sources.push(JSON.parse(readFileSync(file, 'utf8')));
      }
      if (sources.length) game.registerExternalCardSourcesJson(JSON.stringify(sources));
    },
  };
}


// A direct (same-thread) engine: the session's `engine.call` over a wasm game.
function methodNames(game) {
  const names = new Set(['previewCryptoRequirementsWithMaterial']);
  for (let proto = Object.getPrototypeOf(game); proto && proto !== Object.prototype; proto = Object.getPrototypeOf(proto)) {
    for (const name of Object.getOwnPropertyNames(proto)) if (name !== 'constructor') names.add(name);
  }
  return names;
}

function createDirectEngine(wasm) {
  return {
    methods: methodNames(wasm),
    async call(method, ...args) {
      if (method === 'previewCryptoRequirementsWithMaterial') return previewCryptoRequirementsWithMaterial(wasm, args[0], args[1]);
      return wasm[method](...args);
    },
  };
}

export async function createSeat({ name, network, deckText = '' }) {
  const mod = await loadEngine();
  const wasm = new mod.WasmGame();
  const cards = createCardLoader(wasm);
  cards.ensure(cardNamesInArguments(deckText));
  const session = createVerifiedSession({
    engine: createDirectEngine(wasm),
    createPeer: network.factory(name),
    ensureCards: async names => cards.ensure(names),
  });
  const seat = {
    name, wasm, cards, session,
    statuses: session.statuses,
    get lobby() { return session.lobby; },
    get multiplayer() { return session.multiplayer; },
    get state() { return session.state; },
    dispose() { session.dispose(); },
  };
  return seat;
}
