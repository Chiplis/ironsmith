// Entry point of the framework-free Verified session: the peer-lobby stack
// (committed decks, jointly shuffled libraries, signed actions, quorum, resync)
// running on a headless hook host, with the engine, transport and status sink
// supplied by the embedding application (Manabrew, a node harness).
//
//   const session = createVerifiedSession({
//     engine,       // { call(method, ...args), methods?: names } over the wasm game
//     createPeer,   // PeerJS-compatible peer factory (the room transport)
//     ensureCards,  // optional: (names) => Promise<void>, registers card sources
//     onStatus,     // optional: (text, isError) => void
//   });
//   await session.lobby.createLobby({ ... });
//   session.subscribe(({ state, multiplayer }) => ...);
import { createHeadlessHook } from "./hook-runtime.js";
import { WORKER_METHODS } from "./engine-methods.js";
import { resolveSyncedCommand } from "../sync-command-resolve.js";
import { isDecisionCommandCompatible } from "../sync-commands.js";
import { usePeerLobby } from "../../hooks/usePeerLobby.js";

export { previewCryptoRequirementsWithMaterial } from "../preview-crypto-material.js";
export { WORKER_METHODS, ZIFFLE_WORKER_METHODS } from "./engine-methods.js";
export { registerPeerFactory } from "./peer-factory.js";

const NAME_KEYS = new Set(["cardName", "card_name", "card", "name"]);

// Card names an engine call mentions, so their definitions can be registered
// before the engine reads them (an opponent's card is only named when opened).
export function cardNamesInArguments(value, found = new Set(), depth = 0) {
  if (depth > 6 || value == null) return found;
  if (Array.isArray(value)) {
    for (const entry of value) cardNamesInArguments(entry, found, depth + 1);
  } else if (typeof value === "object") {
    for (const [key, entry] of Object.entries(value)) {
      if (typeof entry === "string" && NAME_KEYS.has(key)) found.add(entry);
      else if (entry && typeof entry === "object") cardNamesInArguments(entry, found, depth + 1);
    }
  } else if (typeof value === "string" && depth === 0) {
    for (const line of value.split("\n")) {
      const match = /^\s*\d+\s+(.+?)\s*$/.exec(line);
      if (match) found.add(match[1]);
    }
  }
  return found;
}

export function createVerifiedSession({ engine, createPeer, ensureCards, onStatus, onError } = {}) {
  if (!engine || typeof engine.call !== "function") throw new TypeError("createVerifiedSession needs an engine with call()");
  if (typeof createPeer !== "function") throw new TypeError("createVerifiedSession needs a createPeer transport factory");
  const statuses = [];
  const listeners = new Set();
  const store = { state: null };

  // Only the methods this engine actually has: callers feature-detect with
  // `typeof game.method === "function"`.
  const available = engine.methods ? new Set(engine.methods) : null;
  const game = {};
  for (const method of WORKER_METHODS) {
    if (available && !available.has(method)) continue;
    game[method] = async (...args) => {
      if (ensureCards) {
        const names = cardNamesInArguments(args);
        if (names.size) await ensureCards([...names]);
      }
      return engine.call(method, ...args);
    };
  }
  game.supportsRuntimeSavepoints = true;
  game.supportsExactBuildSnapshots = false;
  game.supportsRuntimeBranches = false;
  game.runtimeGeneration = 0;
  game.ensureCards = ensureCards;

  let hook = null;
  const notify = () => {
    for (const listener of [...listeners]) listener({ state: store.state, multiplayer: hook?.current?.multiplayer });
  };
  const publish = (next) => {
    store.state = typeof next === "function" ? next(store.state) : next;
    notify();
    hook?.update(inputs());
  };
  const inputs = () => ({
    game,
    state: store.state,
    setState: publish,
    subscribeState: (listener) => {
      const wrapped = ({ state }) => listener(state);
      listeners.add(wrapped);
      return () => listeners.delete(wrapped);
    },
    setStatus: (text, isError) => {
      statuses.push({ text: String(text), isError: Boolean(isError) });
      if (statuses.length > 200) statuses.splice(0, statuses.length - 200);
      onStatus?.(String(text), Boolean(isError));
    },
    applySyncedCommand: async (command, _message = "", syncOptions = null) => {
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
      if (resolved?.type === "cancel_decision") next = await current.cancelDecision();
      else if (resolved?.type === "forfeit_player") next = await current.forfeitPlayer(Number(resolved.player));
      else next = await current.dispatch(resolved);
      if (syncOptions?.publishState !== false) publish(next);
      return next;
    },
    createPeer,
  });
  hook = createHeadlessHook(usePeerLobby, inputs(), {
    onError: (error) => {
      statuses.push({ text: String(error?.stack || error), isError: true });
      if (onError) onError(error);
    },
  });
  hook.subscribe(notify);

  return {
    game,
    statuses,
    get lobby() { return hook.current; },
    get multiplayer() { return hook.current?.multiplayer; },
    get state() { return store.state; },
    subscribe(listener) { listeners.add(listener); return () => listeners.delete(listener); },
    publishState: publish,
    settled: () => hook.settled(),
    dispose() { hook.dispose(); listeners.clear(); },
  };
}
