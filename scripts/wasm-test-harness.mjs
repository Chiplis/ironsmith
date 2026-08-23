import { readFile } from "node:fs/promises";
import { readFileSync } from "node:fs";

export const DEFAULT_PLAYER_NAMES = ["Alice", "Bob"];

// The wasm-lean build ships with an empty baked card registry; card
// definitions are registered at runtime from the per-card JSON assets the
// frontend serves from web/ui/public/cards/. In Node we read the same files
// from disk and register them before any name-based lookup reaches the WASM.
const CARD_ASSETS_BASE = new URL("../web/ui/public/cards/", import.meta.url);
const cardSourcePayloadCache = new Map();
const missingCardRoutes = new Set();
const gameRegisteredCardRoutes = new WeakMap();

function cardRouteKey(name) {
  return String(name || "")
    .trim()
    .toLocaleLowerCase("en-US")
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function readCardSourcePayload(route) {
  if (missingCardRoutes.has(route)) return null;
  if (cardSourcePayloadCache.has(route)) return cardSourcePayloadCache.get(route);
  let payload = null;
  try {
    payload = JSON.parse(readFileSync(new URL(`${route}.json`, CARD_ASSETS_BASE), "utf8"));
  } catch {
    missingCardRoutes.add(route);
    return null;
  }
  if (!payload || typeof payload !== "object" || !payload.group) {
    missingCardRoutes.add(route);
    return null;
  }
  cardSourcePayloadCache.set(route, payload);
  return payload;
}

export function ensureCardSourcesForNames(game, names) {
  if (!game || typeof game.registerExternalCardSourcesJson !== "function") return;
  let registered = gameRegisteredCardRoutes.get(game);
  if (!registered) {
    registered = new Set();
    gameRegisteredCardRoutes.set(game, registered);
  }
  const payloads = [];
  for (const raw of Array.isArray(names) ? names : [names]) {
    const name = String(raw || "").trim();
    if (!name) continue;
    const route = cardRouteKey(name);
    if (!route || registered.has(route)) continue;
    registered.add(route);
    if (typeof game.isKnownCardName === "function") {
      try {
        if (game.isKnownCardName(name)) continue;
      } catch {
        // fall through to registration
      }
    }
    const payload = readCardSourcePayload(route);
    if (payload) payloads.push(payload);
  }
  if (payloads.length === 0) return;
  try {
    game.registerExternalCardSourcesJson(JSON.stringify(payloads));
  } catch {
    // Retry one-by-one so a single bad payload doesn't block the rest.
    for (const payload of payloads) {
      try {
        game.registerExternalCardSourcesJson(JSON.stringify(payload));
      } catch {
        // leave resolution errors to the engine's unknown-card-name path
      }
    }
  }
}

function deckEntryName(entry) {
  if (typeof entry === "string") return entry;
  if (entry && typeof entry === "object") return entry.name ?? entry.cardName ?? null;
  return null;
}

function instrumentWasmGameClass(WasmGame) {
  if (!WasmGame || WasmGame.__cardSourceInstrumented) return;
  WasmGame.__cardSourceInstrumented = true;
  const proto = WasmGame.prototype;
  const patch = (method, extractNames) => {
    const original = proto[method];
    if (typeof original !== "function") return;
    proto[method] = function (...args) {
      ensureCardSourcesForNames(this, extractNames(args));
      return original.apply(this, args);
    };
  };
  patch("addCardToZone", (args) => [args[1]]);
  patch("addCardToHand", (args) => [args[1]]);
  patch("startMatch", (args) => {
    const options = args[0] || {};
    const lists = [...(options.decks || []), ...(options.sideboards || [])];
    return lists.flatMap((list) => (list || []).map(deckEntryName));
  });
}

export function packageBase(pkg = "root") {
  if (pkg === "root") return "../pkg";
  if (pkg === "demo") return "../web/wasm_demo/pkg";
  if (pkg === "bench") return "../target/bench-wasm-pkg";
  throw new Error(`unknown wasm package: ${pkg}`);
}

const wasmRuntimeCache = new Map();

export async function initWasmRuntime({ pkg = "root" } = {}) {
  if (!wasmRuntimeCache.has(pkg)) {
    wasmRuntimeCache.set(pkg, loadWasmRuntime(pkg));
  }
  return wasmRuntimeCache.get(pkg);
}

async function loadWasmRuntime(pkg) {
  const base = packageBase(pkg);
  const wasmModule = await import(`${base}/ironsmith.js`);
  const [engine, compiler, verifier] = await Promise.all([
    readFile(new URL(`${base}/engine_bg.wasm`, import.meta.url)),
    readFile(new URL(`${base}/compiler_bg.wasm`, import.meta.url)),
    readFile(new URL(`${base}/verifier_bg.wasm`, import.meta.url)),
  ]);
  await wasmModule.default({ engine, compiler, verifier });
  instrumentWasmGameClass(wasmModule.WasmGame);
  return {
    wasmModule,
    packagePath: base.replace(/^\.\.\//, ""),
  };
}

export async function initWasmGame({ pkg = "root" } = {}) {
  const runtime = await initWasmRuntime({ pkg });
  return {
    ...runtime,
    game: new runtime.wasmModule.WasmGame(),
  };
}

export function startEmptyMatch(
  game,
  {
    playerNames = DEFAULT_PLAYER_NAMES,
    startingLife = 20,
    seed = 1,
    format = "normal",
    openingHandSize = 0,
    decks = playerNames.map(() => []),
  } = {},
) {
  return game.startMatch({
    playerNames,
    startingLife,
    seed,
    format,
    decks,
    openingHandSize,
  });
}

export function assert(condition, message, details) {
  if (condition) return;
  const suffix = details === undefined ? "" : `\n${JSON.stringify(details, null, 2)}`;
  throw new Error(`${message}${suffix}`);
}

export function getState(game) {
  return game.uiState();
}

export function getCheckpoint(game) {
  return game.exportSyncCheckpoint();
}

export function getGame(game) {
  return getCheckpoint(game);
}

export function importCheckpoint(game, checkpoint, { perspective = checkpoint?.perspective ?? 0 } = {}) {
  return game.importSyncCheckpoint(checkpoint, normalizePlayerId(perspective));
}

export function runCode(game, mutator, { perspective = null } = {}) {
  const checkpoint = getCheckpoint(game);
  const result = mutator(checkpoint);
  try {
    importCheckpoint(game, checkpoint, {
      perspective: perspective ?? checkpoint.perspective ?? 0,
    });
  } catch (error) {
    throw new Error(
      `runCode checkpoint import failed. This helper can mutate ordinary sync-checkpoint fields, but custom cards created only in the live registry cannot currently be restored through export/import. ${error.message}`,
    );
  }
  return result;
}

export function captureCheckpoint(game) {
  return structuredClone(getCheckpoint(game));
}

export function restoreCheckpoint(game, checkpoint, { perspective = checkpoint?.perspective ?? 0 } = {}) {
  return importCheckpoint(game, structuredClone(checkpoint), { perspective });
}

export function createNewGameAndPlayers(game, options = {}) {
  const {
    playerNames = DEFAULT_PLAYER_NAMES,
    startingLife = 20,
    seed = 1,
    format = "normal",
    openingHandSize = 0,
    decks = playerNames.map(() => []),
    sideboards = undefined,
  } = options;
  const state = game.startMatch({
    playerNames,
    startingLife,
    seed,
    format,
    decks,
    sideboards,
    openingHandSize,
  });
  return {
    state,
    players: playerNames.map((name, id) => ({ id, name })),
  };
}

export function initAndCreateGame(game, options = {}) {
  return createNewGameAndPlayers(game, options);
}

export function createPlayer(id, name = `Player ${Number(id) + 1}`) {
  return { id: normalizePlayerId(id), name };
}

export function setSideboard(game, player, cardNames) {
  const playerId = normalizePlayerId(player);
  return cardNames.map((cardName) =>
    Number(game.addCardToZone(playerId, cardName, "outside_game", true)),
  );
}

export function objectIndex(checkpointOrGame) {
  const checkpoint = isGameLike(checkpointOrGame) ? getCheckpoint(checkpointOrGame) : checkpointOrGame;
  return new Map((checkpoint.objects || []).map((object) => [Number(object.id), object]));
}

export function normalizePlayerId(player) {
  if (typeof player === "number") return player;
  if (typeof player === "bigint") return Number(player);
  if (typeof player === "string") {
    const lowered = player.trim().toLowerCase();
    if (lowered === "alice" || lowered === "playera" || lowered === "player a") return 0;
    if (lowered === "bob" || lowered === "playerb" || lowered === "player b") return 1;
    if (lowered === "charlie" || lowered === "playerc" || lowered === "player c") return 2;
    if (lowered === "dana" || lowered === "playerd" || lowered === "player d") return 3;
    const numeric = Number(player);
    if (Number.isInteger(numeric)) return numeric;
  }
  if (player && Number.isInteger(player.id)) return player.id;
  throw new Error(`invalid player reference: ${String(player)}`);
}

export function getPlayer(checkpointOrGame, player) {
  const checkpoint = isGameLike(checkpointOrGame) ? getCheckpoint(checkpointOrGame) : checkpointOrGame;
  const playerId = normalizePlayerId(player);
  const found = (checkpoint.players || []).find((candidate) => Number(candidate.id) === playerId);
  assert(found, `unknown player ${playerId}`);
  return found;
}

export function getObjectsInZone(checkpointOrGame, zone, player = null) {
  const checkpoint = isGameLike(checkpointOrGame) ? getCheckpoint(checkpointOrGame) : checkpointOrGame;
  const objectsById = objectIndex(checkpoint);
  const normalizedZone = normalizeZoneName(zone);
  let ids;

  if (player !== null && player !== undefined) {
    const playerSnapshot = getPlayer(checkpoint, player);
    ids = idsForPlayerZone(playerSnapshot, normalizedZone);
  } else if (normalizedZone === "battlefield") {
    ids = checkpoint.battlefield || [];
  } else if (normalizedZone === "exile") {
    ids = checkpoint.exile || [];
  } else if (normalizedZone === "command") {
    ids = checkpoint.command || [];
  } else if (normalizedZone === "stack") {
    ids = (checkpoint.stack || []).map((entry) => entry.objectId ?? entry.object_id);
  } else {
    ids = (checkpoint.players || []).flatMap((playerSnapshot) =>
      idsForPlayerZone(playerSnapshot, normalizedZone),
    );
  }

  return ids.map((id) => objectsById.get(Number(id))).filter(Boolean);
}

export function getBattlefield(checkpointOrGame, player = null) {
  const all = getObjectsInZone(checkpointOrGame, "battlefield");
  if (player === null || player === undefined) return all;
  const playerId = normalizePlayerId(player);
  return all.filter((object) => Number(object.controller ?? object.owner) === playerId);
}

export function getAllActivePermanents(checkpointOrGame, player = null) {
  return getBattlefield(checkpointOrGame, player).filter((object) => !object.phasedOut);
}

export function getHand(checkpointOrGame, player) {
  return getObjectsInZone(checkpointOrGame, "hand", player);
}

export function getLibrary(checkpointOrGame, player, { topFirst = true } = {}) {
  const cards = getObjectsInZone(checkpointOrGame, "library", player);
  return topFirst ? [...cards].reverse() : cards;
}

export function getGraveyard(checkpointOrGame, player, { topFirst = true } = {}) {
  const cards = getObjectsInZone(checkpointOrGame, "graveyard", player);
  return topFirst ? [...cards].reverse() : cards;
}

export function getExile(checkpointOrGame, player = null) {
  const cards = getObjectsInZone(checkpointOrGame, "exile");
  if (player === null || player === undefined) return cards;
  const playerId = normalizePlayerId(player);
  return cards.filter((object) => Number(object.owner) === playerId);
}

export function getManaPool(checkpointOrGame, player) {
  return getPlayer(checkpointOrGame, player).manaPool;
}

export function getPermanent(checkpointOrGame, player, nameOrPredicate, options = {}) {
  const matches = findPermanents(checkpointOrGame, player, nameOrPredicate, options);
  const { index = 0, optional = false } = options;
  const found = matches[index];
  if (!found && !optional) {
    throw new Error(`permanent not found: ${describeQuery(nameOrPredicate)}`);
  }
  return found ?? null;
}

export function findPermanents(checkpointOrGame, player, nameOrPredicate, { includePhasedOut = false } = {}) {
  const permanents = includePhasedOut
    ? getBattlefield(checkpointOrGame, player)
    : getAllActivePermanents(checkpointOrGame, player);
  return filterObjects(permanents, nameOrPredicate);
}

export function getId(checkpointOrGame, player, nameOrPredicate, options = {}) {
  const object = getPermanent(checkpointOrGame, player, nameOrPredicate, options);
  return object ? Number(object.id) : null;
}

export function getObject(checkpointOrGame, id) {
  const found = objectIndex(checkpointOrGame).get(Number(id));
  assert(found, `unknown object id ${id}`);
  return found;
}

export function getObjectDetails(game, objectOrId) {
  const id = typeof objectOrId === "object" ? objectOrId.id : objectOrId;
  return game.objectDetails(BigInt(id));
}

export function getAbilities(game, objectOrId) {
  return getObjectDetails(game, objectOrId).abilities || [];
}

export function hasAbility(game, objectOrId, textOrPredicate) {
  const abilities = getAbilities(game, objectOrId);
  if (typeof textOrPredicate === "function") return abilities.some(textOrPredicate);
  return abilities.some((ability) => ability.includes(textOrPredicate));
}

export function getAttachments(checkpointOrGame, objectOrId) {
  const checkpoint = isGameLike(checkpointOrGame) ? getCheckpoint(checkpointOrGame) : checkpointOrGame;
  const object = typeof objectOrId === "object" ? objectOrId : getObject(checkpoint, objectOrId);
  return (object.attachments || []).map((id) => getObject(checkpoint, id));
}

export function getAttachedTo(checkpointOrGame, objectOrId) {
  const checkpoint = isGameLike(checkpointOrGame) ? getCheckpoint(checkpointOrGame) : checkpointOrGame;
  const object = typeof objectOrId === "object" ? objectOrId : getObject(checkpoint, objectOrId);
  const target = object.attachedTo;
  if (!target) return null;
  if (target.kind === "object") return getObject(checkpoint, target.object);
  return { kind: "player", player: Number(target.player) };
}

export function showAvailableAbilities(stateOrGame) {
  const state = isGameLike(stateOrGame) ? getState(stateOrGame) : stateOrGame;
  return (state?.decision?.actions || []).map((action) => ({
    index: action.index,
    label: action.label,
    kind: action.kind,
    objectId: action.object_id === undefined ? null : Number(action.object_id),
    actionRef: action.action_ref,
  }));
}

export function actionByLabel(state, labelOrPattern) {
  const actions = state?.decision?.actions || [];
  const found =
    labelOrPattern instanceof RegExp
      ? actions.find((action) => labelOrPattern.test(action.label || ""))
      : actions.find((action) => action.label === labelOrPattern);
  assert(found, `could not find action ${String(labelOrPattern)}`, showAvailableAbilities(state));
  return found;
}

export function actionByPredicate(state, predicate, description = "matching action") {
  const found = (state?.decision?.actions || []).find(predicate);
  assert(found, `could not find ${description}`, showAvailableAbilities(state));
  return found;
}

export function addCustomCardWithAbility(
  game,
  {
    player = 0,
    zone = "battlefield",
    name = "Ironsmith Test Card",
    manaCost = "",
    typeLine = "Creature - Shapeshifter",
    oracleText = "",
    power = "1",
    toughness = "1",
    loyalty = null,
    defense = null,
    colorIndicator = [],
    skipTriggers = true,
  } = {},
) {
  const parsed = parseTypeLine(typeLine);
  const face = {
    name,
    manaCost: manaCost || null,
    colorIndicator,
    supertypes: parsed.supertypes,
    cardTypes: parsed.cardTypes,
    subtypes: parsed.subtypes,
    oracleText,
    power,
    toughness,
    loyalty,
    defense,
  };
  return Number(
    game.createCustomCard({
      playerIndex: normalizePlayerId(player),
      zoneName: zone,
      skipTriggers,
      draft: {
        layout: "single",
        faces: [face],
      },
    }),
  );
}

export function addCustomEffectTargetDestroy(game, options = {}) {
  return addCustomCardWithAbility(game, {
    name: "Ironsmith Test Destroy Effect",
    manaCost: "{B}",
    typeLine: "Instant",
    oracleText: "Destroy target creature.",
    power: null,
    toughness: null,
    zone: "hand",
    ...options,
  });
}

export const addCustomEffect_TargetDestroy = addCustomEffectTargetDestroy;

export function concede(game, player) {
  const playerId = normalizePlayerId(player);
  return runCode(game, (checkpoint) => {
    const playerSnapshot = checkpoint.players.find((candidate) => Number(candidate.id) === playerId);
    assert(playerSnapshot, `unknown player ${playerId}`);
    playerSnapshot.hasLost = true;
    playerSnapshot.hasLeftGame = true;
  });
}

export function names(objects) {
  return objects.map((object) => object.name);
}

export function countByName(objects, name) {
  return objects.filter((object) => object.name === name).length;
}

function filterObjects(objects, nameOrPredicate) {
  if (typeof nameOrPredicate === "function") return objects.filter(nameOrPredicate);
  if (nameOrPredicate instanceof RegExp) {
    return objects.filter((object) => nameOrPredicate.test(object.name));
  }
  return objects.filter((object) => object.name === nameOrPredicate);
}

function idsForPlayerZone(player, zone) {
  if (zone === "hand") return player.hand || [];
  if (zone === "library") return player.library || [];
  if (zone === "graveyard") return player.graveyard || [];
  if (zone === "sideboard" || zone === "outside_game") return player.sideboard || [];
  throw new Error(`zone ${zone} is not player-local in the sync checkpoint`);
}

function normalizeZoneName(zone) {
  const normalized = String(zone).trim().toLowerCase().replaceAll(" ", "_");
  if (normalized === "outside_the_game" || normalized === "outside_game") return "outside_game";
  if (normalized === "battlefield" || normalized === "hand" || normalized === "library") return normalized;
  if (normalized === "graveyard" || normalized === "exile" || normalized === "command") return normalized;
  if (normalized === "stack" || normalized === "sideboard") return normalized;
  throw new Error(`unknown zone: ${zone}`);
}

function parseTypeLine(typeLine) {
  const [leftRaw, rightRaw = ""] = String(typeLine).split(/\s+[—-]\s+/, 2);
  const left = leftRaw.trim().split(/\s+/).filter(Boolean);
  const supertypes = [];
  const cardTypes = [];
  for (const word of left) {
    if (["Basic", "Legendary", "Snow", "World", "Ongoing"].includes(word)) {
      supertypes.push(word);
    } else {
      cardTypes.push(word);
    }
  }
  return {
    supertypes,
    cardTypes,
    subtypes: rightRaw.trim() ? rightRaw.trim().split(/\s+/).filter(Boolean) : [],
  };
}

function describeQuery(query) {
  if (typeof query === "function") return "predicate";
  return String(query);
}

function isGameLike(value) {
  return value && typeof value.exportSyncCheckpoint === "function";
}
