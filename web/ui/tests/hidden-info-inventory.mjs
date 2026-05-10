import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { extname, join, relative, resolve } from "node:path";

const repoRoot = resolve(new URL("../../..", import.meta.url).pathname);
const scanRoots = [
  "crates/ironsmith-runtime/src",
  "crates/ironsmith-wasm/src",
  "web/ui/src",
];
const categories = [
  ["library", /\blibrary\b|\bdeck\b/gi],
  ["hand", /\bhand\b/gi],
  ["search", /\bsearch\b|\btutor\b/gi],
  ["reveal", /\breveal(?:ed|s|ing)?\b|\blook at\b/gi],
  ["shuffle", /\bshuffle(?:d|s|ing)?\b|ziffle|mental-poker/gi],
  ["scry-surveil", /\bscry\b|\bsurveil\b/gi],
  ["face-down", /face[-_ ]?down|\bmanifest\b|\bcloak\b|\bdisguise\b|\bmorph\b/gi],
  ["exile-hidden", /\bexile\b.*\bface[-_ ]?down\b|\bhiddenCard\b/gi],
  ["sideboard-rematch", /\bsideboard\b|\brematch\b/gi],
  ["redacted-sync", /\bredact(?:ed|ion)?\b|\bhiddenDeckManifests\b|\bexportRedactedSyncCheckpoint\b/gi],
  ["audit-opening", /\bdeckAuditManifest\b|\bbuildDeckSlotOpening\b|\bverifyCardOpeningAgainstManifest\b/gi],
];

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (["target", "node_modules", "dist", "pkg"].includes(entry.name)) continue;
      files.push(...await walk(path));
      continue;
    }
    if (![".rs", ".js", ".jsx", ".ts", ".tsx"].includes(extname(entry.name))) continue;
    files.push(path);
  }
  return files;
}

const files = (await Promise.all(scanRoots.map((root) => walk(resolve(repoRoot, root)))))
  .flat();
const inventory = Object.fromEntries(categories.map(([name]) => [name, []]));

for (const file of files) {
  const text = await readFile(file, "utf8");
  const lines = text.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    for (const [category, pattern] of categories) {
      pattern.lastIndex = 0;
      if (!pattern.test(line)) continue;
      inventory[category].push({
        file: relative(repoRoot, file),
        line: index + 1,
        text: line.trim().slice(0, 180),
      });
    }
  }
}

for (const [category] of categories) {
  assert(
    inventory[category].length > 0,
    `hidden-info inventory category "${category}" had no matches`,
  );
}

const summary = Object.fromEntries(
  Object.entries(inventory).map(([category, entries]) => [category, entries.length]),
);
console.log(JSON.stringify({
  ok: true,
  scannedFiles: files.length,
  summary,
  sample: Object.fromEntries(
    Object.entries(inventory).map(([category, entries]) => [category, entries.slice(0, 8)]),
  ),
}, null, 2));
