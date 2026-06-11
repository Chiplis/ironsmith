import { createHash } from "node:crypto";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";

const ROOT = process.cwd();
const DEFAULT_LOCALE = "es";
const TRANSLATABLE_FIELDS = new Set([
  "name",
  "printed_name",
  "type_line",
  "printed_type_line",
  "oracle_text",
  "printed_text",
  "effect_text",
  "ability_text",
  "keyword_text",
  "description",
  "context_text",
  "source_name",
]);

function parseArgs(argv) {
  const args = { input: [], locale: DEFAULT_LOCALE };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--") {
      continue;
    }
    if (arg === "--input" || arg === "-i") {
      args.input.push(argv[++index]);
    } else if (arg === "--locale" || arg === "-l") {
      args.locale = argv[++index] || DEFAULT_LOCALE;
    } else if (arg === "--out") {
      args.out = argv[++index];
    } else if (arg) {
      args.input.push(arg);
    }
  }
  return args;
}

function normalizeGeneratedEnglishText(text) {
  return String(text || "")
    .replace(/\r\n?/g, "\n")
    .split("\n")
    .map((line) => line.trim().replace(/\s+/g, " "))
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

function hashText(text) {
  return createHash("sha256").update(normalizeGeneratedEnglishText(text), "utf8").digest("hex");
}

function collectStrings(value, source, out) {
  if (value == null) return;
  if (Array.isArray(value)) {
    value.forEach((item, index) => collectStrings(item, `${source}[${index}]`, out));
    return;
  }
  if (typeof value !== "object") return;

  for (const [key, child] of Object.entries(value)) {
    const childSource = source ? `${source}.${key}` : key;
    if (typeof child === "string" && TRANSLATABLE_FIELDS.has(key)) {
      const normalized = normalizeGeneratedEnglishText(child);
      if (normalized) out.push({ field: key, source: childSource, text: normalized });
    } else if (Array.isArray(child) && (key === "compiled_text" || key === "abilities")) {
      const normalized = normalizeGeneratedEnglishText(child.filter(Boolean).join("\n"));
      if (normalized) out.push({ field: key, source: childSource, text: normalized });
    } else {
      collectStrings(child, childSource, out);
    }
  }
}

async function readInputRecords(file) {
  const raw = await readFile(file, "utf8");
  const trimmed = raw.trim();
  if (!trimmed) return [];
  if (trimmed.startsWith("[") || trimmed.startsWith("{")) {
    const parsed = JSON.parse(trimmed);
    return Array.isArray(parsed) ? parsed : [parsed];
  }
  return trimmed
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

const args = parseArgs(process.argv.slice(2));
if (args.input.length === 0) {
  console.error("Usage: pnpm i18n:collect-generated -- --input path/to/cards.jsonl --locale es");
  process.exit(1);
}

const pendingByHash = new Map();
for (const input of args.input) {
  const absInput = path.resolve(ROOT, input);
  const records = await readInputRecords(absInput);
  records.forEach((record, recordIndex) => {
    const strings = [];
    collectStrings(record, `$[${recordIndex}]`, strings);
    strings.forEach((entry) => {
      const sourceHash = hashText(entry.text);
      if (!pendingByHash.has(sourceHash)) {
        pendingByHash.set(sourceHash, {
          schemaVersion: 1,
          sourceLang: "en",
          targetLang: args.locale,
          sourceHash,
          sourceText: entry.text,
          translatedText: "",
          engine: "pending",
          sources: [],
        });
      }
      pendingByHash.get(sourceHash).sources.push({
        input: path.relative(ROOT, absInput).replace(/\\/g, "/"),
        field: entry.field,
        path: entry.source,
      });
    });
  });
}

const outFile = path.resolve(
  ROOT,
  args.out || path.join("reports", "i18n", `generated-${args.locale}-pending.jsonl`)
);
await mkdir(path.dirname(outFile), { recursive: true });
const lines = [...pendingByHash.values()]
  .sort((left, right) => left.sourceHash.localeCompare(right.sourceHash))
  .map((entry) => JSON.stringify(entry));
const tmpFile = `${outFile}.tmp`;
await writeFile(tmpFile, `${lines.join("\n")}${lines.length ? "\n" : ""}`);
await rename(tmpFile, outFile);

console.log(`Collected ${lines.length} generated text entries in ${path.relative(ROOT, outFile)}`);
