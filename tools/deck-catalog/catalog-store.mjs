import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { buildSearchIndex } from "./index.mjs";
import { enrichDeckWithManaProfile, readBulkCardMetadata, resolveCardMetadata } from "./card-metadata.mjs";
import { pruneUnsupportedCommanderDecks } from "./engine-support.mjs";

export const DEFAULT_OUTPUT_DIR = fileURLToPath(new URL("../../catalog", import.meta.url));
export const DEFAULT_CARD_ROOT = fileURLToPath(new URL("../../web/ui/public/cards", import.meta.url));
export const DEFAULT_BULK_CARDS = fileURLToPath(new URL("../../cards.json", import.meta.url));

export async function writeJsonAtomic(path, value) {
  const temporaryPath = `${path}.tmp-${process.pid}`;
  await writeFile(temporaryPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  await rename(temporaryPath, path);
}

export async function readJsonOr(path, fallback) {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return fallback;
    throw error;
  }
}

export function catalogEntry(deck) {
  return {
    id: deck.id,
    format: deck.format,
    name: deck.name,
    archetype: deck.archetype,
    colors: deck.colors,
    mechanics: deck.mechanics,
    collections: deck.collections,
    ...(deck.manaProfile ? { manaProfile: deck.manaProfile } : {}),
    ...(deck.artCard ? { artCard: deck.artCard } : {}),
    cardNames: deck.cardNames,
    event: deck.event,
    date: deck.date,
    placement: deck.placement,
    mainboardCount: deck.mainboard.reduce((total, card) => total + card.count, 0),
    sideboardCount: deck.sideboard.reduce((total, card) => total + card.count, 0),
    commanderCount: deck.commander.reduce((total, card) => total + card.count, 0),
    source: deck.source,
    sourceUrl: deck.sourceUrl,
    tags: deck.tags,
    cards: [...new Set([
      ...deck.mainboard.map((card) => card.name),
      ...deck.sideboard.map((card) => card.name),
      ...deck.commander.map((card) => card.name),
    ])].sort(),
    hash: deck.hash,
    detail: `details/${deck.id}.json`,
  };
}

// Adds mana profiles to normalized decks from the local card metadata.
export async function enrichDecks(decks, { outputDir, label, allowNetwork = false }) {
  await mkdir(join(outputDir, "state"), { recursive: true });
  const metadataPath = join(outputDir, "state", "scryfall-cards.json");
  const metadataCache = {
    ...await readJsonOr(metadataPath, {}),
    ...await readBulkCardMetadata(DEFAULT_BULK_CARDS),
  };
  const cardNames = [...new Set(decks.flatMap((deck) => deck.mainboard.map((card) => card.name)))];
  let resolvedCount = 0;
  const cardMetadata = await resolveCardMetadata(cardNames, {
    cache: metadataCache, localRoot: DEFAULT_CARD_ROOT,
    allowNetwork,
    onResolve: () => {
      resolvedCount += 1;
      if (resolvedCount % 100 === 0) console.log(`${label}: processed metadata for ${resolvedCount} cards`);
    },
  });
  await writeJsonAtomic(metadataPath, cardMetadata);
  return decks.map((deck) => enrichDeckWithManaProfile(deck, cardMetadata));
}

// Writes each deck's detail file and merges the decks into the format index.
export async function writeCatalogDecks(decks, { outputDir, format }) {
  const formatDir = join(outputDir, format);
  ({ kept: decks } = await pruneUnsupportedCommanderDecks(decks, { label: format }));
  await mkdir(join(formatDir, "details"), { recursive: true });
  for (const deck of decks) {
    await writeJsonAtomic(join(formatDir, "details", `${deck.id}.json`), deck);
  }
  const indexPath = join(formatDir, "index.json");
  const existing = await readJsonOr(indexPath, { schemaVersion: 1, format, generatedAt: "", decks: [] });
  const merged = new Map((existing.decks || []).map((entry) => [entry.id, entry]));
  for (const deck of decks) merged.set(deck.id, catalogEntry(deck));
  const generatedAt = new Date().toISOString();
  const indexEntries = [...merged.values()].sort((left, right) => right.date.localeCompare(left.date));
  await writeJsonAtomic(indexPath, { ...existing, schemaVersion: 1, format, generatedAt, decks: indexEntries });
  await writeJsonAtomic(join(formatDir, "search-index.json"), buildSearchIndex(indexEntries, { generatedAt }));
  return { generatedAt, total: indexEntries.length };
}
