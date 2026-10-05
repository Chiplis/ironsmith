import { createLocalAnalysisJournal, releaseRestoredRuntimeSavepoints } from "../lib/local-analysis-replay.js";
import { createPaymentOptionsAnalysis } from "../lib/payment-options-analysis.js";
import { createAsyncLimiter } from "../lib/bounded-async.js";
import { inRuntimeBranch } from "../lib/runtime-branches.js";
import { CARD_ASSET_MISSING, fetchCardAssetJson, versionedCardAssetUrl } from "../lib/card-asset-cache.js";
import { createSnapshotEncoder } from "../lib/snapshot-channel.js";
import { previewCryptoRequirementsWithMaterial } from "../lib/preview-crypto-material.js";
import { replayTrustedMatch, replayTrustedActions } from "../lib/relay/replay-trusted-match.js";
import { compileWasmWithProgress } from "../lib/wasm-loading.js";
import { createExactBuildSnapshotRuntime } from '../lib/exact-build-snapshot.js';
import { publicCheckpointHash } from '../lib/multiplayer-audit.js';
import { exactSnapshotBuildId, exactSnapshotLayout, replaceEngineInstance, attachExactBuildGame } from '../../../wasm_demo/pkg/engine.js';
import { createAdaptiveWorkBudget } from "../lib/adaptive-work-budget.js";
import { createIsolatedPriorityAnalysis } from "../lib/isolated-priority-analysis.js";
import { createWorkerTaskDiagnostics } from "../lib/worker-task-diagnostics.js";
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
let localAnalysisJournal = null;
let localAnalysisEpoch = 0;
let callQueue = Promise.resolve();
let pendingCallCount = 0;
let backgroundCompileDone = false;
let backgroundCompileTimer = null;
const preloadBudget = createAdaptiveWorkBudget({ initial: 1, max: 16 });
let lastRegistryLoaded = -1;
let lastRegistryTotal = -1;
let cardAssetsBaseUrl = null;
let cardIndexPromise = null;
let embeddedCardIndex = null;
const registeredCardRoutes = new Set();
const previewCardSources = new Map();
let latestTargetPreview;
let previewWorker = null;
const targetPreviews = new Map();
let engineModule = null;
let engineExports = null;
let exactSnapshotRuntime = null;
let runtimeGeneration = 0;
// Routes the server answered with a definitive 404. Any other failure (SPA
// HTML fallback, captive portal, truncated cached body) is transient: it is
// only remembered briefly, so a later reveal of that card retries instead of
// failing "unknown card name" on this peer for the rest of the session.
const missingCardRoutes = new Set();
const transientMissingCardRoutes = new Map();
const TRANSIENT_CARD_SOURCE_MISS_MS = 15_000;
// Bound legacy HTTP loading; embedded catalog reads use the same preparation
// path and only decompress the chunks containing the requested cards.
const fetchSource = createAsyncLimiter(24);
const sourceRequests = new Map();
const knownRuntimeCardNames = new Set();
const SNAPSHOT_METHODS = new Set([
  "advancePhase",
  "applyVerifiedHiddenLibraryShuffle",
  "cancelDecision",
  "dispatch",
  "forfeitPlayer",
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
  "previewCryptoRequirementsWithMaterial",
  "previewCastTargets",
  "snapshot",
  "uiState",
]);
// Dungeon cards (CR 309) begin outside the game, so no deck names them. Any
// method that starts or rebuilds a game loads the baked dungeon routes the
// card index lists, so venturing into the dungeon has compiled rooms.
const DUNGEON_LOADING_METHODS = new Set([
  "loadDecks",
  "loadDemoDecks",
  "replayTrustedMatch",
  "reset",
  "startMatch",
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
const ANALYSIS_READ_METHOD = /^(beginPaymentAnalysis|stepPaymentAnalysis|cancelPaymentAnalysis|snapshot|snapshotJson|uiState|last\w*Perf|lastWorkCounters|export\w+|autocompleteCardNames|get\w+|cardsMeetingThreshold|objectDetails|inspectorActions|preview\w+|registrySize|filterKnownCardNames|isKnownCardName|hiddenCardOpenState|pendingVerifiedHiddenLibraryPosition|runtimeVersion|cardLoadDiagnostics|validateMatchConfig|captureExactBuildSnapshot|createRuntimeSavepoint|releaseRuntimeSavepoint)$/;
let priorityIdentity = null;
let priorityViewRevision = 0;
const workerTasks = createWorkerTaskDiagnostics({ publish: message => self.postMessage(message) });
const priorityAnalysis = createIsolatedPriorityAnalysis({
  identity: () => game?.priorityAnalysisIdentity(),
  pending: () => game?.priorityAnalysisPending?.() === true,
  eligible: () => game?.hasPriorityDecision?.() === true,
  capture: () => enqueueCall(() => ({
    localReplay: localAnalysisJournal.capture(),
    module: engineModule,
  }), { kind: 'priority_analysis_capture' }),
  createWorker: () => new Worker(new URL('./priorityAnalysisWorker.js', import.meta.url), { type: 'module' }),
  // Check results only after any temporary verification branch has exited.
  deliver: operation => enqueueCall(operation, { kind: 'priority_analysis_publish' }),
  publish: analysis => {
    if (analysis.decision?.analysis_complete) {
      game.rememberPriorityAffordability(analysis.decision.actions.map(action => action.action_ref));
    }
    self.postMessage({ type: 'priorityAnalysis', ...analysis });
  },
  fail: ({ revision, error }) => self.postMessage({ type: 'priorityAnalysisError', revision, error: serializeError(error) }),
});

const paymentOptionsAnalysis = createPaymentOptionsAnalysis({
  capture: (requestHash, planId) => enqueueCall(() => {
    const request = game.exportManaPaymentOptionsRequest(requestHash, planId);
    if (request === 'null') return null;
    return {
      request,
      localReplay: localAnalysisJournal.capture(),
      module: engineModule,
    };
  }, { kind: 'payment_options_capture' }),
  createWorker: () => new Worker(new URL('./paymentOptionsWorker.js', import.meta.url), { type: 'module' }),
});

function nowMs() {
  return performance.now();
}

function clampMs(value) {
  return Number.isFinite(value) ? Math.max(0, value) : 0;
}

function postWorkerResult(message) {
  self.postMessage({ ...message, sentAtWall: Date.now() });
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

// Cards outside the baked registry are compiled on demand as a game reaches
// them, so both of these grow during a session. Sampled only alongside the
// detailed perf read, which is already rate limited.
function readEngineMemoryBytes() {
  const buffer = engineExports?.memory?.buffer;
  return typeof buffer?.byteLength === "number" ? buffer.byteLength : null;
}

function readRegistrySize() {
  if (!game || typeof game.registrySize !== "function") return null;
  try {
    return Number(game.registrySize());
  } catch {
    return null;
  }
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
  return versionedCardAssetUrl(new URL(`${route}.json`, cardAssetsBaseUrl).href);
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
  // Verified matches have empty decks; their public lists still determine
  // which draws need a Miracle reveal window. Load them before setup so
  // eligibility does not depend on which player's cards this worker cached.
  collectDeckNames(payload.publicDecklists, out);
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
    case "queueVerifiedHiddenLibraryOpening":
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
    case "previewCryptoRequirementsWithMaterial":
      for (const opening of args?.[1]?.libraryEpochOpenings || []) names.push(opening.cardName);
      break;
    case "cardLoadDiagnostics":
    case "getCardSemanticScore":
    case "isKnownCardName":
      names.push(args?.[0]);
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
  if (!embeddedCardIndex && !cardAssetsBaseUrl) {
    return null;
  }
  if (!cardIndexPromise) {
    const indexPromise = (embeddedCardIndex ? Promise.resolve(embeddedCardIndex) : fetchCardAssetJson(
      versionedCardAssetUrl(new URL("index.json", cardAssetsBaseUrl).href),
      { validate: (value) => Boolean(value && typeof value === "object") }
    )).then((index) => {
      if (index === CARD_ASSET_MISSING) {
        throw new Error("Card index fetch failed: HTTP 404");
      }
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
    // Never pin a failed index load for the session; the next caller retries.
    indexPromise.catch(() => {
      if (cardIndexPromise === indexPromise) cardIndexPromise = null;
    });
    cardIndexPromise = indexPromise;
  }
  return cardIndexPromise;
}

async function dungeonCardNames() {
  try {
    const index = await loadCardIndex();
    const dungeons = Array.isArray(index?.dungeons) ? index.dungeons : [];
    return dungeons
      .map((dungeon) => String(dungeon?.name || "").trim())
      .filter((name) => name && !registeredCardRoutes.has(cardRouteKey(name)));
  } catch {
    return [];
  }
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
  const url = embeddedCardIndex ? null : cardAssetUrl(route);
  if (!embeddedCardIndex && !url) return null;
  const retryAt = transientMissingCardRoutes.get(route);
  if (retryAt != null) {
    if (Date.now() < retryAt) return null;
    transientMissingCardRoutes.delete(route);
  }
  let payload;
  try {
    if (embeddedCardIndex) {
      const raw = game.getEmbeddedCardSourceJson(route);
      payload = raw == null ? CARD_ASSET_MISSING : JSON.parse(raw);
      if (payload !== CARD_ASSET_MISSING && (!payload || typeof payload !== "object" || !payload.group)) {
        throw new Error(`Embedded card source is invalid for "${name}"`);
      }
    } else {
      payload = await fetchCardAssetJson(url, {
        validate: (value) => Boolean(value && typeof value === "object" && value.group),
      });
    }
  } catch (error) {
    if (!error?.cardAssetInvalidBody) {
      throw new Error(`Card source fetch failed for "${name}": ${error?.message || error}`);
    }
    console.warn(`[ironsmith] card source for "${name}" is temporarily unavailable`, error);
    transientMissingCardRoutes.set(route, Date.now() + TRANSIENT_CARD_SOURCE_MISS_MS);
    return null;
  }
  if (payload === CARD_ASSET_MISSING) {
    missingCardRoutes.add(route);
    return null;
  }
  previewCardSources.set(route, payload);
  // A prepare card's spell face copies an existing card (Raise Dead,
  // Lightning Bolt...), so only its creature face may claim a route here.
  const faces = Array.isArray(payload?.group?.faces) ? payload.group.faces : [];
  const claimedFaces = payload?.group?.layout === "prepare" ? faces.slice(0, 1) : faces;
  const sourceNames = [
    payload?.canonicalName,
    payload?.group?.name,
    payload?.group?.combinedName,
    ...claimedFaces.map((face) => face?.name),
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
    const status = workerTasks.runSync({ kind: 'registry_preload' }, () => preloadBudget.run(units => {
      workerTasks.phaseActive('registry_preload', { nodeBudget: units });
      return game.preloadRegistryChunk(units);
    }));
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
  workerTasks.reset();
  const task = workerTasks.create({ kind: 'initialization' });
  workerTasks.start(task);
  let outcome = 'error';
  try {
    clearBackgroundTimer();
    snapshotEncoder.reset();
    paymentOptionsAnalysis.cancel();
    previewWorker?.terminate(); previewWorker = null; targetPreviews.clear();
    game = null;
    pendingCallCount = 0;
    backgroundCompileDone = false;
    lastRegistryLoaded = -1;
    lastRegistryTotal = -1;
    cardIndexPromise = null;
    embeddedCardIndex = null;
    knownRuntimeCardNames.clear();
    registeredCardRoutes.clear();
    priorityAnalysis.dispose();
    previewCardSources.clear();
    missingCardRoutes.clear();
    transientMissingCardRoutes.clear();
    const assetBaseUrl = String(msg.assetBaseUrl || "").trim();
    cardAssetsBaseUrl = assetBaseUrl ? new URL("cards/", assetBaseUrl).href : null;
    postProgress("module", 0);

    postProgress("download", 0);
    workerTasks.phase(task, 'module_download');
    engineModule = await compileWasmWithProgress(engineWasmUrl,
      (p) => postProgress("download", p), { estimatedSize: WASM_ESTIMATED_SIZE });
    postProgress("init", 1);
    workerTasks.phase(task, 'wasm_initialization');
    // The engine's exports carry its linear memory. Its size over a session is
    // the one signal that separates "this call is expensive" from "this session
    // has grown expensive", which a single slow call cannot tell apart.
    engineExports = await initWasm({ engine: engineModule, compiler: false, verifier: false });
    exactSnapshotRuntime = createExactBuildSnapshotRuntime({ exports: engineExports,
      layout: exactSnapshotLayout, buildId: exactSnapshotBuildId,
      replace: replaceEngineInstance, attach: attachExactBuildGame });
    localAnalysisJournal = createLocalAnalysisJournal(new WasmGame(), ++localAnalysisEpoch);
    game = localAnalysisJournal.game;
    workerTasks.phase(task, 'catalog_load');
    if (typeof game.getEmbeddedCardCatalogIndexJson === "function") {
      const raw = game.getEmbeddedCardCatalogIndexJson();
      if (raw != null) {
        embeddedCardIndex = JSON.parse(raw);
        if (!Array.isArray(embeddedCardIndex?.cards)) {
          throw new Error("Embedded card catalog has no valid card index");
        }
      }
    }
    game.setDeferredPriorityAnalysis(true);
    game.setDeferredManaOptions?.(true);
    workerTasks.phase(task, 'registry_status');
    const status = readRegistryStatus();
    if (status) {
      postRegistryStatus(status, true);
      backgroundCompileDone = Boolean(status?.done);
      if (!backgroundCompileDone) {
        scheduleBackgroundCompile(0);
      }
    }

    workerTasks.phase(task, 'ready_post');
    self.postMessage({ type: "ready", runtimeSavepoints: typeof game.createRuntimeSavepoint === "function",
      exactBuildSnapshots: true,
      exactSnapshotBuildId,
      runtimeBranches: typeof game.exchangeRuntimeSavepoint === "function",
      embeddedCardCatalog: embeddedCardIndex !== null });
    outcome = 'ok';
  } catch (err) {
    self.postMessage({ type: "error", error: serializeError(err) });
  } finally { workerTasks.finish(task, outcome); }
}

function enqueueCall(operation, metadata = { kind: 'background_analysis' }, retainedTask = null) {
  const task = retainedTask || workerTasks.create(metadata);
  workerTasks.enqueue(task);
  const run = async () => {
    workerTasks.start(task);
    let outcome = 'error';
    try { const result = await operation(); outcome = 'ok'; return result; }
    finally {
      workerTasks.leaveQueue(task);
      if (!retainedTask) workerTasks.finish(task, outcome);
    }
  };
  callQueue = callQueue.then(run, run);
  return callQueue;
}

function handleTargetPreview(id, args) {
  const task = workerTasks.create({ kind: 'target_preview', requestId: id, method: 'previewCastTargets' });
  const respond = (task, message) => {
    workerTasks.phase(task, 'response_post'); postWorkerResult(message);
    workerTasks.finish(task, message.ok ? 'ok' : 'error');
  };
  latestTargetPreview = id;
  for (const [previous, request] of targetPreviews) respond(request.task, { type: "result", id: previous, ok: true, result: null });
  targetPreviews.clear();
  previewWorker?.postMessage({ type: "cancel" });
  pendingCallCount++;
  enqueueCall(() => {
    workerTasks.phase(task, 'target_checkpoint');
    if (!game) throw new Error("Game is not initialized yet");
    return { localReplay: localAnalysisJournal.capture(), identity: game.priorityAnalysisIdentity() };
  }, {}, task).then(input => {
    if (id !== latestTargetPreview) { respond(task, { type: "result", id, ok: true, result: null }); return; }
    if (!previewWorker) {
      previewWorker = new Worker(new URL("./targetPreviewWorker.js", import.meta.url), { type: "module" });
      previewWorker.onmessage = ({ data }) => {
        const request = targetPreviews.get(data.id);
        if (!request) return;
        targetPreviews.delete(data.id);
        const current = game?.priorityAnalysisIdentity() === request.identity;
        respond(request.task, data.error && current
          ? { type: "result", id: data.id, ok: false, error: { message: data.error } }
          : { type: "result", id: data.id, ok: true, result: current ? data.result : null });
      };
      previewWorker.onerror = event => {
        for (const [id, request] of targetPreviews) respond(request.task, { type: "result", id, ok: false, error: { message: event.message } });
        targetPreviews.clear(); previewWorker.terminate(); previewWorker = null;
      };
    }
    targetPreviews.set(id, { identity: input.identity, task });
    workerTasks.phase(task, 'target_worker_wait');
    previewWorker.postMessage({ type: "preview", id, module: engineModule,
      localReplay: input.localReplay, actions: args[0], perspective: args[1] });
  }).catch(error => respond(task, { type: "result", id, ok: false, error: serializeError(error) }))
    .finally(() => { pendingCallCount--; priorityAnalysis.start(priorityViewRevision); });
}

function handleCall(msg) {
  const { id, method, args = [] } = msg;
  if (msg.runtimeBranch == null && method === "getPaymentActivationOptions") {
    paymentOptionsAnalysis.run(...args).then(result =>
      postWorkerResult({ type: "result", id, ok: true, result }), error =>
      postWorkerResult({ type: "result", id, ok: false, error: serializeError(error) }));
    return;
  }
  if (msg.runtimeBranch == null && method === "previewCastTargets") { handleTargetPreview(id, args); return; }
  if (!/^(snapshot|uiState|last\w*Perf|exportPublicAuditCheckpoint|autocompleteCardNames|getCardSemanticScore|cardsMeetingThreshold)$/.test(method)) {
    try {
      console.debug(`[ironsmith] worker call: ${method} ${JSON.stringify({ argumentCount: args.length, commandType: args[0]?.type })}`);
    } catch {
      console.debug(`[ironsmith] worker call: ${method}`);
    }
  }
  // A preview promise must never occupy the authoritative command queue.
  // Its analysis runs on a separate worker against the captured state.
  if (msg.runtimeBranch == null && method === "inspectorActions" && game) {
    const task = workerTasks.create({ kind: 'inspector_request', requestId: id, method });
    workerTasks.phase(task, 'analysis_wait');
    priorityAnalysis.inspector(...args).then(result => {
      if (result && typeof result === "object" && "decision" in result) {
        workerTasks.phase(task, 'snapshot_encode');
        const snapshot = snapshotEncoder.encode(result);
        workerTasks.phase(task, 'response_post');
        postWorkerResult({ type: "result", id, ok: true, snapshot });
      } else {
        workerTasks.phase(task, 'response_post'); postWorkerResult({ type: "result", id, ok: true, result });
      }
      workerTasks.finish(task);
    }).catch(error => {
      postWorkerResult({ type: 'result', id, ok: false, error: serializeError(error) });
      workerTasks.finish(task, 'error');
    });
    return;
  }
  const enqueuedAt = nowMs();
  const diagnosticTask = workerTasks.create({ requestId: id, method,
    commandType: args[0]?.type, runtimeBranch: msg.runtimeBranch });
  const preparation = (DUNGEON_LOADING_METHODS.has(method) ? dungeonCardNames() : Promise.resolve([]))
    .then(dungeons => prepareCardSourcesForNames([...collectNamesForMethod(method, args), ...dungeons]))
    .then(sources => ({ sources }), error => ({ error }))
    .then(prepared => { workerTasks.prepared(diagnosticTask, enqueuedAt); return prepared; });
  pendingCallCount += 1;
  enqueueCall(async () => {
    workerTasks.phase(diagnosticTask, 'preparation_wait');
    const prepared = await preparation;
    if (prepared.error) throw prepared.error;
    if (method === 'restoreExactBuildSnapshot') {
      if (msg.runtimeBranch != null) throw new Error('Cannot restore an instance inside a runtime branch');
      runtimeGeneration++;
    }
    if (msg.runtimeGeneration != null && msg.runtimeGeneration !== runtimeGeneration) throw new Error('Engine instance has expired');
    return inRuntimeBranch(game, msg.runtimeBranch, async () => {
    if (!game) throw new Error("Game is not initialized yet");
    if (method === 'captureExactBuildSnapshot') {
      if (msg.runtimeBranch != null) throw new Error('Cannot capture an instance inside a runtime branch');
      if (args[0]?.publicStateHash && await publicCheckpointHash(game.exportPublicAuditCheckpoint(), globalThis.crypto) !== args[0].publicStateHash) {
        throw new Error('Exact snapshot does not match the accepted public state');
      }
      const result = await exactSnapshotRuntime.capture(game, {
        journal: localAnalysisJournal.capture(), routes: [...registeredCardRoutes],
        cardNames: [...knownRuntimeCardNames], metadata: args[0],
      });
      return { result, registryStatus: null };
    }
    if (method === 'restoreExactBuildSnapshot') {
      priorityAnalysis.invalidate(); paymentOptionsAnalysis.cancel();
      latestTargetPreview = null; previewWorker?.postMessage({ type: 'cancel' });
      let result;
      try {
        const runtime = await exactSnapshotRuntime.restore(args[0], game);
        game = runtime;
        localAnalysisJournal = createLocalAnalysisJournal(runtime, ++localAnalysisEpoch, args[0].recovery.journal);
        game = localAnalysisJournal.game;
        releaseRestoredRuntimeSavepoints(localAnalysisJournal);
        registeredCardRoutes.clear(); knownRuntimeCardNames.clear();
        for (const route of args[0].recovery.routes) registeredCardRoutes.add(route);
        for (const name of args[0].recovery.cardNames) knownRuntimeCardNames.add(name);
        result = game.uiState();
      } catch (error) {
        // Invalid bytes must never be used as the starting point for genesis.
        // Discard the entire instance, including any partially restored heap.
        if (game.__wbg_ptr) game.__destroy_into_raw();
        exactSnapshotRuntime.reset();
        localAnalysisJournal = createLocalAnalysisJournal(new WasmGame(), ++localAnalysisEpoch);
        game = localAnalysisJournal.game;
        registeredCardRoutes.clear(); knownRuntimeCardNames.clear();
        throw error;
      } finally { engineExports = exactSnapshotRuntime.exports; }
      return { result, registryStatus: readRegistryStatus() };
    }
    if (msg.runtimeBranch == null && !ANALYSIS_READ_METHOD.test(method) && method !== "setPerspective") {
      // Publishing a verified branch may copy the identical visible state.
      // Compare its analysis identity after the copy instead of discarding work.
      if (method !== "copyRuntimeSavepoint") priorityAnalysis.invalidate();
      paymentOptionsAnalysis.cancel();
      game?.cancelPaymentAnalysis?.();
      latestTargetPreview = null;
      previewWorker?.postMessage({ type: "cancel" });
    }
    const startedAt = nowMs();
    const queueWaitMs = startedAt - enqueuedAt;
    if (prepared.sources?.length) {
      workerTasks.phase(diagnosticTask, 'card_registration');
      const registration = registerFetchedCardSources([...new Set(prepared.sources)]);
      if (registration?.failed?.length) {
        console.warn("[ironsmith] on-demand card registration warnings", registration.failed);
      }
      for (const source of prepared.sources) {
        const names = [source.canonicalName, source.group?.name, source.group?.combinedName,
          ...(source.group?.faces || []).map(face => face.name)];
        for (const name of names) if (name && cardNameAlreadyKnown(name)) registeredCardRoutes.add(cardRouteKey(name));
        // Dungeons live in the engine's dungeon catalog, not the card registry.
        const failed = new Set((registration?.failed || []).map(failure => failure?.name));
        if (source.group?.kind === "dungeon" && !failed.has(source.group.name)) {
          registeredCardRoutes.add(cardRouteKey(source.group.name));
        }
      }
    }
    workerTasks.phase(diagnosticTask, 'engine_call');
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
    // Cards outside the baked registry are fetched and registered by this
    // worker, not by an engine call, so they never appear in the journal. A
    // replay has to register the same ones before it can replay anything that
    // used them; the routes are enough for a harness to load them from the
    // card asset tree.
    if (method === "getExternalCardRoutes") {
      return {
        result: [...registeredCardRoutes],
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
      : method === "previewCryptoRequirementsWithMaterial" ? (command, material) => previewCryptoRequirementsWithMaterial(game, command, material)
      : game[method];
    if (typeof fn !== "function") {
      throw new Error(`Unknown game method: ${method}`);
    }
    const previousViewIdentity = (method === "setPerspective" || method === "copyRuntimeSavepoint")
      && msg.runtimeBranch == null ? game.priorityAnalysisIdentity() : null;
    const wasmStartedAt = nowMs();
    const result = await fn.apply(game, args);
    if (previousViewIdentity !== null && previousViewIdentity !== game.priorityAnalysisIdentity()) {
      priorityAnalysis.invalidate();
      paymentOptionsAnalysis.cancel();
    }
    rememberCardNamesFromEngineResult(result);
    const wasmCallMs = nowMs() - wasmStartedAt;
    workerTasks.phase(diagnosticTask, 'perf_collection');
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
      engineMemoryBytes: sampleDetailedPerf ? readEngineMemoryBytes() : null,
      registrySize: sampleDetailedPerf ? readRegistrySize() : null,
      snapshot: snapshotPerf || null,
      dispatch: dispatchPerf || null,
      replayExecution: replayExecutionPerf || null,
      advanceUntilDecision: advanceUntilDecisionPerf || null,
    };
    return {
      result: decorateResultWithPerf(result, perf),
      registryStatus,
    };
    }, name => workerTasks.phase(diagnosticTask, name));
  }, {}, diagnosticTask)
    .then(({ result, registryStatus }) => {
      workerTasks.phase(diagnosticTask, 'registry_publish');
      if (registryStatus) {
        postRegistryStatus(registryStatus);
        if (!registryStatus.done) scheduleBackgroundCompile(0);
      }
      if (result && typeof result === "object" && "decision" in result) {
        const identity = msg.runtimeBranch == null ? game.priorityAnalysisIdentity() : priorityIdentity;
        if (priorityIdentity !== identity) {
          priorityAnalysis.invalidate();
          priorityIdentity = identity;
          priorityViewRevision = priorityAnalysis.revision();
        }
        if (msg.runtimeBranch == null) result.__priority_revision = priorityViewRevision;
        workerTasks.phase(diagnosticTask, 'snapshot_encode');
        const snapshot = snapshotEncoder.encode(result, { full: method === "snapshot" });
        workerTasks.phase(diagnosticTask, 'response_post');
        postWorkerResult({ type: "result", id, ok: true, snapshot });
      } else {
        workerTasks.phase(diagnosticTask, 'response_post');
        postWorkerResult({ type: "result", id, ok: true, result });
      }
      workerTasks.finish(diagnosticTask);
    })
    .catch((err) => {
      workerTasks.phase(diagnosticTask, 'error_response_post');
      postWorkerResult({
        type: "result",
        id,
        ok: false,
        error: serializeError(err),
      });
      workerTasks.finish(diagnosticTask, 'error');
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
