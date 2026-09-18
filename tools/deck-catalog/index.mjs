import { slugify, text } from "./catalog-utils.mjs";

const STOP_WORDS = new Set(["the", "and", "with", "for", "from", "of", "de", "del"]);

export function tokenize(value) {
  return slugify(value)
    .split("-")
    .map(text)
    .filter((token) => token.length >= 2 && !STOP_WORDS.has(token));
}

export function buildSearchIndex(entries, { generatedAt = "" } = {}) {
  const tokens = new Map();
  for (const entry of Array.isArray(entries) ? entries : []) {
    if (!entry?.id) continue;
    const values = [
      entry.id,
      entry.format,
      entry.name,
      entry.archetype,
      ...(Array.isArray(entry.colors) ? entry.colors : []),
      ...(Array.isArray(entry.mechanics) ? entry.mechanics : []),
      entry.event,
      entry.source,
      ...(Array.isArray(entry.tags) ? entry.tags : []),
      ...(Array.isArray(entry.cards) ? entry.cards : []),
    ];
    for (const token of new Set(values.flatMap(tokenize))) {
      const ids = tokens.get(token) || new Set();
      ids.add(entry.id);
      tokens.set(token, ids);
    }
  }

  return {
    schemaVersion: 1,
    format: text(entries?.[0]?.format).toLowerCase() || "modern",
    generatedAt,
    tokens: Object.fromEntries(
      [...tokens.entries()]
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([token, ids]) => [token, [...ids].sort()]),
    ),
  };
}

export function searchIndex(index, query, { limit = 50 } = {}) {
  const queryTokens = [...new Set(tokenize(query))];
  if (!queryTokens.length) return [];
  const matches = queryTokens.map((token) => new Set(index?.tokens?.[token] || []));
  return [...matches.slice(1).reduce(
    (current, next) => new Set([...current].filter((id) => next.has(id))),
    matches[0],
  )].slice(0, Math.max(0, Number(limit) || 50));
}
