import { createAsyncLimiter } from "../lib/bounded-async.js";
import { createSnapshotEncoder } from "../lib/snapshot-channel.js";
import { replayTrustedMatch, replayTrustedActions } from "../lib/relay/replay-trusted-match.js";
import { compileWasmWithProgress } from "../lib/wasm-loading.js";
import { createAdaptiveWorkBudget } from "../lib/adaptive-work-budget.js";
import { createPriorityAnalysisScheduler } from "../lib/priority-analysis-scheduler.js";
import initWasm, { WasmGame } from "../../../wasm_demo/pkg/ironsmith.js";
import engineWasmUrl from "../../../wasm_demo/pkg/engine_bg.wasm?url";

const WASM_ESTIMATED_SIZE = 40_000_000;
const DEMO_CARD_NAMES = [
  "Plains",
  "Island",
  "Swamp",
  "Mountain",
  "Forest",
  "Lightning Bolt",
  "Counterspell",
  "Giant Growth",
  "Opt",
  "Divination",
  "Llanowar Elves",
  "Grizzly Bears",
  "Ornithopter",
  "Serra Angel",
  "Doom Blade",
  "Raise Dead",
  "Unsummon",
];

const snapshotEncoder = createSnapshotEncoder();
let game = null;
let callQueue = Promise.resolve();
let pendingCallCount = 0;
let backgroundCompileDone = false;
let backgroundCompileTimer = null;
const preloadBudget = createAdaptiveWorkBudget({ initial: 1, max: 16 });
let lastRegistryLoaded = -1;
let lastRegistryTotal = -1;
let cardAssetsBaseUrl = null;
let cardIndexPromise = null;
const registeredCardRoutes = new Set();
const previewCardSources = new Map();
let latestTargetPreview;
let previewWorker = null;
const targetPreviews = new Map();
let engineModule = null;
const missingCardRoutes = new Set();
const fetchSource = createAsyncLimiter(8);
const sourceRequests = new Map();
const knownRuntimeCardNames = new Set();
const STABLE_CARD_ASSET_FETCH_OPTIONS = { cache: "no-cache" };
const SNAPSHOT_METHODS = new Set([
  "advancePhase",
  "applyVerifiedHiddenLibraryShuffle",
  "cancelDecision",
  "dispatch",
  "forfeitPlayer",
  "importSyncCheckpoint",
  "injectTranscriptRandomSeeds",
  "revealHiddenObject",
  "revealHiddenPosition",
  "revealHiddenPositions",
  "revealHiddenSlot",
  "snapshot",
  "startMatch",
  "switchPerspective",
  "uiState",
]);
const DISPATCH_TRACE_METHODS = new Set([
  "advancePhase",
  "cancelDecision",
  "dispatch",
  "forfeitPlayer",
]);
const RUNTIME_EVALUATION_METHODS = new Set([
  "dispatch",
  "previewCryptoRequirements",
  "previewCastTargets",
  "snapshot",
  "uiState",
]);
const CARD_ZONE_KEYS = [
  "battlefield",
  "battlefield_cards",
  "ante_cards",
  "command_zone_cards",
  "exile_cards",
  "graveyard_cards",
  "hand_cards",
  "library_cards",
  "persistent_look_cards",
  "stack",
];

// Unknown methods invalidate by default. Presentation reads cannot cancel a
// long search merely because the user hovered a card or requested a snapshot.
const ANALYSIS_READ_METHOD = /^(snapshot|snapshotJson|uiState|last\w*Perf|lastWorkCounters|export\w+|autocompleteCardNames|get\w+|cardsMeetingThreshold|objectDetails|inspectorActions|preview\w+|registrySize|filterKnownCardNames|isKnownCardName|cardLoadDiagnostics|validateMatchConfig|createRuntimeSavepoint|releaseRuntimeSavepoint)$/;
let priorityIdentity = null;
let priorityViewRevision = 0;
const priorityAnalysis = createPriorityAnalysisScheduler({
  game: () => game,
  busy: () => pendingCallCount > 0,
  enqueue: enqueueCall,
  publish: (analysis) => {
    priorityIdentity = game.priorityAnalysisIdentity();
    self.postMessage({ type: "priorityAnalysis", ...analysis });
  },
  fail: ({ revision, error }) => self.postMessage({ type: "priorityAnalysisError", revision, error: serializeError(error) }),
});

function nowMs() {
  return performance.now();
}

function clampMs(value) {
  return Number.isFinite(value) ? Math.max(0, value) : 0;
}

function decorateResultWithPerf(result, perf) {
  if (!result || typeof result !== "object" || Array.isArray(result)) {
    return result;
  }
  return {
    ...result,
    __perf: perf,
  };
}

function serializeError(err) {
  if (err instanceof Error) {
    return {
      name: err.name,
      message: err.message,
      stack: err.stack,
    };
  }
  return {
    name: "Error",
    message: String(err),
  };
}

function postProgress(phase, progress) {
  self.postMessage({ type: "progress", phase, progress });
}

function normalizeRegistryStatus(raw) {
  const loaded = Number(raw?.loaded ?? 0);
  const total = Number(raw?.total ?? 0);
  const done = Boolean(raw?.done);
  return {
    loaded: Number.isFinite(loaded) ? Math.max(0, Math.floor(loaded)) : 0,
    total: Number.isFinite(total) ? Math.max(0, Math.floor(total)) : 0,
    done,
  };
}

function readRegistryStatus() {
  if (!game || typeof game.preloadRegistryStatus !== "function") {
    return null;
  }
  return game.preloadRegistryStatus();
}

function cardRouteKey(name) {
  const normalized = String(name || "")
    .trim()
    .toLocaleLowerCase("en-US")
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9_]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return normalized || "";
}

function cardAssetUrl(route) {
  if (!cardAssetsBaseUrl) {
    return null;
  }
  return new URL(`${route}.json`, cardAssetsBaseUrl).href;
}

function cardNameAlreadyKnown(name) {
  if (!game || typeof game.isKnownCardName !== "function") {
    return false;
  }
  try {
    return Boolean(game.isKnownCardName(String(name || "")));
  } catch {
    return false;
  }
}

function compactCardNameList(names) {
  const out = [];
  const seen = new Set();
  for (const raw of names || []) {
    const name = String(raw || "").trim();
    if (!name) continue;
    const key = name.toLocaleLowerCase("en-US");
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(name);
  }
  return out;
}

function rememberRuntimeCardName(rawName) {
  const name = String(rawName || "").trim();
  if (
    !name
    || /^hidden card$/i.test(name)
    || /^card details unavailable$/i.test(name)
  ) {
    return;
  }
  knownRuntimeCardNames.add(name);
  if (knownRuntimeCardNames.size > 1024) {
    const first = knownRuntimeCardNames.values().next();
    if (!first.done) knownRuntimeCardNames.delete(first.value);
  }
}

function rememberCardNamesFromZoneCards(cards) {
  if (!Array.isArray(cards)) return;
  for (const card of cards) {
    if (!card || typeof card !== "object") continue;
    rememberRuntimeCardName(card.name);
  }
}

function rememberCardNamesFromEngineResult(value) {
  if (!value || typeof value !== "object") return;
  if (typeof value.name === "string" && (
    typeof value.oracle_text === "string"
    || typeof value.type_line === "string"
    || Array.isArray(value.actions)
    || value.stable_id != null
    || value.stableId != null
  )) {
    rememberRuntimeCardName(value.name);
  }
  const players = Array.isArray(value.players) ? value.players : [];
  for (const player of players) {
    if (!player || typeof player !== "object") continue;
    for (const key of CARD_ZONE_KEYS) {
      rememberCardNamesFromZoneCards(player[key]);
    }
  }
  for (const key of CARD_ZONE_KEYS) {
    rememberCardNamesFromZoneCards(value[key]);
  }
  rememberCardNamesFromZoneCards(value?.viewed_cards?.cards);
  rememberCardNamesFromZoneCards(value?.active_viewed_cards?.cards);
  const objects = Array.isArray(value.objects) ? value.objects : [];
  for (const object of objects) {
    if (!object || typeof object !== "object") {
      continue;
    }
    rememberRuntimeCardName(object.name);
  }
}

function collectDeckNames(payload, out = []) {
  if (Array.isArray(payload)) {
    if (payload.every((entry) => typeof entry === "string")) {
      out.push(...payload);
      return out;
    }
    for (const entry of payload) collectDeckNames(entry, out);
    return out;
  }
  if (!payload || typeof payload !== "object") {
    return out;
  }
  collectDeckNames(payload.decks, out);
  collectDeckNames(payload.sideboards, out);
  collectDeckNames(payload.commanders, out);
  return out;
}

function collectCheckpointCardNames(checkpoint, out = []) {
  if (!checkpoint || typeof checkpoint !== "object") {
    return out;
  }
  const objects = Array.isArray(checkpoint.objects) ? checkpoint.objects : [];
  for (const object of objects) {
    if (!object || typeof object !== "object") continue;
    const name = String(object.name || "").trim();
    const isToken = Boolean(object.token);
    if (name && !isToken && name.toLocaleLowerCase("en-US") !== "hidden card") {
      out.push(name);
    }
  }
  return out;
}

function collectNamesForMethod(method, args) {
  const names = [];
  switch (method) {
    case "reset":
    case "loadDemoDecks":
      names.push(...DEMO_CARD_NAMES);
      break;
    case "filterKnownCardNames":
      names.push(...(Array.isArray(args?.[0]) ? args[0] : []));
      break;
    case "replayTrustedMatch":
    case "startMatch": {
      const config = args?.[0] || {};
      collectDeckNames(config, names);
      if (!config?.decks) names.push(...DEMO_CARD_NAMES);
      break;
    }
    case "validateMatchConfig":
      collectDeckNames(args?.[0] || {}, names);
      break;
    case "loadDecks":
      collectDeckNames(args?.[0], names);
      break;
    case "addCardToHand":
    case "addCardToZone":
      names.push(args?.[1]);
      break;
    case "addCardsToZones": {
      const cards = Array.isArray(args?.[0]) ? args[0] : [];
      for (const card of cards) {
        names.push(card?.cardName || card?.card_name);
      }
      break;
    }
    case "revealHiddenObject":
    case "revealHiddenSlot":
    case "revealHiddenPosition":
      names.push(args?.[0]?.cardName || args?.[0]?.card_name);
      break;
    case "revealHiddenPositions": {
      const input = args?.[0];
      const reveals = Array.isArray(input) ? input : input?.reveals;
      if (Array.isArray(reveals)) {
        for (const reveal of reveals) {
          names.push(reveal?.cardName || reveal?.card_name);
        }
      }
      break;
    }
    case "cardLoadDiagnostics":
    case "getCardSemanticScore":
    case "isKnownCardName":
      names.push(args?.[0]);
      break;
    case "importSyncCheckpoint":
      collectCheckpointCardNames(args?.[0], names);
      break;
    case "dispatch": {
      const command = args?.[0] || {};
      if (command?.type === "text_choice") {
        names.push(command.value);
      }
      break;
    }
    default:
      break;
  }
  if (RUNTIME_EVALUATION_METHODS.has(method)) {
    for (const name of knownRuntimeCardNames) {
      if (!registeredCardRoutes.has(cardRouteKey(name))) names.push(name);
    }
  }
  return compactCardNameList(names);
}

async function loadCardIndex() {
  if (!cardAssetsBaseUrl) {
    return null;
  }
  if (!cardIndexPromise) {
    cardIndexPromise = fetch(
      new URL("index.json", cardAssetsBaseUrl).href,
      STABLE_CARD_ASSET_FETCH_OPTIONS
    ).then(async (response) => {
      if (!response.ok) {
        throw new Error(`Card index fetch failed: HTTP ${response.status}`);
      }
      const index = await response.json();
      const cards = Array.isArray(index.cards) ? index.cards : [];
      const normalizedCards = cards.map((card) => {
        const name = String(card?.name || "").trim();
        return {
          name,
          lower: name.toLocaleLowerCase("en-US"),
          route: String(card?.route || cardRouteKey(name)),
          score: Number.isFinite(Number(card?.score)) ? Number(card.score) : null,
        };
      }).filter((card) => card.name);
      return {
        ...index,
        cards: normalizedCards,
      };
    });
  }
  return cardIndexPromise;
}

function fetchCardSource(name) {
  const route = cardRouteKey(name);
  if (!sourceRequests.has(route)) {
    const request = fetchSource(() => fetchCardSourceUncached(name)).finally(() => sourceRequests.delete(route));
    sourceRequests.set(route, request);
  }
  return sourceRequests.get(route);
}
async function fetchCardSourceUncached(name) {
  const route = cardRouteKey(name);
  if (!route || registeredCardRoutes.has(route) || missingCardRoutes.has(route)) {
    return null;
  }
  if (cardNameAlreadyKnown(name)) {
    registeredCardRoutes.add(route);
    return null;
  }
  if (previewCardSources.has(route)) return previewCardSources.get(route);
  const url = cardAssetUrl(route);
  if (!url) return null;
  const response = await fetch(url, STABLE_CARD_ASSET_FETCH_OPTIONS);
  if (response.status === 404) {
    missingCardRoutes.add(route);
    return null;
  }
  if (!response.ok) {
    throw new Error(`Card source fetch failed for "${name}": HTTP ${response.status}`);
  }
  const contentType = String(response.headers.get("content-type") || "").toLowerCase();
  if (!contentType.includes("application/json")) {
    missingCardRoutes.add(route);
    return null;
  }
  let payload = null;
  try {
    payload = await response.json();
  } catch {
    missingCardRoutes.add(route);
    return null;
  }
  if (!payload || typeof payload !== "object" || !payload.group) {
    missingCardRoutes.add(route);
    return null;
  }
  previewCardSources.set(route, payload);
  const sourceNames = [
    payload?.canonicalName,
    payload?.group?.name,
    payload?.group?.combinedName,
    ...(Array.isArray(payload?.group?.faces)
      ? payload.group.faces.map((face) => face?.name)
      : []),
    ...(Array.isArray(payload?.aliases)
      ? payload.aliases.flatMap((alias) => [alias?.alias, alias?.canonical])
      : []),
  ];
  for (const sourceName of sourceNames) {
    const sourceRoute = cardRouteKey(sourceName);
    if (sourceRoute) previewCardSources.set(sourceRoute, payload);
  }
  return payload;
}

function registerFetchedCardSources(sources) {
  if (!game || !Array.isArray(sources) || sources.length === 0) {
    return null;
  }
  if (typeof game.registerExternalCardSourcesJson === "function") {
    try {
      const raw = game.registerExternalCardSourcesJson(JSON.stringify(sources));
      return raw ? JSON.parse(raw) : null;
    } catch (error) {
      const summary = { loaded: 0, failed: [] };
      for (const source of sources) {
        try {
          const raw = game.registerExternalCardSourcesJson(JSON.stringify(source));
          const registered = raw ? JSON.parse(raw) : null;
          summary.loaded += Number(registered?.loaded || 0);
          if (Array.isArray(registered?.failed)) {
            summary.failed.push(...registered.failed);
          }
        } catch (sourceError) {
          summary.failed.push({
            name: String(
              source?.canonicalName
                || source?.group?.name
                || source?.group?.combinedName
                || "unknown card source"
            ),
            error: String(sourceError?.message || sourceError || error),
          });
        }
      }
      return summary;
    }
  }
  if (typeof game.registerExternalCardSources === "function") {
    try {
      return game.registerExternalCardSources(sources);
    } catch (error) {
      const summary = { loaded: 0, failed: [] };
      for (const source of sources) {
        try {
          const registered = game.registerExternalCardSources(source);
          summary.loaded += Number(registered?.loaded || 0);
          if (Array.isArray(registered?.failed)) {
            summary.failed.push(...registered.failed);
          }
        } catch (sourceError) {
          summary.failed.push({
            name: String(
              source?.canonicalName
                || source?.group?.name
                || source?.group?.combinedName
                || "unknown card source"
            ),
            error: String(sourceError?.message || sourceError || error),
          });
        }
      }
      return summary;
    }
  }
  return null;
}

async function prepareCardSourcesForNames(names) {
  if (
    !game
    || (
      typeof game.registerExternalCardSourcesJson !== "function"
      && typeof game.registerExternalCardSources !== "function"
    )
  ) {
    return;
  }
  const uniqueNames = compactCardNameList(names);
  if (uniqueNames.length === 0) return;
  const sources = (await Promise.all(uniqueNames.map(fetchCardSource))).filter(Boolean);
  if (sources.length === 0) return;
  return sources;
}

async function currentSemanticThreshold() {
  if (!game || typeof game.getSemanticThreshold !== "function") return 0;
  const raw = game.getSemanticThreshold();
  const percent = Number(raw);
  return Number.isFinite(percent) ? Math.max(0, percent / 100) : 0;
}

async function autocompleteFromCardIndex(query, limit) {
  const trimmed = String(query || "").trim();
  if (!trimmed) return [];
  const index = await loadCardIndex();
  if (!index) return [];
  const queryLower = trimmed.toLocaleLowerCase("en-US");
  const cappedLimit = Math.max(1, Math.min(25, Math.floor(Number(limit) || 5)));
  const threshold = await currentSemanticThreshold();
  const matches = [];
  for (const card of index.cards) {
    if (threshold > 0 && card.score !== null && card.score < threshold) continue;
    let rank = null;
    if (card.lower === queryLower) {
      rank = 0;
    } else if (card.lower.startsWith(queryLower)) {
      rank = 1;
    } else if (card.lower.split(/\s+/).some((word) => word.startsWith(queryLower))) {
      rank = 2;
    } else if (card.lower.includes(queryLower)) {
      rank = 3;
    }
    if (rank === null) continue;
    matches.push([rank, card.name.length, card.name]);
  }
  matches.sort((left, right) => (
    left[0] - right[0]
    || left[1] - right[1]
    || left[2].localeCompare(right[2])
  ));
  return matches.slice(0, cappedLimit).map((entry) => entry[2]);
}

async function semanticScoreFromCardIndex(cardName) {
  const route = cardRouteKey(cardName);
  if (!route) return -1;
  const index = await loadCardIndex();
  const card = index?.cards?.find((entry) => entry.route === route);
  return card && card.score !== null ? card.score : -1;
}

async function cardsMeetingThresholdFromCardIndex() {
  const index = await loadCardIndex();
  if (!index) return 0;
  const threshold = await currentSemanticThreshold();
  if (threshold <= 0) {
    return Number(index.scoredCount || 0);
  }
  const thresholdCounts = Array.isArray(index.thresholdCounts) ? index.thresholdCounts : [];
  const thresholdIndex = Math.max(0, Math.min(99, Math.ceil(threshold * 100) - 1));
  return Number(thresholdCounts[thresholdIndex] || 0);
}

function postRegistryStatus(raw, force = false) {
  const status = normalizeRegistryStatus(raw);
  if (
    !force
    && status.loaded === lastRegistryLoaded
    && status.total === lastRegistryTotal
  ) {
    return;
  }
  lastRegistryLoaded = status.loaded;
  lastRegistryTotal = status.total;
  self.postMessage({
    type: "registry",
    loaded: status.loaded,
    total: status.total,
    done: status.done,
  });
}

function clearBackgroundTimer() {
  if (backgroundCompileTimer !== null) {
    self.clearTimeout(backgroundCompileTimer);
    backgroundCompileTimer = null;
  }
}

function scheduleBackgroundCompile(delay = 0) {
  if (backgroundCompileDone || !game || typeof game.preloadRegistryChunk !== "function") {
    return;
  }
  if (backgroundCompileTimer !== null) return;
  backgroundCompileTimer = self.setTimeout(async () => {
    backgroundCompileTimer = null;
    await runBackgroundCompileStep();
  }, delay);
}

async function runBackgroundCompileStep() {
  if (backgroundCompileDone || !game || typeof game.preloadRegistryChunk !== "function") {
    return;
  }
  if (pendingCallCount > 0) {
    scheduleBackgroundCompile(32);
    return;
  }
  try {
    const status = preloadBudget.run(units => game.preloadRegistryChunk(units));
    postRegistryStatus(status);
    if (status?.done) {
      backgroundCompileDone = true;
      return;
    }
  } catch (err) {
    self.postMessage({ type: "error", error: serializeError(err) });
    return;
  }
  scheduleBackgroundCompile(16);
}

async function handleInit(msg = {}) {
  try {
    clearBackgroundTimer();
    snapshotEncoder.reset();
    previewWorker?.terminate(); previewWorker = null; targetPreviews.clear();
    game = null;
    pendingCallCount = 0;
    backgroundCompileDone = false;
    lastRegistryLoaded = -1;
    lastRegistryTotal = -1;
    cardIndexPromise = null;
    knownRuntimeCardNames.clear();
    registeredCardRoutes.clear();
    previewCardSources.clear();
    missingCardRoutes.clear();
    const assetBaseUrl = String(msg.assetBaseUrl || "").trim();
    cardAssetsBaseUrl = assetBaseUrl ? new URL("cards/", assetBaseUrl).href : null;
    postProgress("module", 0);

    postProgress("download", 0);
    engineModule = await compileWasmWithProgress(engineWasmUrl,
      (p) => postProgress("download", p), { estimatedSize: WASM_ESTIMATED_SIZE });
    postProgress("init", 1);
    await initWasm({ engine: engineModule, compiler: false, verifier: false });
    game = new WasmGame();
    game.setDeferredPriorityAnalysis(true);
    const status = readRegistryStatus();
    if (status) {
      postRegistryStatus(status, true);
      backgroundCompileDone = Boolean(status?.done);
      if (!backgroundCompileDone) {
        scheduleBackgroundCompile(0);
      }
    }

    self.postMessage({ type: "ready", runtimeSavepoints: typeof game.createRuntimeSavepoint === "function" });
  } catch (err) {
    self.postMessage({ type: "error", error: serializeError(err) });
  }
}

function enqueueCall(task) {
  callQueue = callQueue.then(task, task);
  return callQueue;
}

function handleTargetPreview(id, args) {
  latestTargetPreview = id;
  for (const previous of targetPreviews.keys()) self.postMessage({ type: "result", id: previous, ok: true, result: null });
  targetPreviews.clear();
  previewWorker?.postMessage({ type: "cancel" });
  pendingCallCount++;
  enqueueCall(() => {
    if (!game) throw new Error("Game is not initialized yet");
    return { checkpoint: game.exportSyncCheckpoint(), identity: game.priorityAnalysisIdentity(), sources: [...new Map([...previewCardSources].map(([route, source]) => [source, route])).entries()].map(([source, route]) => [route, source]) };
  }).then(input => {
    if (id !== latestTargetPreview) { self.postMessage({ type: "result", id, ok: true, result: null }); return; }
    if (!previewWorker) {
      previewWorker = new Worker(new URL("./targetPreviewWorker.js", import.meta.url), { type: "module" });
      previewWorker.onmessage = ({ data }) => {
        const request = targetPreviews.get(data.id);
        if (!request) return;
        targetPreviews.delete(data.id);
        const current = game?.priorityAnalysisIdentity() === request.identity;
        self.postMessage(data.error && current
          ? { type: "result", id: data.id, ok: false, error: { message: data.error } }
          : { type: "result", id: data.id, ok: true, result: current ? data.result : null });
      };
      previewWorker.onerror = event => {
        for (const id of targetPreviews.keys()) self.postMessage({ type: "result", id, ok: false, error: { message: event.message } });
        targetPreviews.clear(); previewWorker.terminate(); previewWorker = null;
      };
    }
    targetPreviews.set(id, { identity: input.identity });
    previewWorker.postMessage({ type: "preview", id, module: engineModule,
      checkpoint: input.checkpoint, sources: input.sources, actions: args[0], perspective: args[1] });
  }).catch(error => self.postMessage({ type: "result", id, ok: false, error: serializeError(error) }))
    .finally(() => { pendingCallCount--; priorityAnalysis.start(priorityViewRevision); });
}

function handleCall(msg) {
  const { id, method, args = [] } = msg;
  if (method === "previewCastTargets") { handleTargetPreview(id, args); return; }
  if (!/^(snapshot|uiState|last\w*Perf|exportSyncCheckpoint|exportPublicAuditCheckpoint|autocompleteCardNames|getCardSemanticScore|cardsMeetingThreshold)$/.test(method)) {
    try {
      console.debug(`[ironsmith] worker call: ${method} ${JSON.stringify({ argumentCount: args.length, commandType: args[0]?.type })}`);
    } catch {
      console.debug(`[ironsmith] worker call: ${method}`);
    }
  }
  // A preview promise must never occupy the authoritative command queue.
  // Its bounded slices use that queue separately, yielding to game commands.
  if (method === "inspectorActions" && game) {
    priorityAnalysis.inspector(...args).then(result => {
      if (result && typeof result === "object" && "decision" in result) {
        self.postMessage({ type: "result", id, ok: true, snapshot: snapshotEncoder.encode(result, { full: method === "snapshot" }) });
      } else self.postMessage({ type: "result", id, ok: true, result });
    });
    return;
  }
  const enqueuedAt = nowMs();
  const preparation = prepareCardSourcesForNames(collectNamesForMethod(method, args))
    .then(sources => ({ sources }), error => ({ error }));
  pendingCallCount += 1;
  enqueueCall(async () => {
    if (!game) throw new Error("Game is not initialized yet");
    if (!ANALYSIS_READ_METHOD.test(method) && method !== "setPerspective") {
      priorityAnalysis.invalidate();
      latestTargetPreview = null;
      previewWorker?.postMessage({ type: "cancel" });
    }
    const startedAt = nowMs();
    const queueWaitMs = startedAt - enqueuedAt;
    const prepared = await preparation;
    if (prepared.error) throw prepared.error;
    if (prepared.sources?.length) {
      registerFetchedCardSources([...new Set(prepared.sources)]);
      for (const source of prepared.sources) {
        const names = [source.canonicalName, source.group?.name, source.group?.combinedName,
          ...(source.group?.faces || []).map(face => face.name)];
        for (const name of names) if (name && cardNameAlreadyKnown(name)) registeredCardRoutes.add(cardRouteKey(name));
      }
    }
    if (method === "autocompleteCardNames") {
      return {
        result: await autocompleteFromCardIndex(args[0], args[1]),
        registryStatus: readRegistryStatus(),
      };
    }
    if (method === "getCardSemanticScore") {
      return {
        result: await semanticScoreFromCardIndex(args[0]),
        registryStatus: readRegistryStatus(),
      };
    }
    if (method === "cardsMeetingThreshold") {
      return {
        result: await cardsMeetingThresholdFromCardIndex(),
        registryStatus: readRegistryStatus(),
      };
    }
    if (method === "filterKnownCardNames") {
      const names = compactCardNameList(args?.[0]);
      return {
        result: names.filter(cardNameAlreadyKnown),
        registryStatus: readRegistryStatus(),
      };
    }
    const replayOptions = { yieldControl: () => new Promise(resolve => setTimeout(resolve, 0)) };
    const fn = method === "replayTrustedMatch" ? (config, actions, perspective) => replayTrustedMatch(game, config, actions, perspective, replayOptions)
      : method === "replayTrustedActions" ? (actions, sequence) => replayTrustedActions(game, actions, sequence, replayOptions)
      : game[method];
    if (typeof fn !== "function") {
      throw new Error(`Unknown game method: ${method}`);
    }
    const previousPerspectiveIdentity = method === "setPerspective" ? game.priorityAnalysisIdentity() : null;
    const wasmStartedAt = nowMs();
    const result = await fn.apply(game, args);
    if (previousPerspectiveIdentity !== null && previousPerspectiveIdentity !== game.priorityAnalysisIdentity()) priorityAnalysis.invalidate();
    rememberCardNamesFromEngineResult(result);
    const wasmCallMs = nowMs() - wasmStartedAt;
    let snapshotPerf = null;
    let snapshotPerfReadMs = 0;
    let dispatchPerf = null;
    let dispatchPerfReadMs = 0;
    let replayExecutionPerf = null;
    let replayExecutionPerfReadMs = 0;
    let advanceUntilDecisionPerf = null;
    let advanceUntilDecisionPerfReadMs = 0;
    const sampleDetailedPerf = id % 16 === 0 || wasmCallMs >= 16;
    if (sampleDetailedPerf && SNAPSHOT_METHODS.has(method)) {
      const snapshotPerfStartedAt = nowMs();
      snapshotPerf = typeof game.lastSnapshotPerf === "function"
        ? await game.lastSnapshotPerf()
        : null;
      snapshotPerfReadMs = nowMs() - snapshotPerfStartedAt;
    }
    if (sampleDetailedPerf && DISPATCH_TRACE_METHODS.has(method)) {
      const dispatchPerfStartedAt = nowMs();
      dispatchPerf = typeof game.lastDispatchPerf === "function"
        ? await game.lastDispatchPerf()
        : null;
      dispatchPerfReadMs = nowMs() - dispatchPerfStartedAt;
      const replayExecutionPerfStartedAt = nowMs();
      replayExecutionPerf = typeof game.lastReplayExecutionPerf === "function"
        ? await game.lastReplayExecutionPerf()
        : null;
      replayExecutionPerfReadMs = nowMs() - replayExecutionPerfStartedAt;
      const advanceUntilDecisionPerfStartedAt = nowMs();
      advanceUntilDecisionPerf = typeof game.lastAdvanceUntilDecisionPerf === "function"
        ? await game.lastAdvanceUntilDecisionPerf()
        : null;
      advanceUntilDecisionPerfReadMs = nowMs() - advanceUntilDecisionPerfStartedAt;
    }
    const registryStatusStartedAt = nowMs();
    const registryStatus = readRegistryStatus();
    const registryStatusMs = nowMs() - registryStatusStartedAt;
    const totalWorkerMs = nowMs() - enqueuedAt;
    const snapshotTotalMs = Number(snapshotPerf?.totalSnapshotMs ?? 0);
    const perf = {
      method,
      queueWaitMs: clampMs(queueWaitMs),
      wasmCallMs: clampMs(wasmCallMs),
      snapshotPerfReadMs: clampMs(snapshotPerfReadMs),
      dispatchPerfReadMs: clampMs(dispatchPerfReadMs),
      replayExecutionPerfReadMs: clampMs(replayExecutionPerfReadMs),
      advanceUntilDecisionPerfReadMs: clampMs(advanceUntilDecisionPerfReadMs),
      registryStatusMs: clampMs(registryStatusMs),
      totalWorkerMs: clampMs(totalWorkerMs),
      estimatedEngineMs: clampMs(wasmCallMs - snapshotTotalMs),
      snapshot: snapshotPerf || null,
      dispatch: dispatchPerf || null,
      replayExecution: replayExecutionPerf || null,
      advanceUntilDecision: advanceUntilDecisionPerf || null,
    };
    return {
      result: decorateResultWithPerf(result, perf),
      registryStatus,
    };
  })
    .then(({ result, registryStatus }) => {
      if (registryStatus) {
        postRegistryStatus(registryStatus);
        if (!registryStatus.done) scheduleBackgroundCompile(0);
      }
      if (result && typeof result === "object" && "decision" in result) {
        const identity = game.priorityAnalysisIdentity();
        if (priorityIdentity !== identity) {
          priorityAnalysis.invalidate();
          priorityIdentity = identity;
          priorityViewRevision = priorityAnalysis.revision();
        }
        result.__priority_revision = priorityViewRevision;
        self.postMessage({ type: "result", id, ok: true, snapshot: snapshotEncoder.encode(result, { full: method === "snapshot" }) });
      } else self.postMessage({ type: "result", id, ok: true, result });
    })
    .catch((err) => {
      self.postMessage({
        type: "result",
        id,
        ok: false,
        error: serializeError(err),
      });
    })
    .finally(() => {
      pendingCallCount = Math.max(0, pendingCallCount - 1);
      // A rejected command can cancel a job without changing game state.
      // Resume against the still-visible snapshot instead of stranding its
      // pending menu behind an unpublished worker generation.
      const visibleRevision = game && game.priorityAnalysisIdentity() === priorityIdentity
        ? priorityViewRevision : priorityAnalysis.revision();
      priorityAnalysis.start(visibleRevision);
      if (!backgroundCompileDone) {
        scheduleBackgroundCompile(0);
      }
    });
}

self.addEventListener("message", (event) => {
  const msg = event.data || {};
  if (msg.type === "init") {
    handleInit(msg);
    return;
  }
  if (msg.type === "call") {
    handleCall(msg);
  }
});
