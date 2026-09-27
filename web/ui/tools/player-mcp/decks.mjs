import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { importDeckCatalogEntry } from "../../src/lib/deck-catalog-import.js";

const DEFAULT_ROOTS = [
  fileURLToPath(new URL("../../../../catalog/", import.meta.url)),
  fileURLToPath(new URL("../../public/catalog/", import.meta.url)),
];

function normalized(value) {
  return String(value ?? "").normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "").toLowerCase()
    .replace(/[^a-z0-9]+/g, " ").trim();
}

function formatName(format) {
  if (format == null || format === "") return null;
  const value = String(format).trim().toLowerCase();
  if (!/^[a-z][a-z0-9-]*$/.test(value)) throw new Error("Invalid catalog format");
  return value;
}

function summary(entry, catalog) {
  return {
    id: entry.id,
    format: catalog.format,
    name: entry.name || entry.archetype || entry.id,
    archetype: entry.archetype || "",
    colors: entry.colors || [],
    event: entry.event || "",
    date: entry.date || "",
    placement: entry.placement ?? null,
    source: entry.source || "",
    sourceUrl: entry.sourceUrl || "",
    counts: {
      mainboard: entry.mainboardCount ?? null,
      sideboard: entry.sideboardCount ?? null,
      commander: entry.commanderCount ?? null,
    },
    catalogGeneratedAt: catalog.generatedAt || null,
  };
}

// Generated tournament catalogs are local artifacts. Read them fresh so a
// running MCP server sees a newly synchronized catalog without restarting.
export function createDeckCatalog({ catalogRoots = DEFAULT_ROOTS } = {}) {
  async function catalogs(format = null) {
    const found = new Map();
    for (const root of catalogRoots) {
      let formats;
      try {
        formats = format ? [format] : (await readdir(root, { withFileTypes: true }))
          .filter(entry => entry.isDirectory() && /^[a-z][a-z0-9-]*$/.test(entry.name))
          .map(entry => entry.name).sort();
      } catch (error) {
        if (error.code === "ENOENT") continue;
        throw error;
      }
      for (const currentFormat of formats) {
        if (found.has(currentFormat)) continue;
        const directory = path.resolve(root, currentFormat);
        let index;
        try {
          index = JSON.parse(await readFile(path.join(directory, "index.json"), "utf8"));
        } catch (error) {
          if (error.code === "ENOENT") continue;
          throw error;
        }
        if (!Array.isArray(index.decks) || index.format !== currentFormat) {
          throw new Error(`Invalid tournament catalog index: ${directory}`);
        }
        found.set(currentFormat, { ...index, directory });
      }
    }
    if (found.size === 0) {
      throw new Error(`No local tournament deck catalog${format ? ` for ${format}` : ""}. Generate it with tools/deck-catalog/sync.mjs first.`);
    }
    return [...found.values()];
  }

  async function listDecks({ format, query = "", limit = 20 } = {}) {
    const count = Number(limit);
    if (!Number.isInteger(count) || count < 0 || count > 200) {
      throw new Error("Deck limit must be an integer between 0 and 200");
    }
    const words = normalized(query).split(/\s+/).filter(Boolean);
    const matches = (await catalogs(formatName(format))).flatMap(catalog =>
      catalog.decks.filter(entry => {
        const haystack = normalized([
          entry.name, entry.archetype, entry.event, catalog.format,
          ...(entry.cardNames || entry.cards || []), ...(entry.tags || []),
        ].join(" "));
        return words.every(word => haystack.includes(word));
      }).map(entry => summary(entry, catalog)));
    matches.sort((left, right) =>
      right.date.localeCompare(left.date)
      || (left.placement ?? Infinity) - (right.placement ?? Infinity)
      || left.id.localeCompare(right.id));
    return matches.slice(0, count);
  }

  async function getDeck({ id } = {}) {
    if (typeof id !== "string" || !/^[a-zA-Z0-9_-]+$/.test(id)) {
      throw new Error("A valid catalog deck id is required");
    }
    const matches = (await catalogs()).flatMap(catalog =>
      catalog.decks.filter(entry => entry.id === id).map(entry => ({ entry, catalog })));
    if (matches.length === 0) throw new Error(`Tournament deck not found: ${id}`);
    if (matches.length > 1) throw new Error(`Ambiguous tournament deck id: ${id}`);
    const { entry, catalog } = matches[0];
    if (typeof entry.detail !== "string" || !entry.detail) {
      throw new Error(`Tournament deck has no detail file: ${id}`);
    }
    const detailPath = path.resolve(catalog.directory, entry.detail);
    const relative = path.relative(catalog.directory, detailPath);
    if (!relative || relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
      throw new Error(`Tournament deck detail escapes its catalog: ${id}`);
    }
    const detail = JSON.parse(await readFile(detailPath, "utf8"));
    if (detail.id !== id || detail.format !== catalog.format) {
      throw new Error(`Tournament deck detail does not match its index: ${id}`);
    }
    for (const section of ["mainboard", "sideboard", "commander"]) {
      if (!Array.isArray(detail[section]) || detail[section].some(card =>
        typeof card?.name !== "string" || !card.name.trim()
        || !Number.isInteger(card.count) || card.count < 1)) {
        throw new Error(`Invalid ${section} in tournament deck: ${id}`);
      }
    }
    const imported = importDeckCatalogEntry(detail, { format: detail.format });
    return {
      ...detail,
      deckText: imported.deckText,
      commanderText: imported.commanderText,
      counts: imported.counts,
      sourceMetadata: {
        source: detail.source,
        sourceUrl: detail.sourceUrl || "",
        event: detail.event || "",
        date: detail.date || "",
        placement: detail.placement ?? null,
        catalogGeneratedAt: catalog.generatedAt || null,
        catalogPath: detailPath,
        hash: detail.hash || null,
      },
    };
  }

  return { listDecks, getDeck };
}

export const { listDecks, getDeck } = createDeckCatalog();
