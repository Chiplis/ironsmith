// A commander deck is only playable when the engine can load its commander:
// the 99 can fall back to basic lands, but a commander cannot. The baked card
// assets record whether each card compiled, so they decide which decks stay.
import { readFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

export const DEFAULT_CARD_ROOT = fileURLToPath(new URL("../../web/ui/public/cards", import.meta.url));

function cardRouteKey(name) {
  return String(name || "")
    .trim()
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .toLocaleLowerCase("en-US")
    .replace(/[^a-z0-9_]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

const statusCache = new Map();

async function readCardAsset(root, name) {
  const route = cardRouteKey(name);
  if (!route) return null;
  try {
    return JSON.parse(await readFile(join(root, `${route}.json`), "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

// "unsupported" when the asset says the engine failed to compile the card,
// "unknown" when there is no asset to ask (an unbaked checkout or a name the
// corpus lacks), so a missing build never empties the catalog.
export async function cardEngineStatus(name, { root = DEFAULT_CARD_ROOT } = {}) {
  const cacheKey = `${root}\u0000${name}`;
  if (statusCache.has(cacheKey)) return statusCache.get(cacheKey);
  const request = (async () => {
    // Exports sometimes print only the front face of a double-faced card.
    const asset = await readCardAsset(root, name)
      ?? (String(name).includes("//") ? await readCardAsset(root, String(name).split("//")[0]) : null);
    if (!asset) return "unknown";
    if (asset.artifactError || !Array.isArray(asset.artifacts) || !asset.artifacts.length) return "unsupported";
    return asset.artifacts.some((artifact) => artifact?.diagnostics?.errorCount > 0) ? "unsupported" : "ok";
  })();
  statusCache.set(cacheKey, request);
  return request;
}

export async function unsupportedCommanders(deck, options = {}) {
  const names = [...new Set((deck?.commander || []).map((card) => card.name))];
  const statuses = await Promise.all(names.map((name) => cardEngineStatus(name, options)));
  return names.filter((_, index) => statuses[index] === "unsupported");
}

// Splits decks into those the engine can host and those it cannot, deleting
// the rejected decks' detail files when a format directory is given.
export async function pruneUnsupportedCommanderDecks(decks, { formatDir = "", root = DEFAULT_CARD_ROOT, label = "" } = {}) {
  const kept = [];
  const removed = [];
  for (const deck of decks) {
    const unsupported = await unsupportedCommanders(deck, { root });
    if (!unsupported.length) {
      kept.push(deck);
      continue;
    }
    removed.push({ id: deck.id, name: deck.name, unsupported });
    if (formatDir) await rm(join(formatDir, "details", `${deck.id}.json`), { force: true });
  }
  for (const { id, name, unsupported } of removed) {
    console.warn(`${label || "catalog"}: dropping ${id} (${name}); the engine cannot load ${unsupported.join(", ")}`);
  }
  return { kept, removed };
}
