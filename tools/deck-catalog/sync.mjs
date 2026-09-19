import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  extractDeckLinks,
  extractArchetypeLinks,
  extractEventCollections,
  extractEventLinks,
  fetchText,
  formatUrl,
  normalizeFormat,
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

async function sync() {
  const outputDir = resolve(argument("output", DEFAULT_OUTPUT_DIR));
  const format = normalizeFormat(argument("format", "modern"));
  const formatDir = join(outputDir, format.slug);
  const detailsDir = join(formatDir, "details");
  const statePath = join(outputDir, "state", `mtgtop8-${format.slug}.json`);
  const eventLimit = integerArgument("events", 1);
  const deckLimit = integerArgument("limit", 24);
  const collectionDeckLimit = integerArgument("collection-limit", Math.max(1, Math.floor(deckLimit / 2)));
  const recentEventLimit = integerArgument("recent-events", 20);
  const majorEventLimit = integerArgument("major-events", 5);
  const meta = integerArgument("meta", 54);
  const page = integerArgument("page", 0);
  const monoMeta = integerArgument("mono-meta", 51);
  const archetypeLimit = integerArgument("archetype-limit", 6);
  const archetypeDeckLimit = integerArgument("archetype-decks", 4);
  const includeMono = hasFlag("include-mono") || page === 0;
  if (!deckLimit) throw new Error("--limit must be greater than zero");

  const state = await readState(statePath);
  const formatHtml = await fetchText(formatUrl({ format: format.code, meta, page }), { minDelayMs: 0 });
  const monoFormatHtml = includeMono && monoMeta !== meta
    ? await fetchText(formatUrl({ format: format.code, meta: monoMeta, page: 0 }), { minDelayMs: 0 })
    : formatHtml;
  const monoArchetypes = includeMono
    ? extractArchetypeLinks(monoFormatHtml, { limit: archetypeLimit })
    : [];
  const recentCollections = extractEventCollections(formatHtml, {
    recentLimit: recentEventLimit,
    majorLimit: majorEventLimit,
  });
  const events = page === 0
    ? [
      ...recentCollections.lastMajorEvents.map((event) => ({ ...event, collection: "last-major-events" })),
      ...recentCollections.last20Events.map((event) => ({ ...event, collection: "last-20-events" })),
    ]
    : extractEventLinks(formatHtml, { limit: eventLimit });
  const rawDecksById = new Map();
  let lastEventId = state.lastEventId || "";
  const eventHtmlCache = new Map();
  const deckHtmlCache = new Map();

  const eventGroups = page === 0
    ? [
      { name: "last-major-events", events: recentCollections.lastMajorEvents },
      { name: "last-20-events", events: recentCollections.last20Events },
    ]
    : [{ name: "history", events }];

  for (const group of eventGroups) {
    let groupDeckCount = 0;
    for (const event of group.events) {
      if (groupDeckCount >= (page === 0 ? collectionDeckLimit : deckLimit)) break;
      lastEventId = event.id;
      const eventHtml = eventHtmlCache.has(event.id)
        ? eventHtmlCache.get(event.id)
        : await fetchText(event.url, { minDelayMs: 750 });
      eventHtmlCache.set(event.id, eventHtml);
      const deckLinks = extractDeckLinks(eventHtml, {
        eventId: event.id,
        limit: (page === 0 ? collectionDeckLimit : deckLimit) - groupDeckCount,
      });
      for (const deckLink of deckLinks) {
        if (groupDeckCount >= (page === 0 ? collectionDeckLimit : deckLimit)) break;
        const deckHtml = deckHtmlCache.has(deckLink.deckId)
          ? deckHtmlCache.get(deckLink.deckId)
          : await fetchText(deckLink.url, { minDelayMs: 750 });
        deckHtmlCache.set(deckLink.deckId, deckHtml);
        const parsed = parseDeckPage(deckHtml, {
          eventId: deckLink.eventId,
          deckId: deckLink.deckId,
          sourceUrl: deckLink.url,
          date: event.date || "",
          format: format.slug,
          collections: event.collection ? [event.collection] : (group.name === "history" ? [] : [group.name]),
        });
        const previous = rawDecksById.get(parsed.id);
        rawDecksById.set(parsed.id, previous
          ? { ...parsed, collections: [...new Set([...(previous.collections || []), ...(parsed.collections || [])])] }
          : parsed);
        groupDeckCount += 1;
      }
    }
  }

  for (const archetype of monoArchetypes) {
    const archetypeHtml = await fetchText(archetype.url, { minDelayMs: 750 });
    const deckLinks = extractDeckLinks(archetypeHtml, { limit: archetypeDeckLimit });
    for (const deckLink of deckLinks) {
      const deckHtml = deckHtmlCache.has(deckLink.deckId)
        ? deckHtmlCache.get(deckLink.deckId)
        : await fetchText(deckLink.url, { minDelayMs: 750 });
      deckHtmlCache.set(deckLink.deckId, deckHtml);
      const parsed = parseDeckPage(deckHtml, {
        eventId: deckLink.eventId,
        deckId: deckLink.deckId,
        sourceUrl: deckLink.url,
        format: format.slug,
        collections: ["mono-color"],
      });
      const previous = rawDecksById.get(parsed.id);
      rawDecksById.set(parsed.id, previous
        ? { ...parsed, collections: [...new Set([...(previous.collections || []), ...(parsed.collections || [])])] }
        : parsed);
    }
  }

  const rawDecks = [...rawDecksById.values()];
  const normalizedDecks = normalizeCompetitiveDecks(rawDecks);
  if (hasFlag("dry-run")) {
    console.log(JSON.stringify({
      events: events.length,
      archetypes: monoArchetypes.length,
      decks: normalizedDecks.length,
      collections: Object.fromEntries([...new Set(normalizedDecks.flatMap((deck) => deck.collections || []))]
        .map((collection) => [collection, normalizedDecks.filter((deck) => deck.collections?.includes(collection)).length])),
      ids: normalizedDecks.map((deck) => deck.id),
    }, null, 2));
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
  let existing = { schemaVersion: 1, format: format.slug, generatedAt: "", decks: [] };
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
    format: format.slug,
    generatedAt,
    decks: indexEntries,
  });
  await writeJsonAtomic(searchIndexPath, buildSearchIndex(indexEntries, { generatedAt }));
  await writeJsonAtomic(statePath, {
    ...state,
    schemaVersion: 1,
    source: "mtgtop8",
    format: format.slug,
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
