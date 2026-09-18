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

function searchTokens(query) {
  return catalogQueryTokens(query);
}

export function catalogQueryTokens(query) {
  return [...new Set(normalize(query).split(/\s+/).filter((token) => token.length >= 2))];
}

export function searchCatalogEntries(entries, query, { limit = 30, searchIndex } = {}) {
  const tokens = searchTokens(query);
  let candidates = Array.isArray(entries) ? entries : [];
  if (tokens.length && searchIndex?.tokens) {
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
  const response = await fetchImpl(`/catalog/${format}/${entry.detail}`, { cache: "no-store" });
  if (!response?.ok) throw new Error(`Catalog deck request failed (${response?.status || "unknown"})`);
  return response.json();
}
