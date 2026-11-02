import fs from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const RULES_PAGE = "https://magic.wizards.com/en/rules";

export function discoverRulesUrl(html) {
  const urls = [...html.replace(/&amp;/g, "&").matchAll(/https:\/\/media\.wizards\.com\/[^\r\n"'<>]+?\.txt/gi)]
    .map(match => new URL(match[0]).href).filter(url => /MagicCompRules/i.test(url));
  const unique = [...new Set(urls)];
  if (unique.length !== 1) throw new Error(`Expected one official Comprehensive Rules TXT link; found ${unique.length}.`);
  return unique[0];
}

async function fetchText(url) {
  const response = await fetch(url, { signal: AbortSignal.timeout(30000) });
  if (!response.ok) throw new Error(`Failed to fetch ${url}: HTTP ${response.status}`);
  return response.text();
}
const DEFAULT_OUTPUT = new URL("../src/lib/mtg-keyword-rules.generated.js", import.meta.url);

const MANUAL_SUMMARIES = {
  "Open an Attraction": "Reveal cards from your Attraction deck until you reveal an Attraction, then put that Attraction onto the battlefield.",
  "Roll to Visit Your Attractions": "Roll a six-sided die and visit each Attraction whose lit number matches the result.",
  "Suspect": "A suspected creature has menace and can't block. It stops being suspected if it leaves the battlefield or stops being a creature.",
};

const MANUAL_ALIASES = {
  "Behold": ["beheld"],
  "Counter": ["countered", "countering"],
  "Destroy": ["destroyed"],
  "Discard": ["discarded"],
  "Exile": ["exiled"],
  "Landwalk": [
    "artifact landwalk",
    "forestwalk",
    "islandwalk",
    "mountainwalk",
    "nonbasic landwalk",
    "plainswalk",
    "snow landwalk",
    "swampwalk",
  ],
  "Cycling": [
    "basic landcycling",
    "forestcycling",
    "islandcycling",
    "mountaincycling",
    "plainscycling",
    "swampcycling",
    "typecycling",
    "wizardcycling",
  ],
  "Goad": ["goaded"],
  "Tap and Untap": ["tapped", "untapped"],
  "Monstrosity": ["monstrous"],
  "Manifest": ["manifested"],
  "Exert": ["exerted"],
  "Explore": ["explored"],
  "Amass": ["amassed"],
  "Connive": ["connived"],
  "The Ring Tempts You": ["ring-bearer"],
  "Cloak": ["cloaked"],
  "Suspect": ["suspected"],
  "Harness": ["harnessed"],
  "Phasing": ["phased in", "phased out", "phased-in", "phased-out"],
  "Kicker": ["kicked", "multikicker", "sticker kicker"],
  "Partner": [
    "choose a background",
    "doctor's companion",
    "friends forever",
    "partner with",
    "partner-",
  ],
  "Champion": ["championed"],
  "Soulbond": ["paired"],
  "Cipher": ["encoded"],
  "Fuse": ["fused", "fused split spell"],
  "Renown": ["renowned"],
  "Daybound and Nightbound": ["daybound", "nightbound"],
  "Ascend": ["city's blessing"],
  "Escape": ["escaped"],
  "Plot": ["plotted"],
  "Saddle": ["saddled"],
  "Gift": ["gift was promised"],
  "Warp": ["warped"],
  "\u221e (Infinity)": ["infinity", "∞"],
};

function argValue(name) {
  const index = process.argv.indexOf(name);
  if (index < 0) return null;
  return process.argv[index + 1] || null;
}

function normalizeText(value) {
  return String(value || "")
    .replace(/^\uFEFF/, "")
    .replace(/\r/g, "")
    .replace(/[“”]/g, "\"")
    .replace(/[‘’]/g, "'")
    .replace(/[–—]/g, "-")
    .replace(/\s+/g, " ")
    .trim();
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function slugify(value) {
  return normalizeText(value)
    .toLowerCase()
    .replace(/∞/g, "infinity")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function titleAliases(title) {
  const normalized = normalizeText(title);
  const withoutParenthetical = normalizeText(normalized.replace(/\s*\([^)]*\)/g, ""));
  const aliases = new Set([normalized, withoutParenthetical]);

  if (withoutParenthetical.includes("-")) {
    aliases.add(withoutParenthetical.replace(/-/g, " "));
  }
  if (withoutParenthetical.endsWith("!")) {
    aliases.add(withoutParenthetical.slice(0, -1));
  }
  if (/^∞/.test(normalized)) {
    aliases.add("∞");
    aliases.add("Infinity");
  }

  const andMatch = withoutParenthetical.match(/^(.+?)\s+and\s+(.+)$/i);
  if (andMatch) {
    aliases.add(andMatch[1]);
    aliases.add(andMatch[2]);
  }

  if (/^tap and untap$/i.test(withoutParenthetical)) {
    aliases.add("Tap");
    aliases.add("Untap");
  }
  if (/^venture into the dungeon$/i.test(withoutParenthetical)) {
    aliases.add("Venture into");
  }

  return [...aliases].map(normalizeText).filter(Boolean);
}

function aliasesFromGlossaryName(name) {
  const normalized = normalizeText(name);
  return normalized
    .split(/\s*,\s*/)
    .flatMap((part) => {
      const cleaned = normalizeText(part.replace(/"/g, ""));
      const aliases = [cleaned];
      if (cleaned.includes("[")) {
        aliases.push(normalizeText(cleaned.replace(/\s*\[[^\]]+\]/g, "").replace(/-$/g, "")));
      }
      if (cleaned.includes("-")) {
        aliases.push(cleaned.replace(/-/g, " "));
      }
      return aliases;
    })
    .map(normalizeText)
    .filter(Boolean);
}

function sentenceSummary(definition) {
  const stripped = normalizeText(definition)
    .replace(/\s+See rules?.*$/i, "")
    .replace(/\s+For more information.*$/i, "");
  if (!stripped) return "";
  if (stripped.length <= 260) return stripped;

  const sentences = stripped.match(/[^.!?]+[.!?]+/g) || [];
  let summary = "";
  for (const sentence of sentences) {
    const next = normalizeText(`${summary} ${sentence}`);
    if (next.length > 260) break;
    summary = next;
  }
  if (summary) return summary;

  const words = stripped.split(/\s+/);
  let truncated = "";
  for (const word of words) {
    const next = truncated ? `${truncated} ${word}` : word;
    if (next.length > 240) break;
    truncated = next;
  }
  return `${truncated}...`;
}

function referencesRule(definition, rule) {
  return new RegExp(`\\brule\\s+${escapeRegExp(rule)}\\b`, "i").test(definition);
}

function isKeywordishGlossaryEntry(entry) {
  return /\b(keyword ability|keyword action|variant of|partner ability|cycling ability|landwalk|kicker variant)\b/i
    .test(entry.definition);
}

function parseGlossary(lines, startIndex) {
  const glossaryStart = lines.findIndex((line, index) => index > startIndex && /^Glossary$/.test(line));
  const creditsStart = lines.findIndex((line, index) => index > glossaryStart && /^Credits$/.test(line));
  if (glossaryStart < 0 || creditsStart < 0) {
    throw new Error("Could not find glossary section in rules text.");
  }

  const blocks = lines
    .slice(glossaryStart + 1, creditsStart)
    .join("\n")
    .split(/\n{2,}/)
    .map((block) => block.trim())
    .filter(Boolean);

  return blocks
    .map((block) => {
      const [rawName, ...definitionLines] = block.split("\n");
      const name = normalizeText(rawName);
      const definition = normalizeText(definitionLines.join(" "));
      return name && definition ? { name, definition, aliases: aliasesFromGlossaryName(name) } : null;
    })
    .filter(Boolean);
}

function parseKeywordRules(lines) {
  const sectionStart = lines.findIndex((line, index) => index > 1000 && /^701\. Keyword Actions$/.test(line));
  const sectionEnd = lines.findIndex((line, index) => index > sectionStart && /^703\. Turn-Based Actions$/.test(line));
  if (sectionStart < 0 || sectionEnd < 0) {
    throw new Error("Could not find keyword rules sections 701-702.");
  }

  const entries = [];
  for (let index = sectionStart; index < sectionEnd; index += 1) {
    const match = lines[index].match(/^(70[12]\.\d+)\.\s+(.+)$/);
    if (!match || match[1].endsWith(".1")) continue;
    entries.push({
      rule: match[1],
      title: normalizeText(match[2]),
      kind: match[1].startsWith("701") ? "keywordAction" : "keywordAbility",
    });
  }
  return { sectionStart, entries };
}

function ruleFallbackSummary(lines, rule) {
  const headingIndex = lines.findIndex((line) => line.startsWith(`${rule}. `));
  if (headingIndex < 0) return "";
  const body = [];
  const subrulePattern = new RegExp(`^${escapeRegExp(rule)}[a-z]\\s+(.+)$`);
  for (let index = headingIndex + 1; index < lines.length; index += 1) {
    if (/^70[123]\.\d+\.\s+/.test(lines[index])) break;
    const match = lines[index].match(subrulePattern);
    if (match) body.push(match[1]);
    if (body.length >= 2) break;
  }
  return sentenceSummary(body.join(" "));
}

function findSummary(entry, glossary, lines) {
  if (MANUAL_SUMMARIES[entry.title]) return MANUAL_SUMMARIES[entry.title];

  const aliases = titleAliases(entry.title).map((alias) => alias.toLowerCase());
  const exact = glossary.find((item) =>
    item.aliases.some((alias) => aliases.includes(alias.toLowerCase()))
  );
  if (exact) return sentenceSummary(exact.definition);

  if (/^tap and untap$/i.test(entry.title)) {
    const tap = glossary.find((item) => item.name.toLowerCase() === "tap");
    const untap = glossary.find((item) => item.name.toLowerCase() === "untap");
    return sentenceSummary(`${tap?.definition || ""} ${untap?.definition || ""}`);
  }

  if (/∞/.test(entry.title)) {
    const infinity = glossary.find((item) => item.name.toLowerCase() === "infinity");
    if (infinity) return sentenceSummary(infinity.definition);
  }

  const related = glossary.find((item) =>
    referencesRule(item.definition, entry.rule) && isKeywordishGlossaryEntry(item)
  );
  if (related) return sentenceSummary(related.definition);

  return ruleFallbackSummary(lines, entry.rule);
}

function relatedAliases(entry, glossary) {
  const aliases = new Set(titleAliases(entry.title));
  for (const alias of MANUAL_ALIASES[entry.title] || []) aliases.add(alias);

  for (const item of glossary) {
    if (!referencesRule(item.definition, entry.rule)) continue;
    if (!isKeywordishGlossaryEntry(item)) continue;
    for (const alias of item.aliases) aliases.add(alias);
  }

  return [...aliases]
    .map((alias) => normalizeText(alias).toLowerCase())
    .filter((alias) => !alias.includes("[") && !alias.includes("]"))
    .filter((alias) => alias.length > 1 || alias === "∞")
    .filter((alias, index, all) => all.indexOf(alias) === index)
    .sort((left, right) => left.localeCompare(right));
}

async function readRulesText() {
  const input = argValue("--input");
  if (input) return { rawText: await fs.readFile(input, "utf8"), sourceUrl: argValue("--url") || pathToFileURL(input).href };
  const sourceUrl = argValue("--url") || discoverRulesUrl(await fetchText(RULES_PAGE));
  return { rawText: await fetchText(sourceUrl), sourceUrl };
}

async function main() {
  const { rawText, sourceUrl } = await readRulesText();
  const text = rawText.replace(/^\uFEFF/, "").replace(/\r/g, "");
  const lines = text.split("\n").map(line => line.trim());
  const effectiveDate = text.match(/These rules are effective as of ([^.]+)\./)?.[1] || "unknown";
  const { sectionStart, entries } = parseKeywordRules(lines);
  const glossary = parseGlossary(lines, sectionStart);
  if (effectiveDate === "unknown" || entries.filter(entry => entry.kind === "keywordAction").length < 50 || entries.filter(entry => entry.kind === "keywordAbility").length < 150) {
    throw new Error("Incomplete Comprehensive Rules: refusing to replace the official keyword registry.");
  }

  const generated = entries.map((entry) => ({
    id: `${entry.kind === "keywordAction" ? "action" : "ability"}-${entry.rule.replace(/\./g, "-")}-${slugify(entry.title)}`,
    kind: entry.kind,
    rule: entry.rule,
    title: normalizeText(entry.title),
    aliases: relatedAliases(entry, glossary),
    summary: findSummary(entry, glossary, lines),
  }));

  const missingSummary = generated.filter((entry) => !entry.summary);
  if (missingSummary.length > 0) {
    throw new Error(`Missing summaries for ${missingSummary.map((entry) => entry.rule).join(", ")}`);
  }

  const output = [
    "// Generated by web/ui/scripts/generate-mtg-keyword-rules.mjs.",
    "// Source: Magic: The Gathering Comprehensive Rules.",
    "",
    "export const MTG_KEYWORD_RULESET_SOURCE = Object.freeze(",
    JSON.stringify({
      title: "Magic: The Gathering Comprehensive Rules",
      effectiveDate,
      url: sourceUrl,
    }, null, 2),
    ");",
    "",
    "export const MTG_KEYWORD_RULES = Object.freeze(",
    JSON.stringify(generated, null, 2),
    ");",
    "",
  ].join("\n");

  const outputArg = argValue("--output");
  const outputPath = outputArg ? pathToFileURL(outputArg) : DEFAULT_OUTPUT;
  const temporary = new URL(`${outputPath.href}.${process.pid}.tmp`);
  try {
    await fs.writeFile(temporary, output, "utf8");
    await fs.rename(temporary, outputPath);
  } finally { await fs.rm(temporary, { force: true }); }
  console.log(`Wrote ${generated.length} keyword summaries to ${outputPath.pathname}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
