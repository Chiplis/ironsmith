// Runs the real peer-lobby stack (usePeerLobby) outside React and outside a
// browser: a headless hook host, the real wasm engine, and an in-memory
// PeerJS-compatible network. One call to createSeat() is one player's client.
import "fake-indexeddb/auto";
import { readFileSync, existsSync } from "node:fs";
import { createHeadlessHook } from '../../src/lib/verified-session/hook-runtime.js';
import { resolveSyncedCommand } from '../../src/lib/sync-command-resolve.js';
import { isDecisionCommandCompatible } from '../../src/lib/sync-commands.js';
import { previewCryptoRequirementsWithMaterial } from '../../src/lib/preview-crypto-material.js';
import { usePeerLobby } from '../../src/hooks/usePeerLobby.js';

const PKG = new URL('../../../wasm_demo/pkg/', import.meta.url);
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

function methodNames(game) {
  const names = new Set();
  for (let proto = Object.getPrototypeOf(game); proto && proto !== Object.prototype; proto = Object.getPrototypeOf(proto)) {
    for (const name of Object.getOwnPropertyNames(proto)) if (name !== 'constructor') names.add(name);
  }
  return names;
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

const NAME_KEYS = new Set(['cardName', 'card_name', 'card', 'name']);
function namesIn(value, found = new Set(), depth = 0) {
  if (depth > 6 || value == null) return found;
  if (Array.isArray(value)) { for (const entry of value) namesIn(entry, found, depth + 1); return found; }
  if (typeof value === 'object') {
    for (const [key, entry] of Object.entries(value)) {
      if (typeof entry === 'string' && NAME_KEYS.has(key)) found.add(entry);
      else if (entry && typeof entry === 'object') namesIn(entry, found, depth + 1);
    }
  } else if (typeof value === 'string' && depth === 0) {
    for (const line of value.split('\n')) {
      const match = /^\s*\d+\s+(.+?)\s*$/.exec(line);
      if (match) found.add(match[1]);
    }
  }
  return found;
}

function createGameProxy(wasm, cards) {
  const proxy = {};
  for (const name of methodNames(wasm)) {
    proxy[name] = async (...args) => {
      cards.ensure(namesIn(args));
      if (name === 'previewCryptoRequirementsWithMaterial') return previewCryptoRequirementsWithMaterial(wasm, args[0], args[1]);
      return wasm[name](...args);
    };
  }
  proxy.__wasm = wasm;
  proxy.supportsRuntimeSavepoints = typeof wasm.createRuntimeSavepoint === 'function';
  proxy.supportsExactBuildSnapshots = false;
  proxy.supportsRuntimeBranches = false;
  proxy.runtimeGeneration = 0;
  return proxy;
}

export async function createSeat({ name, network, deckText = '', autoStatus = false }) {
  const mod = await loadEngine();
  const wasm = new mod.WasmGame();
  const cards = createCardLoader(wasm);
  cards.ensure(namesIn(deckText));
  const game = createGameProxy(wasm, cards);
  const statuses = [];
  const listeners = new Set();
  const store = { state: null };
  const publish = (next) => {
    store.state = typeof next === 'function' ? next(store.state) : next;
    for (const listener of [...listeners]) listener(store.state);
    hook?.update(inputs());
  };
  const inputs = () => ({
    game,
    state: store.state,
    setState: publish,
    subscribeState: (listener) => { listeners.add(listener); return () => listeners.delete(listener); },
    setStatus: (text, isError) => { statuses.push({ text: String(text), isError: Boolean(isError) }); if (autoStatus && isError) console.error(`[${name}] ${text}`); },
    applySyncedCommand: async (command, _message = '', syncOptions = null) => {
      const { preState, runtimeGame } = syncOptions || {};
      const current = runtimeGame || game;
      const before = current.isCurrentSnapshot?.(preState) ? preState : await current.uiState();
      const resolved = resolveSyncedCommand(command, before);
      if (!isDecisionCommandCompatible(before?.decision || null, resolved)) {
        const error = new Error(`Command ${resolved?.type} does not match decision ${before?.decision?.kind}`);
        error.syncedNeedsResync = true;
        throw error;
      }
      let next;
      if (resolved?.type === 'cancel_decision') next = await current.cancelDecision();
      else if (resolved?.type === 'forfeit_player') next = await current.forfeitPlayer(Number(resolved.player));
      else next = await current.dispatch(resolved);
      if (syncOptions?.publishState !== false) publish(next);
      return next;
    },
    createPeer: network.factory(name),
  });
  let hook = null;
  hook = createHeadlessHook(usePeerLobby, inputs(), { onError: (error) => { statuses.push({ text: String(error?.stack || error), isError: true }); console.error(`[${name}] render error`, error); } });
  const seat = {
    name, game, wasm, cards, statuses, store, hook,
    get lobby() { return hook.current; },
    get multiplayer() { return hook.current?.multiplayer; },
    get state() { return store.state; },
    async until(label, predicate, timeoutMs = 30_000) {
      const started = Date.now();
      for (;;) {
        await hook.settled();
        const value = predicate(seat);
        if (value) return value;
        if (Date.now() - started > timeoutMs) {
          const errors = statuses.filter(entry => entry.isError).slice(-5).map(entry => entry.text);
          throw new Error(`${name}: timed out waiting for ${label}; mode=${seat.multiplayer?.mode} started=${seat.multiplayer?.matchStarted} errors=${JSON.stringify(errors)}`);
        }
        await new Promise(resolve => setTimeout(resolve, 20));
      }
    },
    dispose() { hook.dispose(); },
  };
  return seat;
}
