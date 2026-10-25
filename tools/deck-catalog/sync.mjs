import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import {
  allDecksMeta,
  extractDeckLinks,
  extractArchetypeLinks,
  extractEventCollections,
  extractEventLinks,
  fetchText,
  formatUrl,
  normalizeFormat,
  parseDeckPage,
} from "./sources/mtgtop8.mjs";
import { validateDeckCatalogEntry } from "../../web/ui/src/lib/deck-catalog-import.js";
import { normalizeCompetitiveDecks } from "./normalize.mjs";
import { DEFAULT_OUTPUT_DIR, enrichDecks, readJsonOr, writeCatalogDecks, writeJsonAtomic } from "./catalog-store.mjs";

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

async function readState(statePath) {
  try {
    return JSON.parse(await readFile(statePath, "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return { schemaVersion: 1, source: "mtgtop8", format: "modern" };
    throw error;
  }
}

async function sync() {
  const outputDir = resolve(argument("output", DEFAULT_OUTPUT_DIR));
  const format = normalizeFormat(argument("format", "modern"));
  const formatDir = join(outputDir, format.slug);
  const statePath = join(outputDir, "state", `mtgtop8-${format.slug}.json`);
  const eventLimit = integerArgument("events", 1);
  const deckLimit = integerArgument("limit", 24);
  const collectionDeckLimit = integerArgument("collection-limit", Math.max(1, Math.floor(deckLimit / 2)));
  const recentEventLimit = integerArgument("recent-events", 20);
  const majorEventLimit = integerArgument("major-events", 5);
  const meta = process.argv.includes("--meta") ? integerArgument("meta", 54)
    : allDecksMeta(await fetchText(`https://mtgtop8.com/format?f=${format.code}`));
  const page = integerArgument("page", 0);
  const monoMeta = integerArgument("mono-meta", meta);
  const archetypeLimit = integerArgument("archetype-limit", 6);
  const archetypeDeckLimit = integerArgument("archetype-decks", 4);
  const includeMono = hasFlag("include-mono") || page === 0;
  if (!deckLimit) throw new Error("--limit must be greater than zero");

  const existingIndex = await readJsonOr(join(formatDir, "index.json"), { decks: [] });
  const savedIds = new Set();
  if (hasFlag("skip-existing")) {
    for (const entry of existingIndex.decks || []) {
      if (!entry.detail) continue;
      const detail = await readJsonOr(join(formatDir, entry.detail), null);
      if (detail?.id === entry.id && detail.format === format.slug && validateDeckCatalogEntry(detail).valid) savedIds.add(entry.id);
    }
  }
  const state = await readState(statePath);
  const formatHtml = await fetchText(formatUrl({ format: format.code, meta, page }));
  const monoFormatHtml = includeMono && monoMeta !== meta
    ? await fetchText(formatUrl({ format: format.code, meta: monoMeta, page: 0 }))
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
        limit: hasFlag("skip-existing") ? Number.MAX_SAFE_INTEGER : (page === 0 ? collectionDeckLimit : deckLimit) - groupDeckCount,
      });
      for (const deckLink of deckLinks) {
        if (savedIds.has(`mtgtop8-${deckLink.eventId}-${deckLink.deckId}`)) continue;
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
        if (rawDecksById.size % 50 === 0) console.log(`${format.slug}: fetched ${rawDecksById.size} decks on page ${page || 1}`);
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
  console.log(`${format.slug}: fetched ${rawDecks.length} decks; validating and enriching card metadata`);
  const normalizedDecks = normalizeCompetitiveDecks(rawDecks).filter((deck) => {
    const validation = validateDeckCatalogEntry(deck);
    if (!validation.valid) console.warn(`Skipping ${deck.id}: ${validation.errors.join("; ")}`);
    return validation.valid;
  });
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

  const decks = await enrichDecks(normalizedDecks, {
    outputDir, label: format.slug, allowNetwork: hasFlag("metadata-network"),
  });
  const metadataSource = [...new Set(decks
    .map((deck) => deck.manaProfile?.metadataCoverage?.source)
    .filter(Boolean))].sort().join("+") || "none";
  const { generatedAt } = await writeCatalogDecks(decks, { outputDir, format: format.slug });
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
