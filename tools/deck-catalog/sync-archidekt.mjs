// Fills a format's catalog from Archidekt's public decks, most viewed first,
// for formats MTGTop8 does not cover (Pauper EDH). Only decks that pass the
// format's legality check are kept.
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { deckDetailUrl, deckSearchUrl, fetchJson, parseDeckDetail, parseDeckSearch } from "./sources/archidekt.mjs";
import { catalogFormat } from "./formats.mjs";
import { normalizeCompetitiveDeck } from "./normalize.mjs";
import { DEFAULT_OUTPUT_DIR, enrichDecks, readJsonOr, writeCatalogDecks, writeJsonAtomic } from "./catalog-store.mjs";
import { validateDeckCatalogEntry } from "../../web/ui/src/lib/deck-catalog-import.js";
import { validateFormatDeck } from "../../web/ui/src/lib/relay/format-legality.js";
import { usesCommanders } from "../../web/ui/src/lib/relay/formats.js";
import { unsupportedCommanders } from "./engine-support.mjs";

const FORMAT_CATALOG = fileURLToPath(new URL("../../web/ui/src/lib/relay/format-catalog.generated.json", import.meta.url));

function argument(name, fallback = "") {
  const index = process.argv.indexOf(`--${name}`);
  return index >= 0 ? process.argv[index + 1] || fallback : fallback;
}

function integerArgument(name, fallback) {
  const value = Number(argument(name, fallback));
  return Number.isInteger(value) && value > 0 ? value : fallback;
}

const expand = (cards) => cards.flatMap((card) => Array(card.count).fill(card.name));

export function deckProblems(deck, format, legality) {
  const shape = validateDeckCatalogEntry(deck);
  if (!shape.valid) return shape.errors;
  return validateFormatDeck(format, expand(deck.mainboard), expand(deck.commander), expand(deck.sideboard), legality).errors;
}

async function sync() {
  const outputDir = resolve(argument("output", DEFAULT_OUTPUT_DIR));
  const format = catalogFormat(argument("format", "paupercommander"));
  if (format.source !== "archidekt") throw new Error(`${format.slug} decks come from ${format.source}, not Archidekt`);
  const target = integerArgument("target", 500);
  const maxPages = integerArgument("max-pages", 60);
  const legality = JSON.parse(await readFile(FORMAT_CATALOG, "utf8"));
  if (format.slug === "paupercommander" && !Object.values(legality.cards).some((card) => card.pdhCommander)) {
    throw new Error("The format catalog has no Pauper EDH commanders; run `pnpm formats:build` in web/ui first");
  }

  const formatDir = join(outputDir, format.slug);
  const index = await readJsonOr(join(formatDir, "index.json"), { decks: [] });
  const saved = new Set();
  for (const entry of index.decks || []) {
    const detail = entry.detail ? await readJsonOr(join(formatDir, entry.detail), null) : null;
    if (detail?.id === entry.id && !deckProblems(detail, format.slug, legality).length
      && !(await unsupportedCommanders(detail)).length) saved.add(entry.id);
  }
  console.log(`${format.slug}: ${saved.size}/${target} legal decks already saved`);

  let rejected = 0;
  for (let page = 1; page <= maxPages && saved.size < target; page += 1) {
    const search = parseDeckSearch(await fetchJson(deckSearchUrl({ format: format.slug, page })));
    const batch = [];
    for (const listing of search.decks) {
      if (saved.size + batch.length >= target) break;
      if (saved.has(`archidekt-${listing.id}`) || listing.size < 99 || listing.size > 101) continue;
      let deck;
      try {
        const raw = parseDeckDetail(await fetchJson(deckDetailUrl(listing.id)), { format: format.slug });
        // Commander decks have no sideboard; Archidekt's holds considered cuts.
        deck = normalizeCompetitiveDeck(usesCommanders(format.slug) ? { ...raw, sideboard: [] } : raw);
      } catch (error) {
        console.warn(`Skipping archidekt-${listing.id}: ${error.message}`);
        rejected += 1;
        continue;
      }
      deck = { ...deck, event: "Archidekt", viewCount: listing.viewCount };
      const unsupported = await unsupportedCommanders(deck);
      const problems = [
        ...deckProblems(deck, format.slug, legality),
        ...(unsupported.length ? [`The engine cannot load ${unsupported.join(", ")}.`] : []),
      ];
      if (problems.length) {
        rejected += 1;
        console.warn(`Skipping ${deck.id} (${deck.name}): ${problems.slice(0, 3).join(" ")}`);
        continue;
      }
      batch.push(deck);
    }
    if (batch.length) {
      // Single-color decks feed the browser's mono-color view.
      const decks = (await enrichDecks(batch, { outputDir, label: format.slug }))
        .map((deck) => deck.colors?.length === 1 ? { ...deck, collections: [...new Set([...deck.collections, "mono-color"])].sort() } : deck);
      await writeCatalogDecks(decks, { outputDir, format: format.slug });
      for (const deck of decks) saved.add(deck.id);
    }
    console.log(`${format.slug}: ${saved.size}/${target} legal decks after page ${page} (${rejected} rejected)`);
    if (!search.hasNext) break;
  }

  await writeJsonAtomic(join(outputDir, "state", `archidekt-${format.slug}.json`), {
    schemaVersion: 1, source: "archidekt", format: format.slug, updatedAt: new Date().toISOString(), saved: saved.size, rejected,
  });
  if (saved.size < target) throw new Error(`${format.slug}: only ${saved.size}/${target} legal decks; raise --max-pages and rerun`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  sync().catch((error) => {
    console.error(error.stack || error.message || error);
    process.exitCode = 1;
  });
}
