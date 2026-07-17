const BASIC_LAND_NAMES = new Set([
  "Plains",
  "Island",
  "Swamp",
  "Mountain",
  "Forest",
]);
const BASIC_LAND_KEYS = new Set(
  [...BASIC_LAND_NAMES].map((name) => name.toLocaleLowerCase("en-US"))
);
const PREFERRED_BASIC_LAND_SET = "fdn";

const CUSTOM_CARD_ART_URLS_STORAGE_KEY = "ironsmith-custom-card-art-urls";
const CARD_PRINT_PREFERENCES_STORAGE_KEY = "ironsmith-card-print-preferences";
const HIDDEN_CARD_NAMES = new Set(["hidden card"]);
const HIDDEN_CARD_BACK_SVG = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 488 680" role="img" aria-label="Hidden card">
  <defs>
    <linearGradient id="bg" x1="0" x2="1" y1="0" y2="1">
      <stop offset="0" stop-color="#21150f"/>
      <stop offset="0.42" stop-color="#12151d"/>
      <stop offset="1" stop-color="#08090d"/>
    </linearGradient>
    <radialGradient id="core" cx="50%" cy="42%" r="58%">
      <stop offset="0" stop-color="#8e6be8" stop-opacity="0.85"/>
      <stop offset="0.46" stop-color="#7b4ed6" stop-opacity="0.32"/>
      <stop offset="1" stop-color="#04050a" stop-opacity="0"/>
    </radialGradient>
    <linearGradient id="rim" x1="0" x2="1" y1="0" y2="1">
      <stop offset="0" stop-color="#f2d492"/>
      <stop offset="0.52" stop-color="#7f5bca"/>
      <stop offset="1" stop-color="#e9b15f"/>
    </linearGradient>
  </defs>
  <rect x="8" y="8" width="472" height="664" rx="28" fill="#050507"/>
  <rect x="19" y="19" width="450" height="642" rx="22" fill="url(#bg)" stroke="url(#rim)" stroke-width="5"/>
  <rect x="37" y="37" width="414" height="606" rx="14" fill="none" stroke="#d7b66c" stroke-opacity="0.48" stroke-width="2"/>
  <circle cx="244" cy="290" r="210" fill="url(#core)"/>
  <g fill="none" stroke="#e7d09a" stroke-opacity="0.72" stroke-width="8">
    <path d="M244 128c54 72 112 117 178 136-66 19-124 64-178 136-54-72-112-117-178-136 66-19 124-64 178-136Z"/>
    <path d="M244 186c27 36 56 59 89 69-33 10-62 33-89 69-27-36-56-59-89-69 33-10 62-33 89-69Z"/>
  </g>
  <g fill="#f2dcaa" font-family="Alegreya Sans SC, Georgia, serif" text-anchor="middle">
    <text x="244" y="492" font-size="40" font-weight="800" letter-spacing="4">IRONSMITH</text>
    <text x="244" y="535" font-size="20" font-weight="700" letter-spacing="5" opacity="0.78">HIDDEN CARD</text>
  </g>
</svg>`.trim();

export const HIDDEN_CARD_BACK_IMAGE_URL =
  `data:image/svg+xml;charset=utf-8,${encodeURIComponent(HIDDEN_CARD_BACK_SVG)}`;

const resolvedCardImageUrlCache = new Map();
const cardJsonCache = new Map();
const localCardPayloadCache = new Map();
const imagePreloadCache = new Map();
let scryfallApiQueue = Promise.resolve();
let nextScryfallApiRequestAt = 0;
let scryfallApiBackoffUntil = 0;

const SCRYFALL_API_MIN_INTERVAL_MS = 140;
const SCRYFALL_API_DEFAULT_BACKOFF_MS = 60_000;

function storage() {
  try {
    return globalThis?.localStorage || null;
  } catch {
    return null;
  }
}

function customArtKey(cardName) {
  return String(cardName || "").trim().toLowerCase();
}

function normalizeSetCode(raw) {
  const value = String(raw || "").trim().toLowerCase();
  return /^[a-z0-9]{2,8}$/.test(value) ? value : "";
}

function normalizeCollectorNumber(raw) {
  return String(raw || "")
    .trim()
    .replace(/\*$/, "")
    .slice(0, 24);
}

function normalizePrintPreference(raw) {
  if (!raw || typeof raw !== "object") return null;
  const setCode = normalizeSetCode(raw.setCode || raw.set || raw.code);
  if (!setCode) return null;
  const collectorNumber = normalizeCollectorNumber(
    raw.collectorNumber || raw.collector_number || raw.number
  );
  return collectorNumber ? { setCode, collectorNumber } : { setCode };
}

function readJsonStorageMap(storageKey) {
  const localStorage = storage();
  if (!localStorage) return {};

  try {
    const raw = localStorage.getItem(storageKey);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : {};
  } catch {
    return {};
  }
}

function writeJsonStorageMap(storageKey, map) {
  const localStorage = storage();
  if (!localStorage) return;

  const entries = Object.entries(map || {})
    .filter(([, value]) => {
      if (typeof value === "string") return value.trim();
      return value && typeof value === "object" && !Array.isArray(value);
    })
    .sort(([left], [right]) => left.localeCompare(right));
  if (entries.length === 0) {
    localStorage.removeItem(storageKey);
    return;
  }

  localStorage.setItem(storageKey, JSON.stringify(Object.fromEntries(entries)));
}

function readCustomCardArtUrlMap() {
  return readJsonStorageMap(CUSTOM_CARD_ART_URLS_STORAGE_KEY);
}

function writeCustomCardArtUrlMap(map) {
  writeJsonStorageMap(CUSTOM_CARD_ART_URLS_STORAGE_KEY, map);
}

function readCardPrintPreferenceMap() {
  const rawMap = readJsonStorageMap(CARD_PRINT_PREFERENCES_STORAGE_KEY);
  const out = {};
  for (const [key, raw] of Object.entries(rawMap)) {
    const preference = normalizePrintPreference(raw);
    if (preference) out[key] = preference;
  }
  return out;
}

function writeCardPrintPreferenceMap(map) {
  writeJsonStorageMap(CARD_PRINT_PREFERENCES_STORAGE_KEY, map);
}

function printPreferenceCacheKey(printPreference) {
  const preference = normalizePrintPreference(printPreference);
  if (!preference) return "default";
  return [
    `set:${preference.setCode}`,
    preference.collectorNumber ? `number:${preference.collectorNumber.toLowerCase()}` : "",
  ].filter(Boolean).join("|");
}

function clearCachedCardImageUrls(cardName) {
  const keyPrefix = `${customArtKey(cardName)}|`;
  for (const key of resolvedCardImageUrlCache.keys()) {
    if (key.startsWith(keyPrefix)) {
      resolvedCardImageUrlCache.delete(key);
    }
  }
}

export function customCardArtUrl(cardName) {
  const key = customArtKey(cardName);
  if (!key) return "";
  const url = readCustomCardArtUrlMap()[key];
  return typeof url === "string" ? url.trim() : "";
}

export function preferredCardPrint(cardName) {
  const key = customArtKey(cardName);
  if (!key) return null;
  return readCardPrintPreferenceMap()[key] || null;
}

export function isHiddenCardName(cardName) {
  return HIDDEN_CARD_NAMES.has(customArtKey(cardName));
}

function cardImageCacheKey(cardName, version = "normal", printPreference = null) {
  return [
    customArtKey(cardName),
    String(version || "normal").trim() || "normal",
    printPreferenceCacheKey(printPreference),
  ].join("|");
}

function cardJsonCacheKey(cardName, printPreference = null) {
  return `${customArtKey(cardName)}|${printPreferenceCacheKey(printPreference)}`;
}

function isBasicLandName(cardName) {
  return BASIC_LAND_KEYS.has(String(cardName || "").trim().toLocaleLowerCase("en-US"));
}

function prefersLiveScryfallCard(cardName) {
  return isBasicLandName(cardName);
}

function baseAssetUrl() {
  const configured = typeof import.meta !== "undefined"
    ? import.meta.env?.BASE_URL
    : null;
  const base = configured || "/";
  return new URL(base, globalThis?.location?.href || "http://localhost/").href;
}

const STABLE_CARD_ASSET_FETCH_OPTIONS = { cache: "no-cache" };

export function cardRouteKey(name) {
  const normalized = String(name || "")
    .trim()
    .toLocaleLowerCase("en-US")
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9_]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return normalized || "";
}

function namedCardParams(cardName, printPreference = null) {
  const query = String(cardName || "").trim();
  const params = new URLSearchParams();
  const setCode = normalizePrintPreference(printPreference)?.setCode || "";

  if (isBasicLandName(query)) {
    params.set("exact", query);
    params.set("set", setCode || PREFERRED_BASIC_LAND_SET);
  } else {
    params.set("fuzzy", query);
    if (setCode) params.set("set", setCode);
  }

  return params;
}

function namedCardJsonUrls(cardName, printPreference = null) {
  const query = String(cardName || "").trim();
  const preference = normalizePrintPreference(printPreference);
  const urls = [];
  if (preference?.setCode && preference?.collectorNumber) {
    urls.push(
      `https://api.scryfall.com/cards/${encodeURIComponent(preference.setCode)}/${encodeURIComponent(preference.collectorNumber)}`
    );
  }
  if (isBasicLandName(query)) {
    urls.push(`https://api.scryfall.com/cards/named?${namedCardParams(query, preference).toString()}`);
    return urls;
  }

  const exactParams = new URLSearchParams({ exact: query });
  const fuzzyParams = new URLSearchParams({ fuzzy: query });
  if (preference?.setCode) {
    exactParams.set("set", preference.setCode);
    fuzzyParams.set("set", preference.setCode);
  }
  urls.push(
    `https://api.scryfall.com/cards/named?${exactParams.toString()}`,
    `https://api.scryfall.com/cards/named?${fuzzyParams.toString()}`
  );
  return urls;
}

function nonFullArtSearchUrl(cardName, printPreference = null) {
  const query = String(cardName || "").trim();
  if (!query) return "";
  const preference = normalizePrintPreference(printPreference);
  const pieces = [`!"${query.replace(/"/g, '\\"')}"`, "-is:fullart"];
  if (preference?.setCode) pieces.push(`set:${preference.setCode}`);
  const params = new URLSearchParams({
    q: pieces.join(" "),
    unique: "cards",
    order: "released",
    dir: "desc",
  });
  return `https://api.scryfall.com/cards/search?${params.toString()}`;
}

function scryfallCardMatchesName(card, cardName) {
  const queryKey = customArtKey(cardName);
  if (!queryKey || !card || typeof card !== "object") return false;
  if (customArtKey(card.name) === queryKey) return true;
  return Array.isArray(card.card_faces)
    && card.card_faces.some((face) => customArtKey(face?.name) === queryKey);
}

function imageUrlFromScryfallCard(card, version = "normal") {
  return imageUrlFromImageUris(
    card?.image_uris
      || (Array.isArray(card?.card_faces)
        ? card.card_faces.find((face) => face?.image_uris)?.image_uris
        : null),
    version
  );
}

function imageUrlFromImageUris(imageUris, version = "normal") {
  const wanted = String(version || "normal").trim() || "normal";
  if (!imageUris) return "";
  return String(
    imageUris[wanted]
    || imageUris.normal
    || imageUris.large
    || imageUris.art_crop
    || imageUris.small
    || ""
  );
}

function cacheResolvedImageUrls(cardName, card, printPreference = null) {
  const imageUris = card?.image_uris
    || (Array.isArray(card?.card_faces)
      ? card.card_faces.find((face) => face?.image_uris)?.image_uris
      : null);
  if (!imageUris) return;
  for (const [version, url] of Object.entries(imageUris)) {
    if (!url) continue;
    resolvedCardImageUrlCache.set(cardImageCacheKey(cardName, version, printPreference), String(url));
  }
}

function cacheImageUris(cardName, imageUris) {
  if (!imageUris || typeof imageUris !== "object") return;
  for (const [version, url] of Object.entries(imageUris)) {
    if (!url) continue;
    resolvedCardImageUrlCache.set(cardImageCacheKey(cardName, version), String(url));
  }
}

function localScryfallPayloadForName(payload, cardName) {
  const scryfall = payload?.scryfall || null;
  if (!scryfall || typeof scryfall !== "object") return null;
  const queryKey = customArtKey(cardName);
  const faces = Array.isArray(scryfall.faces) ? scryfall.faces : [];
  const exactFace = faces.find((face) => customArtKey(face?.name) === queryKey);
  return exactFace || scryfall;
}

function cacheLocalScryfallPayload(cardName, payload) {
  const scryfall = localScryfallPayloadForName(payload, cardName);
  cacheImageUris(cardName, scryfall?.image_uris);
}

async function fetchLocalCardPayload(cardName) {
  const query = String(cardName || "").trim();
  const route = cardRouteKey(query);
  if (!route || isHiddenCardName(query)) return null;
  if (localCardPayloadCache.has(route)) return localCardPayloadCache.get(route);

  const request = (async () => {
    const url = new URL(`cards/${route}.json`, baseAssetUrl()).href;
    const response = await fetch(url, STABLE_CARD_ASSET_FETCH_OPTIONS);
    if (response.status === 404) return null;
    if (!response.ok) throw new Error(`Local card metadata fetch failed: HTTP ${response.status}`);
    const payload = await response.json();
    cacheLocalScryfallPayload(query, payload);
    return payload && typeof payload === "object" ? payload : null;
  })()
    .catch((error) => {
      localCardPayloadCache.delete(route);
      throw error;
    });

  localCardPayloadCache.set(route, request);
  return request;
}

function parseRetryAfterMs(response) {
  const raw = response?.headers?.get?.("retry-after");
  const seconds = Number(raw);
  if (Number.isFinite(seconds) && seconds > 0) {
    return Math.min(Math.ceil(seconds * 1000), 10 * 60_000);
  }
  const timestamp = Date.parse(String(raw || ""));
  if (Number.isFinite(timestamp)) {
    return Math.max(0, Math.min(timestamp - Date.now(), 10 * 60_000));
  }
  return SCRYFALL_API_DEFAULT_BACKOFF_MS;
}

function wait(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function fetchScryfallApiJson(url) {
  const run = async () => {
    const now = Date.now();
    if (now < scryfallApiBackoffUntil) {
      throw new Error("Scryfall API temporarily backed off after rate limiting");
    }

    const delay = Math.max(0, nextScryfallApiRequestAt - now);
    if (delay > 0) await wait(delay);
    nextScryfallApiRequestAt = Date.now() + SCRYFALL_API_MIN_INTERVAL_MS;

    const response = await fetch(url, {
      headers: {
        Accept: "application/json;q=0.9,*/*;q=0.8",
      },
    });
    if (response.status === 429) {
      scryfallApiBackoffUntil = Date.now() + parseRetryAfterMs(response);
    }
    return response;
  };

  const request = scryfallApiQueue.then(run, run);
  scryfallApiQueue = request.catch(() => null);
  return request;
}

async function fetchScryfallCardJson(cardName, printPreference = null) {
  const query = String(cardName || "").trim();
  const preference = normalizePrintPreference(printPreference);
  const key = cardJsonCacheKey(query, preference);
  if (!query || isHiddenCardName(query)) return null;
  if (cardJsonCache.has(key)) return cardJsonCache.get(key);

  const request = (async () => {
    if (!preference?.collectorNumber) {
      const searchUrl = nonFullArtSearchUrl(query, preference);
      if (searchUrl) {
        const response = await fetchScryfallApiJson(searchUrl);
        if (response.ok) {
          const payload = await response.json();
          const card = (Array.isArray(payload?.data) ? payload.data : [])
            .find((candidate) => scryfallCardMatchesName(candidate, query));
          if (card) {
            cacheResolvedImageUrls(query, card, preference);
            return card;
          }
        }
      }
    }

    for (const url of namedCardJsonUrls(query, preference)) {
      const response = await fetchScryfallApiJson(url);
      if (!response.ok) continue;
      const card = await response.json();
      if (preference?.collectorNumber && !scryfallCardMatchesName(card, query)) {
        continue;
      }
      if (!preference?.collectorNumber && card?.full_art === true) {
        continue;
      }
      cacheResolvedImageUrls(query, card, preference);
      return card;
    }
    throw new Error(`Could not resolve Scryfall card for ${query}`);
  })()
    .catch((error) => {
      cardJsonCache.delete(key);
      throw error;
    });

  cardJsonCache.set(key, request);
  return request;
}

export function setCustomCardArtUrls(entries) {
  const map = readCustomCardArtUrlMap();
  for (const entry of entries || []) {
    const key = customArtKey(entry?.name);
    if (!key) continue;

    const artUrl = String(entry?.artUrl || "").trim();
    if (artUrl) {
      map[key] = artUrl;
    } else {
      delete map[key];
    }
  }
  writeCustomCardArtUrlMap(map);
}

export function setPreferredCardPrints(entries) {
  const map = readCardPrintPreferenceMap();
  let changed = false;

  for (const entry of entries || []) {
    const key = customArtKey(entry?.name);
    if (!key) continue;

    const preference = normalizePrintPreference(entry);
    const previous = map[key] || null;
    if (preference) {
      if (
        previous?.setCode !== preference.setCode
        || previous?.collectorNumber !== preference.collectorNumber
      ) {
        map[key] = preference;
        changed = true;
        clearCachedCardImageUrls(entry.name);
      }
    } else if (previous) {
      delete map[key];
      changed = true;
      clearCachedCardImageUrls(entry.name);
    }
  }

  if (changed) writeCardPrintPreferenceMap(map);
}

export function scryfallImageUrl(cardName, version = "normal") {
  const query = String(cardName || "").trim();
  if (!query) return "";
  if (isHiddenCardName(query)) return HIDDEN_CARD_BACK_IMAGE_URL;
  const customUrl = customCardArtUrl(query);
  if (customUrl) return customUrl;
  const cached = resolvedCardImageUrlCache.get(cardImageCacheKey(query, version, preferredCardPrint(query)));
  if (cached) return cached;
  return "";
}

export async function resolveScryfallImageUrl(cardName, version = "normal") {
  const query = String(cardName || "").trim();
  if (!query) return "";
  if (isHiddenCardName(query)) return HIDDEN_CARD_BACK_IMAGE_URL;
  const customUrl = customCardArtUrl(query);
  if (customUrl) return customUrl;

  const preference = preferredCardPrint(query);
  const preferredKey = cardImageCacheKey(query, version, preference);
  const cached = resolvedCardImageUrlCache.get(preferredKey);
  if (cached) return cached;

  if (preference) {
    const card = await fetchScryfallCardJson(query, preference).catch(() => null);
    const resolved = imageUrlFromScryfallCard(card, version);
    if (resolved) {
      resolvedCardImageUrlCache.set(preferredKey, resolved);
      return resolved;
    }
  }

  const defaultKey = cardImageCacheKey(query, version);
  const defaultCached = resolvedCardImageUrlCache.get(defaultKey);
  if (defaultCached) return defaultCached;

  if (prefersLiveScryfallCard(query)) {
    const card = await fetchScryfallCardJson(query).catch(() => null);
    const resolved = imageUrlFromScryfallCard(card, version);
    if (resolved) {
      resolvedCardImageUrlCache.set(defaultKey, resolved);
      return resolved;
    }
  }

  const localPayload = await fetchLocalCardPayload(query).catch(() => null);
  const localScryfall = localScryfallPayloadForName(localPayload, query);
  const localResolved = localScryfall?.full_art === true
    ? ""
    : imageUrlFromImageUris(localScryfall?.image_uris, version);
  if (localResolved) {
    resolvedCardImageUrlCache.set(defaultKey, localResolved);
    return localResolved;
  }

  const card = await fetchScryfallCardJson(query);
  const resolved = imageUrlFromScryfallCard(card, version);
  if (resolved) {
    resolvedCardImageUrlCache.set(defaultKey, resolved);
    return resolved;
  }
  return "";
}

export async function preloadScryfallImage(cardName, version = "normal") {
  const url = await resolveScryfallImageUrl(cardName, version);
  if (!url || url === HIDDEN_CARD_BACK_IMAGE_URL) return url;
  if (imagePreloadCache.has(url)) return imagePreloadCache.get(url);

  const request = new Promise((resolve) => {
    if (typeof Image === "undefined") {
      resolve(url);
      return;
    }
    const image = new Image();
    image.decoding = "async";
    image.referrerPolicy = "no-referrer";
    image.onload = () => resolve(url);
    image.onerror = () => resolve(url);
    image.src = url;
  });

  imagePreloadCache.set(url, request);
  return request;
}

export async function preloadCardArt(cardNames, options = {}) {
  const versions = Array.isArray(options.versions) && options.versions.length > 0
    ? options.versions
    : ["normal"];
  const concurrency = Math.max(1, Math.min(Number(options.concurrency) || 4, 8));
  const names = [...new Set((cardNames || [])
    .map((name) => String(name || "").trim())
    .filter((name) => name && !isHiddenCardName(name)))];
  const jobs = [];
  for (const name of names) {
    for (const version of versions) {
      jobs.push({ name, version });
    }
  }

  let nextIndex = 0;
  const workers = Array.from({ length: Math.min(concurrency, jobs.length) }, async () => {
    while (nextIndex < jobs.length) {
      const job = jobs[nextIndex];
      nextIndex += 1;
      await preloadScryfallImage(job.name, job.version).catch(() => null);
    }
  });
  await Promise.all(workers);
  return { requested: jobs.length, cards: names.length };
}

const namedCardMetaCache = new Map();
const localizedCardTranslationCache = new Map();

export async function fetchScryfallCardMeta(cardName) {
  const query = String(cardName || "").trim();
  if (!query) {
    return { mana_cost: null, oracle_text: "", produced_mana: [] };
  }
  if (isHiddenCardName(query)) {
    return { mana_cost: null, oracle_text: "", produced_mana: [] };
  }

  if (namedCardMetaCache.has(query)) {
    return namedCardMetaCache.get(query);
  }

  const request = (async () => {
    const localPayload = await fetchLocalCardPayload(query).catch(() => null);
    const localScryfall = localScryfallPayloadForName(localPayload, query);
    if (localScryfall) {
      return {
        mana_cost: localScryfall?.mana_cost || null,
        oracle_text: localScryfall?.oracle_text || "",
        produced_mana: Array.isArray(localScryfall?.produced_mana) ? localScryfall.produced_mana : [],
      };
    }

    const fuzzyCard = await fetchScryfallCardJson(query);
    return {
      mana_cost: fuzzyCard?.mana_cost || null,
      oracle_text: fuzzyCard?.oracle_text || "",
      produced_mana: Array.isArray(fuzzyCard?.produced_mana) ? fuzzyCard.produced_mana : [],
    };
  })()
    .catch((error) => {
      namedCardMetaCache.delete(query);
      throw error;
    });

  namedCardMetaCache.set(query, request);
  return request;
}

function firstFaceValue(card, field) {
  if (card?.[field]) return card[field];
  if (Array.isArray(card?.card_faces)) {
    return card.card_faces.map((face) => face?.[field]).filter(Boolean).join("\n//\n");
  }
  return "";
}

function parenGroupCount(text) {
  return (String(text || "").match(/\(/g) || []).length;
}

function wordTokens(text) {
  return new Set(String(text || "").toLowerCase().match(/\p{L}{3,}/gu) || []);
}

// True when most of the text's words appear in the English oracle text —
// i.e. it is English data mislabeled with a target language, not a translation.
function looksUntranslated(englishTokens, text) {
  const tokens = wordTokens(text);
  if (tokens.size === 0) return false;
  let shared = 0;
  for (const token of tokens) {
    if (englishTokens.has(token)) shared += 1;
  }
  return shared / tokens.size >= 0.6;
}

function localizedCardPayload(card, locale) {
  if (!card || typeof card !== "object") return null;
  // Only the printed_* fields are localized; name/type_line/oracle_text are
  // always English on Scryfall card objects. Never backfill with those —
  // consumers fall back to their own English text when a field is empty.
  const name = firstFaceValue(card, "printed_name");
  const typeLine = firstFaceValue(card, "printed_type_line");
  const oracleText = firstFaceValue(card, "printed_text");
  if (!name && !typeLine && !oracleText) return null;
  return {
    schemaVersion: 1,
    source: "scryfall-live",
    sourceLang: "en",
    targetLang: locale,
    oracleId: card.oracle_id || null,
    englishName: card.name || "",
    route: cardRouteKey(card.name || ""),
    name,
    typeLine,
    oracleText,
    scryfallId: card.id || null,
    set: card.set || null,
    collectorNumber: card.collector_number || null,
  };
}

export async function fetchScryfallLocalizedCardTranslation(cardName, locale) {
  const query = String(cardName || "").trim();
  const targetLang = String(locale || "").trim().toLowerCase();
  if (!query || !targetLang || targetLang === "en" || isHiddenCardName(query)) return null;

  const cacheKey = `${targetLang}:${cardJsonCacheKey(query)}`;
  if (localizedCardTranslationCache.has(cacheKey)) {
    return localizedCardTranslationCache.get(cacheKey);
  }

  const request = (async () => {
    const englishCard = await fetchScryfallCardJson(query).catch(() => null);
    const oracleId = String(englishCard?.oracle_id || "").trim();
    if (!oracleId) return null;

    const params = new URLSearchParams({
      q: `lang:${targetLang} oracleid:${oracleId}`,
      unique: "prints",
      order: "released",
      dir: "desc",
    });
    const response = await fetchScryfallApiJson(`https://api.scryfall.com/cards/search?${params.toString()}`);
    if (!response.ok) return null;
    const payload = await response.json();
    // Scryfall marks not-yet-localized printings as lang:<locale> with a
    // localized printed_name but printed_text still in English — often with
    // wording that drifted from the current oracle text, so compare by word
    // overlap rather than equality. Treat such text as absent so a properly
    // translated older printing wins instead.
    const englishTokens = wordTokens(firstFaceValue(englishCard, "oracle_text"));
    const candidates = (payload?.data || [])
      .map((card) => localizedCardPayload(card, targetLang))
      .map((card) => (
        card && card.oracleText && looksUntranslated(englishTokens, card.oracleText)
          ? { ...card, oracleText: "" }
          : card
      ))
      .filter((card) => card && (card.oracleText || card.name || card.typeLine));
    // Reminder text only exists on printings that physically carried it. Prefer
    // the newest printing whose printed_text keeps at least as many parenthetical
    // (reminder) groups as the current English oracle text, then the newest with
    // any printed_text, then the newest with just a localized name/type line.
    const englishParens = parenGroupCount(firstFaceValue(englishCard, "oracle_text"));
    const translated = candidates.find((card) => card.oracleText && parenGroupCount(card.oracleText) >= englishParens)
      || candidates.find((card) => card.oracleText)
      || candidates[0];
    return translated || null;
  })()
    .catch((error) => {
      localizedCardTranslationCache.delete(cacheKey);
      throw error;
    });

  localizedCardTranslationCache.set(cacheKey, request);
  return request;
}
