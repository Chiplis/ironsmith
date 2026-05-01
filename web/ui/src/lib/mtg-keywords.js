import { MTG_KEYWORD_RULES } from "./mtg-keyword-rules.generated.js";

const WORD_BOUNDARY_CLASS = "A-Za-z0-9_";

function normalizeAlias(value) {
  return String(value || "")
    .replace(/[‘’]/g, "'")
    .replace(/[“”]/g, "\"")
    .replace(/[‐‑‒–—―]/g, "-")
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function aliasPattern(alias) {
  return normalizeAlias(alias)
    .split("")
    .map((char) => {
      if (/\s/.test(char)) return "\\s+";
      if (char === "'") return "['’]";
      if (char === "\"") return "[\"“”]";
      if (char === "-") return "[-‐‑‒–—―]";
      return escapeRegExp(char);
    })
    .join("");
}

const KEYWORD_ALIAS_ENTRIES = MTG_KEYWORD_RULES
  .flatMap((rule) => {
    const aliases = new Set([rule.title, ...(rule.aliases || [])]);
    return [...aliases]
      .map((alias) => normalizeAlias(alias))
      .filter((alias) => alias.length > 1 || alias === "∞")
      .map((alias) => ({ alias, rule }));
  })
  .sort((left, right) => right.alias.length - left.alias.length);

const KEYWORD_BY_ALIAS = new Map();
for (const entry of KEYWORD_ALIAS_ENTRIES) {
  if (!KEYWORD_BY_ALIAS.has(entry.alias)) {
    KEYWORD_BY_ALIAS.set(entry.alias, entry.rule);
  }
}

const KEYWORD_MATCH_RE = new RegExp(
  `(^|[^${WORD_BOUNDARY_CLASS}])(${KEYWORD_ALIAS_ENTRIES
    .map((entry) => aliasPattern(entry.alias))
    .join("|")})(?![${WORD_BOUNDARY_CLASS}])`,
  "giu"
);

export function getMtgKeywordRule(alias) {
  return KEYWORD_BY_ALIAS.get(normalizeAlias(alias)) || null;
}

function isCounterActionContext(source, keywordStart, keywordEnd, alias) {
  if (alias !== "counter") return true;

  const after = source.slice(keywordEnd);
  if (/^\s+(?:it|this|that|unless)\b/i.test(after)) return true;
  if (/^\s+(?:all|each|any|up to|the next)\b/i.test(after)) return true;
  if (/^\s+(?:target\s+)?(?:a\s+|an\s+|the\s+)?(?:[^.,;:(){}]{0,80}\s+)?(?:spell|ability)\b/i.test(after)) {
    return true;
  }

  const before = source.slice(0, keywordStart);
  if (/\b(?:can't|cannot|can not|can|would|will|is|are|be|been|was|were)\s+$/i.test(before)) {
    return true;
  }

  return false;
}

function shouldKeepKeywordMatch(source, keywordStart, keywordEnd, rule, matchedText) {
  if (rule?.rule === "701.6") {
    return isCounterActionContext(
      source,
      keywordStart,
      keywordEnd,
      normalizeAlias(matchedText),
    );
  }
  return true;
}

export function splitTextWithMtgKeywordRules(text) {
  const source = String(text || "");
  if (!source) return [];

  const segments = [];
  let lastIndex = 0;

  KEYWORD_MATCH_RE.lastIndex = 0;
  for (const match of source.matchAll(KEYWORD_MATCH_RE)) {
    const prefix = match[1] || "";
    const matchedText = match[2] || "";
    const keywordStart = Number(match.index || 0) + prefix.length;
    const keywordEnd = keywordStart + matchedText.length;
    const rule = getMtgKeywordRule(matchedText);
    if (!rule) continue;
    if (!shouldKeepKeywordMatch(source, keywordStart, keywordEnd, rule, matchedText)) continue;

    if (keywordStart > lastIndex) {
      segments.push({ type: "text", text: source.slice(lastIndex, keywordStart) });
    }
    segments.push({ type: "keyword", text: source.slice(keywordStart, keywordEnd), rule });
    lastIndex = keywordEnd;
  }

  if (lastIndex < source.length) {
    segments.push({ type: "text", text: source.slice(lastIndex) });
  }

  return segments;
}

export { MTG_KEYWORD_RULES };
