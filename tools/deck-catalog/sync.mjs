import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  extractDeckLinks,
  extractEventLinks,
  fetchText,
  modernFormatUrl,
  parseDeckPage,
} from "./sources/mtgtop8.mjs";
import { normalizeCompetitiveDecks } from "./normalize.mjs";
import { buildSearchIndex } from "./index.mjs";
import { enrichDeckWithManaProfile, resolveCardMetadata } from "./card-metadata.mjs";

const DEFAULT_OUTPUT_DIR = fileURLToPath(new URL("../../catalog", import.meta.url));
const DEFAULT_CARD_ROOT = fileURLToPath(new URL("../../web/ui/public/cards", import.meta.url));

function argument(name, fallback = "") {
  const index = process.argv.indexOf(`--${name}`);
  return index >= 0 ? process.argv[index + 1] || fallback : fallback;
}

function hasFlag(name) {
  return process.argv.includes(`--${name}`);
}

function integerArgument(name, fallback) {
  const value = Number(argument(name, fallback));
  return Number.isInteger(value) && value >= 0 ? value : fallback;
}

async function writeJsonAtomic(path, value) {
  const temporaryPath = `${path}.tmp-${process.pid}`;
  await writeFile(temporaryPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
  await rename(temporaryPath, path);
}

async function readState(statePath) {
  try {
    return JSON.parse(await readFile(statePath, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return { schemaVersion: 1, source: "mtgtop8", format: "modern" };
    throw error;
  }
}

async function readJsonOr(path, fallback) {
  try {
    return JSON.parse(await readFile(path, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return fallback;
    throw error;
  }
}

function catalogEntry(deck) {
  return {
    id: deck.id,
    format: deck.format,
    name: deck.name,
    archetype: deck.archetype,
    colors: deck.colors,
    mechanics: deck.mechanics,
    ...(deck.manaProfile ? { manaProfile: deck.manaProfile } : {}),
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

async function sync() {
  const outputDir = resolve(argument("output", DEFAULT_OUTPUT_DIR));
  const formatDir = join(outputDir, "modern");
  const detailsDir = join(formatDir, "details");
  const statePath = join(outputDir, "state", "mtgtop8-modern.json");
  const eventLimit = integerArgument("events", 1);
  const deckLimit = integerArgument("limit", 1);
  const meta = integerArgument("meta", 54);
  const page = integerArgument("page", 0);
  if (!deckLimit) throw new Error("--limit must be greater than zero");

  const state = await readState(statePath);
  const formatHtml = await fetchText(modernFormatUrl({ meta, page }), { minDelayMs: 0 });
  const events = extractEventLinks(formatHtml, { limit: eventLimit });
  const rawDecks = [];
  let lastEventId = state.lastEventId || "";

  for (const event of events) {
    lastEventId = event.id;
    const eventHtml = await fetchText(event.url, { minDelayMs: 750 });
    const deckLinks = extractDeckLinks(eventHtml, { eventId: event.id, limit: deckLimit - rawDecks.length });
    for (const deckLink of deckLinks) {
      if (rawDecks.length >= deckLimit) break;
      const deckHtml = await fetchText(deckLink.url, { minDelayMs: 750 });
      rawDecks.push(parseDeckPage(deckHtml, {
        eventId: deckLink.eventId,
        deckId: deckLink.deckId,
        sourceUrl: deckLink.url,
      }));
    }
    if (rawDecks.length >= deckLimit) break;
  }

  const normalizedDecks = normalizeCompetitiveDecks(rawDecks);
  if (hasFlag("dry-run")) {
    console.log(JSON.stringify({ events: events.length, decks: normalizedDecks.length, ids: normalizedDecks.map((deck) => deck.id) }, null, 2));
    return;
  }

  await mkdir(detailsDir, { recursive: true });
  await mkdir(join(outputDir, "state"), { recursive: true });
  const metadataPath = join(outputDir, "state", "scryfall-cards.json");
  const metadataCache = await readJsonOr(metadataPath, {});
  const cardNames = [...new Set(normalizedDecks.flatMap((deck) => deck.mainboard.map((card) => card.name)))];
  const cardMetadata = await resolveCardMetadata(cardNames, { cache: metadataCache, localRoot: DEFAULT_CARD_ROOT });
  const decks = normalizedDecks.map((deck) => enrichDeckWithManaProfile(deck, cardMetadata));
  const metadataSource = [...new Set(decks
    .map((deck) => deck.manaProfile?.metadataCoverage?.source)
    .filter(Boolean))].sort().join("+") || "none";
  for (const deck of decks) {
    await writeJsonAtomic(join(detailsDir, `${deck.id}.json`), deck);
  }
  await writeJsonAtomic(metadataPath, cardMetadata);

  const indexPath = join(formatDir, "index.json");
  const searchIndexPath = join(formatDir, "search-index.json");
  let existing = { schemaVersion: 1, format: "modern", generatedAt: "", decks: [] };
  try {
    existing = JSON.parse(await readFile(indexPath, "utf8"));
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  const merged = new Map((existing.decks || []).map((entry) => [entry.id, entry]));
  for (const deck of decks) merged.set(deck.id, catalogEntry(deck));
  const generatedAt = new Date().toISOString();
  const indexEntries = [...merged.values()].sort((left, right) => right.date.localeCompare(left.date));
  await writeJsonAtomic(indexPath, {
    ...existing,
    schemaVersion: 1,
    format: "modern",
    generatedAt,
    decks: indexEntries,
  });
  await writeJsonAtomic(searchIndexPath, buildSearchIndex(indexEntries, { generatedAt }));
  await writeJsonAtomic(statePath, {
    ...state,
    schemaVersion: 1,
    source: "mtgtop8",
    format: "modern",
    lastEventId,
    lastPage: page,
    updatedAt: generatedAt,
    metadataSource,
    metadataUpdatedAt: generatedAt,
  });
  console.log(JSON.stringify({ events: events.length, decks: decks.length, outputDir }, null, 2));
}

sync().catch((error) => {
  console.error(error.stack || error.message || error);
  process.exitCode = 1;
});
