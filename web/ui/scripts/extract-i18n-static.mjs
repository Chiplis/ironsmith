import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";

const ROOT = process.cwd();
const SRC_DIR = path.join(ROOT, "src");
const OUT_FILE = path.join(ROOT, "reports", "i18n", "static-strings.json");

const SKIP_DIRS = new Set(["node_modules", "dist", "test-results", "reports"]);
const SOURCE_EXTENSIONS = new Set([".js", ".jsx", ".ts", ".tsx"]);
const STRING_RE = /(["'`])((?:\\.|(?!\1)[\s\S])*?[A-Za-z][\s\S]*?)\1/g;
const MAX_SOURCE_BYTES = 500_000;

function isLikelyUiText(value) {
  const text = String(value || "").trim();
  if (text.length < 2 || text.length > 180) return false;
  if (/^(?:[_#.]|--)/.test(text)) return false;
  if (/^[./@#?&:=_{}()[\]\-+*|\\,0-9\s]+$/.test(text)) return false;
  if (/^(?:[a-z0-9_-]+:)+[a-z0-9_-]+$/i.test(text)) return false;
  if (/^(?:https?:|data:|var\(|rgba?\(|linear-gradient|radial-gradient)/i.test(text)) return false;
  if (/^(?:className|style|button|div|span|true|false|null|undefined)$/i.test(text)) return false;
  if (/[{}<>]/.test(text) && !/\{[A-Z0-9/]+\}/.test(text)) return false;
  return /[A-Za-z]{3,}/.test(text);
}

function decodeJsString(raw) {
  return raw
    .replace(/\\n/g, "\n")
    .replace(/\\r/g, "\r")
    .replace(/\\t/g, "\t")
    .replace(/\\"/g, "\"")
    .replace(/\\'/g, "'")
    .replace(/\\\\/g, "\\")
    .trim();
}

async function listSourceFiles(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    if (entry.isDirectory()) {
      if (!SKIP_DIRS.has(entry.name)) {
        files.push(...await listSourceFiles(path.join(dir, entry.name)));
      }
      continue;
    }
    if (entry.isFile() && SOURCE_EXTENSIONS.has(path.extname(entry.name))) {
      const file = path.join(dir, entry.name);
      if (entry.name.includes(".generated.")) continue;
      const info = await stat(file);
      if (info.size > MAX_SOURCE_BYTES) continue;
      files.push(file);
    }
  }
  return files;
}

const results = new Map();
for (const file of await listSourceFiles(SRC_DIR)) {
  const source = await readFile(file, "utf8");
  for (const match of source.matchAll(STRING_RE)) {
    const value = decodeJsString(match[2]);
    if (!isLikelyUiText(value)) continue;
    const line = source.slice(0, match.index).split(/\r?\n/).length;
    const rel = path.relative(ROOT, file).replace(/\\/g, "/");
    const current = results.get(value) || [];
    current.push({ file: rel, line });
    results.set(value, current);
  }
}

const payload = [...results.entries()]
  .sort(([left], [right]) => left.localeCompare(right))
  .map(([text, occurrences]) => ({ text, occurrences }));

await mkdir(path.dirname(OUT_FILE), { recursive: true });
await writeFile(OUT_FILE, `${JSON.stringify({
  generatedAt: new Date().toISOString(),
  count: payload.length,
  strings: payload,
}, null, 2)}\n`);

console.log(`Extracted ${payload.length} candidate UI strings to ${path.relative(ROOT, OUT_FILE)}`);
