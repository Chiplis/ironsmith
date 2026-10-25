import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { catalogFormat } from "./formats.mjs";
import { unsupportedCommanders } from "./engine-support.mjs";
import { validateDeckCatalogEntry } from "../../web/ui/src/lib/deck-catalog-import.js";

const toolDir = fileURLToPath(new URL(".", import.meta.url));

export async function countSavedDecks(output, format, { cardRoot } = {}) {
  let index;
  try {
    index = JSON.parse(await readFile(join(output, format, "index.json"), "utf8"));
  } catch (error) {
    if (error.code === "ENOENT") return 0;
    throw error;
  }
  const ids = new Set();
  for (const entry of index.decks || []) {
    if (!entry.detail) continue;
    try {
      const deck = JSON.parse(await readFile(join(output, format, entry.detail), "utf8"));
      if (deck.id === entry.id && deck.format === format && validateDeckCatalogEntry(deck).valid
        && !(await unsupportedCommanders(deck, { root: cardRoot })).length) ids.add(deck.id);
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  return ids.size;
}

function runSync(args, script = "sync.mjs") {
  return new Promise((resolveRun, reject) => {
    const child = spawn(process.execPath, [join(toolDir, script), ...args], { stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (code, signal) => code === 0 ? resolveRun() : reject(new Error(`Deck sync failed (${signal || code})`)));
  });
}

export async function ensureCatalog({ format, output, minimum = 500, maxPages = 100, count = countSavedDecks, sync = runSync }) {
  const { slug, source } = catalogFormat(format);
  format = slug;
  if (!Number.isInteger(minimum) || minimum < 1) throw new Error("Minimum deck count must be a positive integer");
  if (!Number.isInteger(maxPages) || maxPages < 1) throw new Error("Maximum pages must be a positive integer");
  if (source === "archidekt") {
    // Archidekt pages are deck listings, so one run walks as many as it needs.
    await sync(["--format", format, "--output", output, "--target", String(minimum), "--max-pages", String(maxPages)], "sync-archidekt.mjs");
    return count(output, format);
  }
  let saved = await count(output, format);
  // Always refresh the featured/recent sample, even when history is already full.
  for (let pageIndex = 0; pageIndex < maxPages; pageIndex += 1) {
    const page = pageIndex === 0 ? 0 : pageIndex + 1;
    await sync([
      "--format", format, "--output", output, "--page", String(page),
      "--events", "25", "--limit", String(Math.max(24, minimum - saved)),
      "--collection-limit", "24", "--recent-events", "20", "--major-events", "5",
      ...(page ? ["--skip-existing"] : []),
    ]);
    saved = await count(output, format);
    console.log(`${format}: ${saved}/${minimum} saved decks (page ${page || 1})`);
    if (saved >= minimum) return saved;
  }
  throw new Error(`${format}: only ${saved}/${minimum} usable decks after ${maxPages} pages; partial catalog saved. Increase DECK_CATALOG_MAX_PAGES and rerun.`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  ensureCatalog({
    format: process.argv[2] || "modern",
    output: resolve(process.env.DECK_CATALOG_OUTPUT || join(toolDir, "../../catalog")),
    minimum: Number(process.env.DECK_CATALOG_MIN_DECKS || 500),
    maxPages: Number(process.env.DECK_CATALOG_MAX_PAGES || 100),
  }).catch((error) => {
    console.error(error.stack || error);
    process.exitCode = 1;
  });
}
