import { cardRouteKey } from "./scryfall.js";

function normalize(value) {
  return String(value || "")
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLocaleLowerCase("en-US")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

const catalogIndexCache = new Map();
const catalogIndexRequests = new Map();
const catalogDetailCache = new Map();
const catalogDetailRequests = new Map();
const cardArtCache = new Map();
const cardArtRequests = new Map();

function assetUrl(path) {
  const configured = typeof import.meta !== "undefined" ? import.meta.env?.BASE_URL : null;
  return new URL(path, new URL(configured || "/", globalThis?.location?.href || "http://localhost/")).href;
}

function searchTokens(query) {
  return catalogQueryTokens(query);
}

export function catalogQueryTokens(query) {
  return [...new Set(normalize(query).split(/\s+/).filter((token) => token.length >= 2))];
}

export function catalogSearchScore(entry, query) {
  const normalizedQuery = normalize(query);
  if (!normalizedQuery) return 0;
  const tokens = catalogQueryTokens(query);
  const name = normalize(entry?.name);
  const archetype = normalize(entry?.archetype);
  const event = normalize(entry?.event);
  const cards = normalize((entry?.cardNames || entry?.cards || []).join(" "));
  let score = 0;
  if (name === normalizedQuery) score += 1000;
  if (archetype === normalizedQuery) score += 800;
  if (name.startsWith(normalizedQuery)) score += 500;
  if (archetype.startsWith(normalizedQuery)) score += 400;
  for (const token of tokens) {
    if (name.includes(token)) score += 80;
    if (archetype.includes(token)) score += 60;
    if (cards.includes(token)) score += 35;
    if (event.includes(token)) score += 15;
  }
  return score;
}

export function searchCatalogEntries(entries, query, { limit = 30, searchIndex } = {}) {
  const tokens = searchTokens(query);
  let candidates = Array.isArray(entries) ? entries : [];
  const hasPostingsForEveryToken = tokens.length > 0
    && searchIndex?.tokens
    && tokens.every((token) => Object.hasOwn(searchIndex.tokens, token));
  if (hasPostingsForEveryToken) {
    const ids = tokens.map((token) => new Set(searchIndex.tokens[token] || []));
    const matchingIds = ids.slice(1).reduce(
      (current, next) => new Set([...current].filter((id) => next.has(id))),
      ids[0],
    );
    candidates = candidates.filter((entry) => matchingIds.has(entry?.id));
  }
  const results = candidates.filter((entry) => {
    const haystack = normalize([
      entry?.name,
      entry?.archetype,
      entry?.event,
      entry?.format,
      ...(entry?.colors || []),
      ...(entry?.mechanics || []),
      ...(entry?.tags || []),
      ...(entry?.cardNames || []),
    ].join(" "));
    return tokens.every((token) => haystack.includes(token));
  });
  return results
    .sort((left, right) => {
      const scoreOrder = catalogSearchScore(right, query) - catalogSearchScore(left, query);
      if (scoreOrder) return scoreOrder;
      const dateOrder = String(right.date || "").localeCompare(String(left.date || ""));
      return dateOrder || (Number(left.placement || 9999) - Number(right.placement || 9999));
    })
    .slice(0, Math.max(0, Number(limit) || 30));
}

export async function loadCatalogIndex({ format = "modern", fetchImpl = globalThis.fetch } = {}) {
  const cacheable = fetchImpl === globalThis.fetch;
  if (cacheable && catalogIndexCache.has(format)) return catalogIndexCache.get(format);
  if (cacheable && catalogIndexRequests.has(format)) return catalogIndexRequests.get(format);

  const request = Promise.all([
    fetchImpl(`/catalog/${format}/index.json`, { cache: "no-store" }),
    fetchImpl(`/catalog/${format}/search-index.json`, { cache: "no-store" }),
  ]).then(async ([response, searchResponse]) => {
    if (!response?.ok) throw new Error(`Catalog index request failed (${response?.status || "unknown"})`);
    const catalog = await response.json();
    const searchIndex = searchResponse?.ok ? await searchResponse.json() : null;
    return { ...catalog, searchIndex };
  });
  if (!cacheable) return request;
  catalogIndexRequests.set(format, request);
  try {
    const catalog = await request;
    catalogIndexCache.set(format, catalog);
    return catalog;
  } finally {
    catalogIndexRequests.delete(format);
  }
}

export async function loadCatalogDeckDetail(entry, { format = "modern", fetchImpl = globalThis.fetch } = {}) {
  if (!entry?.detail) throw new Error("Catalog entry has no detail path");
  const cacheKey = `${format}/${entry.detail}`;
  const cacheable = fetchImpl === globalThis.fetch;
  if (cacheable && catalogDetailCache.has(cacheKey)) return catalogDetailCache.get(cacheKey);
  if (cacheable && catalogDetailRequests.has(cacheKey)) return catalogDetailRequests.get(cacheKey);
  const request = fetchImpl(`/catalog/${format}/${entry.detail}`, { cache: "force-cache" })
    .then((response) => {
      if (!response?.ok) throw new Error(`Catalog deck request failed (${response?.status || "unknown"})`);
      return response.json();
    });
  if (!cacheable) return request;
  catalogDetailRequests.set(cacheKey, request);
  try {
    const detail = await request;
    catalogDetailCache.set(cacheKey, detail);
    return detail;
  } finally {
    catalogDetailRequests.delete(cacheKey);
  }
}

export async function loadLocalCardArt(cardName, { fetchImpl = globalThis.fetch } = {}) {
  const route = cardRouteKey(cardName);
  if (!route) return "";
  if (cardArtCache.has(route)) return cardArtCache.get(route);
  if (cardArtRequests.has(route)) return cardArtRequests.get(route);
  const request = fetchImpl(assetUrl(`cards/${route}.json`), { cache: "force-cache" })
    .then(async (response) => {
      if (!response?.ok) return "";
      const payload = await response.json();
      return payload?.scryfall?.image_uris?.art_crop
        || payload?.scryfall?.image_uris?.normal
        || "";
    })
    .catch(() => "");
  cardArtRequests.set(route, request);
  try {
    const imageUrl = await request;
    if (imageUrl) cardArtCache.set(route, imageUrl);
    return imageUrl;
  } finally {
    cardArtRequests.delete(route);
  }
}
