function normalize(value) {
  return String(value || "")
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLocaleLowerCase("en-US")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

export function catalogQueryTokens(query) {
  return [...new Set(normalize(query).split(/\s+/).filter((token) => token.length >= 2))];
}

export function searchCatalogEntries(entries, query, { limit = 30 } = {}) {
  const tokens = catalogQueryTokens(query);
  const results = (Array.isArray(entries) ? entries : []).filter((entry) => {
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
  const response = await fetchImpl(`/catalog/${format}/index.json`, { cache: "no-store" });
  if (!response?.ok) throw new Error(`Catalog index request failed (${response?.status || "unknown"})`);
  return response.json();
}

export async function loadCatalogDeckDetail(entry, { format = "modern", fetchImpl = globalThis.fetch } = {}) {
  if (!entry?.detail) throw new Error("Catalog entry has no detail path");
  const response = await fetchImpl(`/catalog/${format}/${entry.detail}`, { cache: "no-store" });
  if (!response?.ok) throw new Error(`Catalog deck request failed (${response?.status || "unknown"})`);
  return response.json();
}
