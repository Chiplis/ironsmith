import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { text } from "./catalog-utils.mjs";

export const MANA_COLORS = ["W", "U", "B", "R", "G", "C"];

const COLOR_SET = new Set(MANA_COLORS);

export function cardMetadataKey(name) {
  return text(name).normalize("NFKD").replace(/[\u0300-\u036f]/g, "").toLocaleLowerCase("en-US");
}

function cardRouteKey(name) {
  return text(name)
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLocaleLowerCase("en-US")
    .replace(/[^a-z0-9_]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function normalizeCodes(values) {
  return [...new Set((Array.isArray(values) ? values : [])
    .map((value) => text(value).toUpperCase())
    .filter((value) => COLOR_SET.has(value)))].sort();
}

function normalizeMetadata(name, payload) {
  if (!payload || typeof payload !== "object") {
    return { name, status: "not_found" };
  }
  return {
    name: text(payload.name) || name,
    status: "ok",
    source: "scryfall",
    colors: normalizeCodes(payload.colors),
    colorIdentity: normalizeCodes(payload.color_identity),
    producedMana: normalizeCodes(payload.produced_mana),
    manaCost: text(payload.mana_cost),
    typeLine: text(payload.type_line),
  };
}

export async function fetchCardMetadata(name, {
  fetchImpl = globalThis.fetch,
  signal,
} = {}) {
  if (typeof fetchImpl !== "function") throw new Error("A fetch implementation is required");
  const query = encodeURIComponent(text(name));
  const response = await fetchImpl(`https://api.scryfall.com/cards/named?exact=${query}`, {
    signal,
    headers: { "user-agent": "IronSmith deck catalog (github actions; contact repository owner)" },
  });
  if (response?.status === 404) return normalizeMetadata(name, null);
  if (response?.status === 429) {
    const error = new Error("Scryfall rate limit reached");
    error.retryAfterMs = Math.max(1000, Number(response.headers?.get?.("retry-after") || 5) * 1000);
    throw error;
  }
  if (!response?.ok) throw new Error(`Scryfall card request failed (${response?.status || "unknown"})`);
  return normalizeMetadata(name, await response.json());
}

const PIP_COLORS = { White: "W", Blue: "U", Black: "B", Red: "R", Green: "G", Colorless: "C" };

export async function readLocalCardMetadata(name, { root } = {}) {
  if (!root) return null;
  const route = cardRouteKey(name);
  if (!route) return null;
  try {
    const payload = JSON.parse(await readFile(join(root, `${route}.json`), "utf8"));
    const card = payload?.artifacts?.[0]?.payload?.definition?.card;
    if (!card) return null;
    const pipColors = (card.mana_cost?.pips || [])
      .flat(Infinity)
      .map((pip) => (typeof pip === "string" ? PIP_COLORS[pip] : ""))
      .filter(Boolean);
    const producedMana = normalizeCodes(payload?.scryfall?.produced_mana);
    return {
      name: text(payload.canonicalName) || name,
      status: "ok",
      source: "local-card-artifact",
      colors: normalizeCodes(pipColors),
      colorIdentity: normalizeCodes(pipColors),
      producedMana,
      manaCost: text(payload?.scryfall?.mana_cost),
      typeLine: (card.card_types || []).join(" — "),
    };
  } catch {
    return null;
  }
}

export async function resolveCardMetadata(names, {
  cache = {},
  fetchImpl = globalThis.fetch,
  minDelayMs = 250,
  localRoot = "",
  allowNetwork = true,
  sleep = (duration) => new Promise((resolve) => setTimeout(resolve, duration)),
  signal,
  onResolve = () => {},
} = {}) {
  const result = { ...cache };
  let lastRequestAt = 0;
  const uniqueNames = [...new Set((Array.isArray(names) ? names : []).map(text).filter(Boolean))]
    .sort((left, right) => left.localeCompare(right, "en"));

  for (const name of uniqueNames) {
    const key = cardMetadataKey(name);
    const local = await readLocalCardMetadata(name, { root: localRoot });
    if (local) {
      result[key] = local;
      onResolve(local);
      continue;
    }
    if (result[key] && result[key].status !== "error") continue;
    if (!allowNetwork) {
      result[key] = { name, status: "unresolved" };
      onResolve(result[key]);
      continue;
    }
    let resolved = null;
    for (let attempt = 0; attempt < 3 && !resolved; attempt += 1) {
      const remaining = Math.max(0, minDelayMs - (Date.now() - lastRequestAt));
      if (remaining) await sleep(remaining);
      try {
        resolved = await fetchCardMetadata(name, { fetchImpl, signal });
      } catch (error) {
        if (error.retryAfterMs && attempt < 2) {
          await sleep(error.retryAfterMs);
          continue;
        }
        resolved = { name, status: "error", error: error.message || "request failed" };
      }
      lastRequestAt = Date.now();
    }
    result[key] = resolved || { name, status: "error", error: "request failed" };
    onResolve(result[key]);
  }
  return result;
}

function addCount(map, key, count) {
  if (!COLOR_SET.has(key)) return;
  map[key] = (map[key] || 0) + count;
}

function manaPips(manaCost) {
  return [...String(manaCost || "").matchAll(/\{([^}]+)\}/g)]
    .flatMap((match) => match[1].toUpperCase().split("/"))
    .filter((symbol) => COLOR_SET.has(symbol));
}

export function buildManaProfile(deck, metadata = {}) {
  const mainboard = Array.isArray(deck?.mainboard) ? deck.mainboard : [];
  const colors = new Set();
  const pipCounts = {};
  const sourceCounts = {};
  const landCounts = new Map();
  const unresolvedCardNames = [];
  const metadataSources = new Set();
  let resolvedCardCount = 0;
  let landCount = 0;
  let totalCardCount = 0;

  for (const card of mainboard) {
    const name = text(card?.name);
    const count = Number(card?.count) || 0;
    if (!name || count < 1) continue;
    totalCardCount += 1;
    const cardData = metadata[cardMetadataKey(name)];
    if (!cardData || cardData.status !== "ok") {
      unresolvedCardNames.push(name);
      continue;
    }
    resolvedCardCount += 1;
    metadataSources.add(cardData.source || "scryfall");
    const isLand = /\bland\b/i.test(cardData.typeLine);
    if (!isLand) {
      for (const color of [...(cardData.colors || []), ...(cardData.colorIdentity || [])]) colors.add(color);
    }
    for (const pip of manaPips(cardData.manaCost)) addCount(pipCounts, pip, count);

    if (isLand) {
      landCount += count;
      landCounts.set(name, (landCounts.get(name) || 0) + count);
      const producedMana = cardData.producedMana || [];
      // A source that can produce every color (for example Cavern of Souls)
      // is not evidence that the deck itself uses every color. Keep those
      // wildcards out of the color identity and source circles.
      if (producedMana.length <= 2) {
        for (const color of producedMana) {
          addCount(sourceCounts, color, count);
          colors.add(color);
        }
      }
    }
  }

  const sortedSourceColors = Object.entries(sourceCounts).sort(([, left], [, right]) => right - left);
  const maxSourceCount = sortedSourceColors[0]?.[1] || 0;
  const predominantColors = maxSourceCount
    ? sortedSourceColors.filter(([, count]) => count === maxSourceCount).map(([color]) => color)
    : Object.entries(pipCounts).sort(([, left], [, right]) => right - left).filter(([, count], index, all) => count === all[0]?.[1]).map(([color]) => color);
  const predominantLands = [...landCounts.entries()]
    .sort(([leftName, leftCount], [rightName, rightCount]) => rightCount - leftCount || leftName.localeCompare(rightName, "en"))
    .slice(0, 3)
    .map(([name, count]) => ({ name, count }));
  const uniqueCardNames = [...new Set(mainboard.map((card) => text(card?.name)).filter(Boolean))];

  return {
    colors: [...colors].sort(),
    pipCounts,
    sourceCounts,
    landCount,
    predominantColors,
    predominantLands,
    metadataCoverage: {
      source: [...metadataSources].sort().join("+") || "none",
      resolvedCardCount,
      totalCardCount: uniqueCardNames.length,
      unresolvedCardNames: [...new Set(unresolvedCardNames)].sort(),
      complete: resolvedCardCount === uniqueCardNames.length,
    },
  };
}

export function enrichDeckWithManaProfile(deck, metadata) {
  const profile = buildManaProfile(deck, metadata);
  return {
    ...deck,
    colors: profile.colors,
    manaProfile: profile,
  };
}
