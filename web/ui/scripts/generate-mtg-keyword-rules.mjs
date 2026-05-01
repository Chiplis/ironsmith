import fs from "node:fs/promises";

const DEFAULT_RULES_URL = "https://media.wizards.com/2026/downloads/MagicCompRules%2020260417.txt";
const DEFAULT_OUTPUT = new URL("../src/lib/mtg-keyword-rules.generated.js", import.meta.url);

const MANUAL_SUMMARIES = {
  "701.51": "Reveal cards from your Attraction deck until you reveal an Attraction, then put that Attraction onto the battlefield.",
  "701.52": "Roll a six-sided die and visit each Attraction whose lit number matches the result.",
  "701.60": "A suspected creature has menace and can't block. It stops being suspected if it leaves the battlefield or stops being a creature.",
};

const MANUAL_ALIASES = {
  "701.4": ["beheld"],
  "701.6": ["countered", "countering"],
  "701.8": ["destroyed"],
  "701.9": ["discarded"],
  "701.13": ["exiled"],
  "702.14": [
    "artifact landwalk",
    "forestwalk",
    "islandwalk",
    "mountainwalk",
    "nonbasic landwalk",
    "plainswalk",
    "snow landwalk",
    "swampwalk",
  ],
  "702.29": [
    "basic landcycling",
    "forestcycling",
    "islandcycling",
    "mountaincycling",
    "plainscycling",
    "swampcycling",
    "typecycling",
    "wizardcycling",
  ],
  "701.15": ["goaded"],
  "701.26": ["tapped", "untapped"],
  "701.37": ["monstrous"],
  "701.40": ["manifested"],
  "701.43": ["exerted"],
  "701.44": ["explored"],
  "701.47": ["amassed"],
  "701.50": ["connived"],
  "701.54": ["ring-bearer"],
  "701.58": ["cloaked"],
  "701.60": ["suspected"],
  "701.64": ["harnessed"],
  "702.26": ["phased in", "phased out", "phased-in", "phased-out"],
  "702.33": ["kicked", "multikicker", "sticker kicker"],
  "702.124": [
    "choose a background",
    "doctor's companion",
    "friends forever",
    "partner with",
    "partner-",
  ],
  "702.72": ["championed"],
  "702.95": ["paired"],
  "702.99": ["encoded"],
  "702.102": ["fused", "fused split spell"],
  "702.112": ["renowned"],
  "702.145": ["daybound", "nightbound"],
  "702.131": ["city's blessing"],
  "702.138": ["escaped"],
  "702.170": ["plotted"],
  "702.171": ["saddled"],
  "702.174": ["gift was promised"],
  "702.185": ["warped"],
  "702.186": ["infinity", "∞"],
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
  if (MANUAL_SUMMARIES[entry.rule]) return MANUAL_SUMMARIES[entry.rule];

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
  for (const alias of MANUAL_ALIASES[entry.rule] || []) aliases.add(alias);

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
  if (input) return fs.readFile(input, "utf8");

  const url = argValue("--url") || DEFAULT_RULES_URL;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to fetch ${url}: HTTP ${response.status}`);
  }
  return response.text();
}

async function main() {
  const rawText = await readRulesText();
  const text = rawText.replace(/^\uFEFF/, "").replace(/\r/g, "");
  const lines = text.split("\n");
  const effectiveDate = text.match(/These rules are effective as of ([^.]+)\./)?.[1] || "unknown";
  const { sectionStart, entries } = parseKeywordRules(lines);
  const glossary = parseGlossary(lines, sectionStart);

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
      url: DEFAULT_RULES_URL,
    }, null, 2),
    ");",
    "",
    "export const MTG_KEYWORD_RULES = Object.freeze(",
    JSON.stringify(generated, null, 2),
    ");",
    "",
  ].join("\n");

  const outputArg = argValue("--output");
  const outputPath = outputArg ? new URL(`file://${outputArg.startsWith("/") ? outputArg : `${process.cwd()}/${outputArg}`}`) : DEFAULT_OUTPUT;
  await fs.writeFile(outputPath, output, "utf8");
  console.log(`Wrote ${generated.length} keyword summaries to ${outputPath.pathname}`);
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
