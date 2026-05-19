import initWasm, { WasmGame } from "../../../wasm_demo/pkg/ironsmith.js";
import wasmUrl from "../../../wasm_demo/pkg/ironsmith_bg.wasm?url";

const WASM_ESTIMATED_SIZE = 12_500_000; // ~12MB fallback estimate
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

let game = null;
let callQueue = Promise.resolve();
let pendingCallCount = 0;
let backgroundCompileDone = false;
let backgroundCompileTimer = null;
let lastRegistryLoaded = -1;
let lastRegistryTotal = -1;
let cardAssetsBaseUrl = null;
let cardIndexPromise = null;
const registeredCardRoutes = new Set();
const missingCardRoutes = new Set();
const knownRuntimeCardNames = new Set();
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
  "snapshot",
  "uiState",
]);
const CARD_ZONE_KEYS = [
  "battlefield",
  "battlefield_cards",
  "command_zone_cards",
  "exile_cards",
  "graveyard_cards",
  "hand_cards",
  "library_cards",
  "stack",
];

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
    .replace(/[^a-z0-9]+/g, "-")
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
    if (!object || typeof object !== "object" || object.hiddenCard || object.hidden_card) {
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
    const hidden = object.hiddenCard || object.hidden_card || null;
    if (name && !isToken && !hidden && name.toLocaleLowerCase("en-US") !== "hidden card") {
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
    case "revealHiddenObject":
    case "revealHiddenSlot":
    case "revealHiddenPosition":
      names.push(args?.[0]?.cardName || args?.[0]?.card_name);
      break;
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
    names.push(...knownRuntimeCardNames);
  }
  return compactCardNameList(names);
}

async function loadCardIndex() {
  if (!cardAssetsBaseUrl) {
    return null;
  }
  if (!cardIndexPromise) {
    cardIndexPromise = fetch(new URL("index.json", cardAssetsBaseUrl).href, {
      cache: "force-cache",
    }).then(async (response) => {
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

async function fetchCardSource(name) {
  const route = cardRouteKey(name);
  if (!route || registeredCardRoutes.has(route) || missingCardRoutes.has(route)) {
    return null;
  }
  if (cardNameAlreadyKnown(name)) {
    registeredCardRoutes.add(route);
    return null;
  }
  const url = cardAssetUrl(route);
  if (!url) return null;
  const response = await fetch(url, { cache: "force-cache" });
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
  registeredCardRoutes.add(route);
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
    if (sourceRoute) registeredCardRoutes.add(sourceRoute);
  }
  return payload;
}

function registerFetchedCardSources(sources) {
  if (!game || !Array.isArray(sources) || sources.length === 0) {
    return null;
  }
  if (typeof game.registerExternalCardSourcesJson === "function") {
    const raw = game.registerExternalCardSourcesJson(JSON.stringify(sources));
    return raw ? JSON.parse(raw) : null;
  }
  if (typeof game.registerExternalCardSources === "function") {
    return game.registerExternalCardSources(sources);
  }
  return null;
}

async function ensureCardSourcesForNames(names) {
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
  registerFetchedCardSources(sources);
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
    const status = await game.preloadRegistryChunk(16);
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

async function fetchWasmWithProgress(url, onProgress) {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) throw new Error(`WASM fetch failed: HTTP ${response.status}`);

  const contentLength = response.headers.get("content-length");
  const parsedTotal = contentLength ? Number.parseInt(contentLength, 10) : NaN;
  const total =
    Number.isFinite(parsedTotal) && parsedTotal > 0
      ? parsedTotal
      : WASM_ESTIMATED_SIZE;

  if (!response.body) {
    const body = await response.arrayBuffer();
    onProgress(1);
    return {
      wasmResponse: new Response(body, {
        headers: { "content-type": "application/wasm" },
      }),
      downloadDone: Promise.resolve(),
    };
  }

  const [progressBody, wasmBody] = response.body.tee();

  const downloadDone = (async () => {
    const reader = progressBody.getReader();
    let received = 0;
    let lastReported = 0;

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      received += value.byteLength;
      const next = Math.min(received / total, 1);
      if (next - lastReported >= 0.005 || next === 1) {
        onProgress(next);
        lastReported = next;
      }
    }
    onProgress(1);
  })();

  return {
    wasmResponse: new Response(wasmBody, {
      headers: { "content-type": "application/wasm" },
    }),
    downloadDone,
  };
}

async function handleInit(msg = {}) {
  try {
    clearBackgroundTimer();
    game = null;
    pendingCallCount = 0;
    backgroundCompileDone = false;
    lastRegistryLoaded = -1;
    lastRegistryTotal = -1;
    cardIndexPromise = null;
    knownRuntimeCardNames.clear();
    registeredCardRoutes.clear();
    missingCardRoutes.clear();
    const assetBaseUrl = String(msg.assetBaseUrl || "").trim();
    cardAssetsBaseUrl = assetBaseUrl ? new URL("cards/", assetBaseUrl).href : null;
    postProgress("module", 0);

    postProgress("download", 0);
    const bust = `v=${Date.now()}-${Math.floor(Math.random() * 1e9)}`;
    const { wasmResponse, downloadDone } = await fetchWasmWithProgress(
      `${wasmUrl}?${bust}`,
      (p) => postProgress("download", p)
    );

    await downloadDone;
    postProgress("init", 1);
    await initWasm(wasmResponse);
    game = new WasmGame();
    const status = readRegistryStatus();
    if (status) {
      postRegistryStatus(status, true);
      backgroundCompileDone = Boolean(status?.done);
      if (!backgroundCompileDone) {
        scheduleBackgroundCompile(0);
      }
    }

    self.postMessage({ type: "ready" });
  } catch (err) {
    self.postMessage({ type: "error", error: serializeError(err) });
  }
}

function enqueueCall(task) {
  callQueue = callQueue.then(task, task);
  return callQueue;
}

function handleCall(msg) {
  const { id, method, args = [] } = msg;
  const enqueuedAt = nowMs();
  pendingCallCount += 1;
  enqueueCall(async () => {
    if (!game) throw new Error("Game is not initialized yet");
    const startedAt = nowMs();
    const queueWaitMs = startedAt - enqueuedAt;
    await ensureCardSourcesForNames(collectNamesForMethod(method, args));
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
    const fn = game[method];
    if (typeof fn !== "function") {
      throw new Error(`Unknown game method: ${method}`);
    }
    const wasmStartedAt = nowMs();
    const result = await fn.apply(game, args);
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
    if (SNAPSHOT_METHODS.has(method)) {
      const snapshotPerfStartedAt = nowMs();
      snapshotPerf = typeof game.lastSnapshotPerf === "function"
        ? await game.lastSnapshotPerf()
        : null;
      snapshotPerfReadMs = nowMs() - snapshotPerfStartedAt;
    }
    if (DISPATCH_TRACE_METHODS.has(method)) {
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
      self.postMessage({ type: "result", id, ok: true, result });
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
