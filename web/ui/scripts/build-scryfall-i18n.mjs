import { createWriteStream } from "node:fs";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import https from "node:https";
import path from "node:path";
import { pipeline } from "node:stream/promises";

const ROOT = process.cwd();
const DEFAULT_LOCALE = "es";
const BULK_DATA_URL = "https://api.scryfall.com/bulk-data";

function parseArgs(argv) {
  const args = {
    locale: DEFAULT_LOCALE,
    bulk: "",
    download: true,
    keepBulk: false,
    outDir: path.join("public", "card-i18n"),
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--") continue;
    if (arg === "--locale" || arg === "-l") {
      args.locale = argv[++index] || DEFAULT_LOCALE;
    } else if (arg === "--bulk") {
      args.bulk = argv[++index] || "";
      args.download = false;
    } else if (arg === "--out-dir") {
      args.outDir = argv[++index] || args.outDir;
    } else if (arg === "--keep-bulk") {
      args.keepBulk = true;
    } else if (arg === "--help" || arg === "-h") {
      args.help = true;
    }
  }
  return args;
}

function requestJson(url) {
  return new Promise((resolve, reject) => {
    https.get(url, {
      headers: {
        Accept: "application/json;q=0.9,*/*;q=0.8",
        "User-Agent": "Ironsmith i18n asset builder (local development)",
      },
    }, (response) => {
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        resolve(requestJson(new URL(response.headers.location, url).href));
        return;
      }
      if (response.statusCode !== 200) {
        reject(new Error(`HTTP ${response.statusCode} for ${url}`));
        response.resume();
        return;
      }
      const chunks = [];
      response.on("data", (chunk) => chunks.push(chunk));
      response.on("end", () => {
        try {
          resolve(JSON.parse(Buffer.concat(chunks).toString("utf8")));
        } catch (error) {
          reject(error);
        }
      });
    }).on("error", reject);
  });
}

async function downloadFile(url, outFile) {
  await mkdir(path.dirname(outFile), { recursive: true });
  await new Promise((resolve, reject) => {
    https.get(url, {
      headers: {
        Accept: "application/json;q=0.9,*/*;q=0.8",
        "User-Agent": "Ironsmith i18n asset builder (local development)",
      },
    }, async (response) => {
      if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
        try {
          await downloadFile(new URL(response.headers.location, url).href, outFile);
          resolve();
        } catch (error) {
          reject(error);
        }
        return;
      }
      if (response.statusCode !== 200) {
        reject(new Error(`HTTP ${response.statusCode} for ${url}`));
        response.resume();
        return;
      }
      try {
        await pipeline(response, createWriteStream(outFile));
        resolve();
      } catch (error) {
        reject(error);
      }
    }).on("error", reject);
  });
}

function routeKey(name) {
  return String(name || "")
    .trim()
    .toLocaleLowerCase("en-US")
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function firstFaceValue(card, field) {
  if (card?.[field]) return card[field];
  if (Array.isArray(card?.card_faces)) {
    return card.card_faces.map((face) => face?.[field]).filter(Boolean).join("\n//\n");
  }
  return "";
}

function parenGroupCount(text) {
  return (String(text || "").match(/\(/g) || []).length;
}

// Scryfall has no localized oracle text — only per-printing printed_text, which
// includes reminder text only when that physical printing carried it. Rank each
// localized printing so we keep the one closest to the current English oracle text:
// 2 = printed_text present with at least as many parenthetical (reminder) groups
//     as the English oracle text, 1 = printed_text present, 0 = no digitized text
// (still useful for name/typeLine). Never backfill rules text with English.
function printingScore(englishCard, localizedCard) {
  const printedText = firstFaceValue(localizedCard, "printed_text");
  if (!printedText) return 0;
  const englishParens = parenGroupCount(firstFaceValue(englishCard, "oracle_text"));
  return parenGroupCount(printedText) >= englishParens ? 2 : 1;
}

function translatedPayload(englishCard, localizedCard, locale) {
  const oracleId = String(localizedCard.oracle_id || englishCard.oracle_id || "").trim();
  const englishName = firstFaceValue(englishCard, "name");
  return {
    schemaVersion: 1,
    source: "scryfall",
    sourceLang: "en",
    targetLang: locale,
    oracleId,
    englishName,
    route: routeKey(englishName),
    name: firstFaceValue(localizedCard, "printed_name") || firstFaceValue(localizedCard, "name") || "",
    typeLine: firstFaceValue(localizedCard, "printed_type_line") || firstFaceValue(localizedCard, "type_line") || "",
    oracleText: firstFaceValue(localizedCard, "printed_text") || "",
    scryfallId: localizedCard.id || null,
    set: localizedCard.set || null,
    collectorNumber: localizedCard.collector_number || null,
  };
}

async function atomicWriteJson(file, payload) {
  await mkdir(path.dirname(file), { recursive: true });
  const tmp = `${file}.tmp`;
  await writeFile(tmp, `${JSON.stringify(payload, null, 2)}\n`);
  await rename(tmp, file);
}

async function resolveBulkFile(args) {
  if (args.bulk) return path.resolve(ROOT, args.bulk);

  const bulkIndex = await requestJson(BULK_DATA_URL);
  const allCards = (bulkIndex.data || []).find((entry) => entry.type === "all_cards");
  if (!allCards?.download_uri) {
    throw new Error("Scryfall all_cards bulk download URI not found");
  }

  const outFile = path.resolve(ROOT, "reports", "i18n", `scryfall-all-cards-${Date.now()}.json`);
  console.log(`Downloading Scryfall all_cards bulk data to ${path.relative(ROOT, outFile)}...`);
  await downloadFile(allCards.download_uri, outFile);
  return outFile;
}

const args = parseArgs(process.argv.slice(2));
if (args.help) {
  console.log([
    "Usage: pnpm i18n:build-scryfall -- --locale es [--bulk path/to/all-cards.json]",
    "",
    "Builds official localized card text assets from Scryfall all_cards bulk data.",
    "Without --bulk, the script downloads the current all_cards bulk file from Scryfall.",
  ].join("\n"));
  process.exit(0);
}
const locale = String(args.locale || DEFAULT_LOCALE).trim().toLowerCase();
const bulkFile = await resolveBulkFile(args);
const cards = JSON.parse(await readFile(bulkFile, "utf8"));
if (!Array.isArray(cards)) {
  throw new Error("Expected Scryfall bulk file to be a JSON array");
}

const englishByOracle = new Map();
for (const card of cards) {
  if (card?.lang !== "en" || !card?.oracle_id) continue;
  if (!englishByOracle.has(card.oracle_id)) englishByOracle.set(card.oracle_id, card);
}

const byOracle = new Map();
for (const card of cards) {
  if (card?.lang !== locale || !card?.oracle_id) continue;
  const englishCard = englishByOracle.get(card.oracle_id);
  if (!englishCard) continue;
  const payload = translatedPayload(englishCard, card, locale);
  if (!payload.route || (!payload.name && !payload.typeLine && !payload.oracleText)) continue;
  const score = printingScore(englishCard, card);
  const releasedAt = String(card.released_at || "");
  const existing = byOracle.get(payload.oracleId);
  if (
    !existing
    || score > existing.score
    || (score === existing.score && releasedAt > existing.releasedAt)
  ) {
    byOracle.set(payload.oracleId, { payload, score, releasedAt });
  }
}

const outRoot = path.resolve(ROOT, args.outDir, locale);
let written = 0;
for (const { payload } of byOracle.values()) {
  await atomicWriteJson(path.join(outRoot, "by-oracle", `${payload.oracleId}.json`), payload);
  await atomicWriteJson(path.join(outRoot, "by-name", `${payload.route}.json`), payload);
  written += 1;
}

console.log(`Wrote ${written} Scryfall ${locale} card translation assets to ${path.relative(ROOT, outRoot)}`);
