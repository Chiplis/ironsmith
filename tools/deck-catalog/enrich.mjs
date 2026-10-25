import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { buildSearchIndex } from "./index.mjs";
import { enrichDeckWithManaProfile, readBulkCardMetadata, resolveCardMetadata } from "./card-metadata.mjs";
import { catalogFormat } from "./formats.mjs";
import { pruneUnsupportedCommanderDecks } from "./engine-support.mjs";

const DEFAULT_OUTPUT_DIR = fileURLToPath(new URL("../../catalog", import.meta.url));
const DEFAULT_CARD_ROOT = fileURLToPath(new URL("../../web/ui/public/cards", import.meta.url));
const DEFAULT_BULK_CARDS = fileURLToPath(new URL("../../cards.json", import.meta.url));

function argument(name, fallback = "") {
  const index = process.argv.indexOf(`--${name}`);
  return index >= 0 ? process.argv[index + 1] || fallback : fallback;
}

async function readJson(path, fallback) {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return fallback;
    throw error;
  }
}

async function writeJsonAtomic(path, value) {
  const temporaryPath = `${path}.tmp-${process.pid}`;
  await writeFile(temporaryPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  await rename(temporaryPath, path);
}

async function enrich() {
  const outputDir = resolve(argument("output", DEFAULT_OUTPUT_DIR));
  const format = catalogFormat(argument("format", "modern"));
  const formatDir = join(outputDir, format.slug);
  const detailsDir = join(formatDir, "details");
  const indexPath = join(formatDir, "index.json");
  const searchIndexPath = join(formatDir, "search-index.json");
  const statePath = join(outputDir, "state", `${format.source}-${format.slug}.json`);
  const metadataPath = join(outputDir, "state", "scryfall-cards.json");
  const index = await readJson(indexPath, { schemaVersion: 1, format: format.slug, decks: [] });
  const loaded = [];
  for (const entry of index.decks || []) {
    if (!entry.detail) continue;
    const detail = await readJson(join(formatDir, entry.detail), null);
    if (detail) loaded.push(detail);
  }
  // Card support changes with every engine build, so decks saved before their
  // commander broke are dropped here rather than only at download time.
  const { kept: details, removed } = await pruneUnsupportedCommanderDecks(loaded, { formatDir, label: format.slug });
  const removedIds = new Set(removed.map((deck) => deck.id));
  const metadataCache = { ...await readJson(metadataPath, {}), ...await readBulkCardMetadata(DEFAULT_BULK_CARDS) };
  const cardNames = [...new Set(details.flatMap((deck) => (deck.mainboard || []).map((card) => card.name)))];
  const cardMetadata = await resolveCardMetadata(cardNames, {
    cache: metadataCache,
    localRoot: DEFAULT_CARD_ROOT,
    allowNetwork: !process.argv.includes("--offline"),
  });
  const enriched = details.map((deck) => enrichDeckWithManaProfile(deck, cardMetadata));
  const byId = new Map(enriched.map((deck) => [deck.id, deck]));
  const entries = (index.decks || []).filter((entry) => !removedIds.has(entry.id)).map((entry) => {
    const deck = byId.get(entry.id);
    if (!deck) return entry;
    return { ...entry, colors: deck.colors, manaProfile: deck.manaProfile, ...(deck.artCard ? { artCard: deck.artCard } : {}) };
  });
  const generatedAt = new Date().toISOString();
  await mkdir(join(outputDir, "state"), { recursive: true });
  for (const deck of enriched) await writeJsonAtomic(join(detailsDir, `${deck.id}.json`), deck);
  await writeJsonAtomic(metadataPath, cardMetadata);
  await writeJsonAtomic(indexPath, { ...index, generatedAt, decks: entries });
  await writeJsonAtomic(searchIndexPath, buildSearchIndex(entries, { generatedAt }));
  const state = await readJson(statePath, { schemaVersion: 1, source: format.source, format: format.slug });
  const metadataSource = [...new Set(enriched
    .map((deck) => deck.manaProfile?.metadataCoverage?.source)
    .filter(Boolean))].sort().join("+") || "none";
  await writeJsonAtomic(statePath, { ...state, metadataSource, metadataUpdatedAt: generatedAt });
  console.log(JSON.stringify({ decks: enriched.length, removed: removed.length, cards: Object.keys(cardMetadata).length, outputDir }, null, 2));
}

enrich().catch((error) => {
  console.error(error.stack || error.message || error);
  process.exitCode = 1;
});
