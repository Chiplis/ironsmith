import { createReadStream, createWriteStream } from "node:fs";
import { mkdir, readFile, readdir, rename, rm, unlink, writeFile } from "node:fs/promises";
import https from "node:https";
import path from "node:path";
import readline from "node:readline";
import { pipeline } from "node:stream/promises";

const ROOT = process.cwd();
const DEFAULT_LOCALE = "es";
const BULK_DATA_URL = "https://api.scryfall.com/bulk-data";
const SCHEMA_VERSION = 3;
const USER_AGENT = "Ironsmith i18n asset builder (local development)";

function parseArgs(argv) {
  const args = {
    locale: DEFAULT_LOCALE,
    bulk: "",
    keepBulk: false,
    ifChanged: false,
    outDir: path.join("public", "card-i18n"),
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--") continue;
    if (arg === "--locale" || arg === "-l") {
      args.locale = argv[++index] || DEFAULT_LOCALE;
    } else if (arg === "--bulk") {
      args.bulk = argv[++index] || "";
    } else if (arg === "--out-dir") {
      args.outDir = argv[++index] || args.outDir;
    } else if (arg === "--keep-bulk") {
      args.keepBulk = true;
    } else if (arg === "--if-changed") {
      args.ifChanged = true;
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
        "User-Agent": USER_AGENT,
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
  const tmpFile = `${outFile}.download`;
  await new Promise((resolve, reject) => {
    https.get(url, {
      headers: {
        Accept: "application/json;q=0.9,*/*;q=0.8",
        "User-Agent": USER_AGENT,
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
        const totalBytes = Number(response.headers["content-length"]) || 0;
        let received = 0;
        let lastLogged = 0;
        response.on("data", (chunk) => {
          received += chunk.length;
          if (received - lastLogged >= 250 * 1024 * 1024) {
            lastLogged = received;
            const progress = totalBytes ? ` of ${(totalBytes / 1e9).toFixed(2)} GB` : "";
            console.log(`  downloaded ${(received / 1e9).toFixed(2)} GB${progress}...`);
          }
        });
        await pipeline(response, createWriteStream(tmpFile));
        await rename(tmpFile, outFile);
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

function normalizeText(text) {
  return String(text || "").replace(/\s+/g, " ").trim();
}

// Scryfall marks not-yet-localized printings as lang:<locale> with a localized
// printed_name but printed_text still in English (e.g. brand-new Commander
// reprints). Treat such text as absent so a properly translated older printing
// wins instead.
function localizedPrintedText(english, localizedCard) {
  const printedText = firstFaceValue(localizedCard, "printed_text");
  if (!printedText || normalizeText(printedText) === english.textNorm) return "";
  return printedText;
}

function translatedPayload(english, localizedCard, locale, printedText) {
  return {
    schemaVersion: SCHEMA_VERSION,
    source: "scryfall",
    sourceLang: "en",
    targetLang: locale,
    oracleId: english.oracleId,
    englishName: english.name,
    route: english.route,
    // Only printed_* fields are localized; name/type_line/oracle_text on a
    // localized card object are English. Consumers fall back to their own
    // English text when a field is empty.
    name: firstFaceValue(localizedCard, "printed_name") || "",
    typeLine: firstFaceValue(localizedCard, "printed_type_line") || "",
    oracleText: printedText,
    scryfallId: localizedCard.id || null,
    set: localizedCard.set || null,
    collectorNumber: localizedCard.collector_number || null,
  };
}

// Scryfall bulk files are a JSON array with one card object per line; parsing
// line-by-line keeps memory flat (the whole file exceeds Node's string limit).
async function* streamBulkCards(bulkFile) {
  const lines = readline.createInterface({
    input: createReadStream(bulkFile),
    crlfDelay: Infinity,
  });
  for await (const rawLine of lines) {
    let line = rawLine.trim();
    if (!line || line === "[" || line === "]") continue;
    if (line.endsWith(",")) line = line.slice(0, -1);
    if (!line.startsWith("{")) continue;
    try {
      yield JSON.parse(line);
    } catch {
      throw new Error(
        "Failed to parse a Scryfall bulk data line as JSON — the one-card-per-line bulk format may have changed"
      );
    }
  }
}

function bucketKey(key) {
  return String(key || "").slice(0, 2) || "_";
}

async function writeBuckets(dir, entries) {
  const buckets = new Map();
  for (const [key, payload] of entries) {
    const bucket = bucketKey(key);
    if (!buckets.has(bucket)) buckets.set(bucket, {});
    buckets.get(bucket)[key] = payload;
  }
  await mkdir(dir, { recursive: true });
  for (const [bucket, contents] of buckets) {
    await writeFile(path.join(dir, `${bucket}.json`), JSON.stringify(contents));
  }
  return buckets.size;
}

async function readManifest(file) {
  try {
    const manifest = JSON.parse(await readFile(file, "utf8"));
    return manifest && typeof manifest === "object" ? manifest : null;
  } catch {
    return null;
  }
}

async function hasBuiltAssets(outRoot) {
  try {
    const files = await readdir(path.join(outRoot, "by-name"));
    return files.some((file) => file.endsWith(".json"));
  } catch {
    return false;
  }
}

async function fetchAllCardsBulkInfo() {
  const bulkIndex = await requestJson(BULK_DATA_URL);
  const allCards = (bulkIndex.data || []).find((entry) => entry.type === "all_cards");
  if (!allCards?.download_uri) {
    throw new Error("Scryfall all_cards bulk download URI not found");
  }
  return allCards;
}

const args = parseArgs(process.argv.slice(2));
if (args.help) {
  console.log([
    "Usage: pnpm i18n:build-scryfall -- --locale es [options]",
    "",
    "Builds official localized card text assets from Scryfall all_cards bulk data",
    "into sharded bucket files under public/card-i18n/<locale>/.",
    "",
    "Options:",
    "  --locale, -l <code>   Target language (default: es)",
    "  --bulk <path>         Use a local all_cards bulk file instead of downloading",
    "  --if-changed          Skip the build when Scryfall's bulk updated_at matches",
    "                        the manifest of the existing assets; network failures",
    "                        are non-fatal so builds keep working offline",
    "  --keep-bulk           Keep the downloaded bulk file (several GB) afterwards",
    "  --out-dir <dir>       Output root (default: public/card-i18n)",
  ].join("\n"));
  process.exit(0);
}

const locale = String(args.locale || DEFAULT_LOCALE).trim().toLowerCase();
const outRoot = path.resolve(ROOT, args.outDir, locale);
const manifestFile = path.join(outRoot, "manifest.json");

let bulkInfo = null;
if (!args.bulk) {
  try {
    bulkInfo = await fetchAllCardsBulkInfo();
  } catch (error) {
    if (args.ifChanged) {
      console.warn(`Skipping Scryfall i18n build (bulk index unavailable: ${error.message})`);
      process.exit(0);
    }
    throw error;
  }
}

if (args.ifChanged && bulkInfo) {
  const manifest = await readManifest(manifestFile);
  if (
    manifest?.schemaVersion === SCHEMA_VERSION
    && manifest?.bulkUpdatedAt === bulkInfo.updated_at
    && await hasBuiltAssets(outRoot)
  ) {
    console.log(`Scryfall ${locale} card translations are up to date (bulk ${bulkInfo.updated_at})`);
    process.exit(0);
  }
}

let bulkFile;
let downloadedBulk = false;
if (args.bulk) {
  bulkFile = path.resolve(ROOT, args.bulk);
} else {
  bulkFile = path.resolve(ROOT, "reports", "i18n", "scryfall-all-cards.json");
  console.log(
    `Downloading Scryfall all_cards bulk data (${(bulkInfo.size / 1e9).toFixed(2)} GB) to ${path.relative(ROOT, bulkFile)}...`
  );
  await downloadFile(bulkInfo.download_uri, bulkFile);
  downloadedBulk = true;
}

console.log("Pass 1/2: indexing English cards...");
const englishByOracle = new Map();
for await (const card of streamBulkCards(bulkFile)) {
  if (card?.lang !== "en" || !card?.oracle_id) continue;
  if (englishByOracle.has(card.oracle_id)) continue;
  const name = firstFaceValue(card, "name");
  const textNorm = normalizeText(firstFaceValue(card, "oracle_text"));
  englishByOracle.set(card.oracle_id, {
    oracleId: card.oracle_id,
    name,
    route: routeKey(name),
    textNorm,
    parens: parenGroupCount(textNorm),
  });
}
console.log(`  indexed ${englishByOracle.size} English cards`);

console.log(`Pass 2/2: selecting best ${locale} printing per card...`);
const byOracle = new Map();
for await (const card of streamBulkCards(bulkFile)) {
  if (card?.lang !== locale || !card?.oracle_id) continue;
  const english = englishByOracle.get(card.oracle_id);
  if (!english || !english.route) continue;
  // Rank printings: 2 = localized text keeping at least as many parenthetical
  // (reminder) groups as the English oracle text, 1 = any localized text,
  // 0 = name/typeLine only. Newest printing wins within each tier.
  const printedText = localizedPrintedText(english, card);
  const payload = translatedPayload(english, card, locale, printedText);
  if (!payload.name && !payload.typeLine && !payload.oracleText) continue;
  const score = !printedText ? 0 : (parenGroupCount(printedText) >= english.parens ? 2 : 1);
  const releasedAt = String(card.released_at || "");
  const existing = byOracle.get(card.oracle_id);
  if (
    !existing
    || score > existing.score
    || (score === existing.score && releasedAt > existing.releasedAt)
  ) {
    byOracle.set(card.oracle_id, { payload, score, releasedAt });
  }
}

await rm(path.join(outRoot, "by-oracle"), { recursive: true, force: true });
await rm(path.join(outRoot, "by-name"), { recursive: true, force: true });
const oracleEntries = [];
const nameEntries = [];
for (const { payload } of byOracle.values()) {
  oracleEntries.push([payload.oracleId, payload]);
  nameEntries.push([payload.route, payload]);
}
const oracleBuckets = await writeBuckets(path.join(outRoot, "by-oracle"), oracleEntries);
const nameBuckets = await writeBuckets(path.join(outRoot, "by-name"), nameEntries);

await writeFile(manifestFile, `${JSON.stringify({
  schemaVersion: SCHEMA_VERSION,
  locale,
  bulkUpdatedAt: bulkInfo?.updated_at || null,
  builtAt: new Date().toISOString(),
  cardCount: byOracle.size,
}, null, 2)}\n`);

if (downloadedBulk && !args.keepBulk) {
  await unlink(bulkFile).catch(() => null);
}

console.log(
  `Wrote ${byOracle.size} Scryfall ${locale} card translations into ${oracleBuckets + nameBuckets} bucket files under ${path.relative(ROOT, outRoot)}`
);
