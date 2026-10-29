import {contextualKeywordReminder} from "./keyword-reminder-context.js";
import { activeCustomMechanics, customMechanicRevision } from "./custom-mechanics.js";
import { localeCatalogs } from "../i18n/catalog.js";
import { MTG_KEYWORD_RULES } from "./mtg-keyword-rules.generated.js";
import { officialKeywordForms, customKeywordForms, surfaceForms } from "./keyword-forms.js";

import { MTG_KEYWORD_REMINDERS } from "./mtg-keyword-reminders.generated.js";
import { reminderParameters, parseParameterDeclaration, bindReminderParameters } from "./reminder-costs.js";

const WORD_BOUNDARY_CLASS = "\\p{L}\\p{N}_";

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

const indexes = new Map();
function keywordIndex(locale) {
  const cacheKey = `${locale}:${customMechanicRevision()}`;
  if (indexes.has(cacheKey)) return indexes.get(cacheKey);
  indexes.clear();
  const localized = localeCatalogs[locale]?.rules || {};
  const customRules = activeCustomMechanics().map(def => ({ id: `custom:${def.keyword}`, title: def.keyword, aliases: surfaceForms(customKeywordForms(def)), summary: def.reminder, definition: def, custom: true }));
  const entries = [...customRules, ...MTG_KEYWORD_RULES].flatMap(rule => {
    const lexicon = rule.custom ? [] : [rule.title, ...(rule.aliases || [])].map(officialKeywordForms).filter(Boolean).flatMap(surfaceForms);
    const aliases = new Set([rule.title, ...(rule.aliases || []), ...lexicon, localized[rule.id]?.title, ...(localized[rule.id]?.aliases || [])]);
    return [...aliases].filter(Boolean).map(normalizeAlias)
      .filter(alias => alias.length > 1 || alias === "∞")
      .map(alias => ({ alias, rule }));
  }).sort((left, right) => right.alias.length - left.alias.length);
  const byAlias = new Map();
  for (const entry of entries) if (!byAlias.has(entry.alias)) byAlias.set(entry.alias, entry.rule);
  const pattern = new RegExp(
    `(^|[^${WORD_BOUNDARY_CLASS}])(${[...byAlias.keys()].map(aliasPattern).join("|")})(?![${WORD_BOUNDARY_CLASS}])`, "giu"
  );
  const index = { byAlias, pattern };
  indexes.set(cacheKey, index);
  return index;
}

export function getMtgKeywordRule(alias, locale = "en") {
  return keywordIndex(locale).byAlias.get(normalizeAlias(alias)) || null;
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

export function splitTextWithMtgKeywordRules(text, locale = "en") {
  const source = String(text || "");
  if (!source) return [];

  const segments = [];
  let lastIndex = 0;

  const { pattern } = keywordIndex(locale);
  pattern.lastIndex = 0;
  for (const match of source.matchAll(pattern)) {
    const prefix = match[1] || "";
    const matchedText = match[2] || "";
    const keywordStart = Number(match.index || 0) + prefix.length;
    const keywordEnd = keywordStart + matchedText.length;
    const rule = getMtgKeywordRule(matchedText, locale);
    if (!rule) continue;
    if (!shouldKeepKeywordMatch(source, keywordStart, keywordEnd, rule, matchedText)) continue;

    if (keywordStart > lastIndex) {
      segments.push({ type: "text", text: source.slice(lastIndex, keywordStart) });
    }
    segments.push({ type: "keyword", text: source.slice(keywordStart, keywordEnd), context: source.slice(keywordEnd), rule });
    lastIndex = keywordEnd;
  }

  if (lastIndex < source.length) {
    segments.push({ type: "text", text: source.slice(lastIndex) });
  }

  return segments;
}

export { MTG_KEYWORD_RULES };

// A parameterized definition reads with the values this card declares
// ("Forge 4—{B}{B}"), not its "{Number}", "{Cost}" and "{Type}" placeholders.
function customKeywordDescription(alias, rule, context) {
  const reminder = rule.summary;
  if (!reminderParameters(reminder).length || !rule.definition) return reminder;
  const declaration = `${alias}${String(context || "").split("\n")[0].replace(/\s*\(.*$/, "")}`;
  const values = parseParameterDeclaration(reminder, alias, declaration, rule.definition.template);
  return values ? bindReminderParameters(reminder, alias, values, customKeywordForms(rule.definition)) : reminder;
}

// Reminder text is English source material; other locales retain their localized
// CR summaries until a translated reminder catalog is available. Match the actual
// keyword, not the shared CR id (cycling, landcycling, etc. differ).
export function getMtgKeywordHelp(alias, rule, locale = "en", context = "") {
  if (rule?.custom) return { title: rule.title, description: customKeywordDescription(alias, rule, context), reminder: null };
  const localizedRule = localeCatalogs[locale]?.rules?.[rule?.id] || rule;
  const key = normalizeAlias(alias);
  const reminder = locale === "en"
    ? contextualKeywordReminder(key, alias, context, MTG_KEYWORD_REMINDERS[key], localizedRule?.summary || '')
    : null;
  return {
    title: localizedRule?.title,
    description: reminder?.text || localizedRule?.summary || "",
    reminder: reminder || null,
  };
}
