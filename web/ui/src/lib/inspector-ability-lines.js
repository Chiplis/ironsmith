import { splitTextWithMtgKeywordRules, getMtgKeywordHelp, getMtgKeywordCatalogReminder } from "./mtg-keywords.js";

export function stripInspectorAbilityPrefixes(text = "") {
  const prefixPatterns = [
    /^\s*(?:Triggered|Activated|Mana|Static)\s+ability(?:\s+\d+)?\s*:\s*/i,
    /^\s*Spell\s+effects?\s*:\s*/i,
    /^\s*Keyword\s+ability(?:\s+\{[^}]+\})*\s*:\s*/i,
  ];

  return String(text)
    .split("\n")
    .map((line) => {
      let cleaned = String(line || "");
      for (const pattern of prefixPatterns) {
        cleaned = cleaned.replace(pattern, "");
      }
      return cleaned;
    })
    .join("\n");
}

export function normalizeAbilityMatchText(text = "") {
  return stripInspectorAbilityPrefixes(text)
    .toLowerCase()
    // Mana symbols distinguish otherwise identical abilities (e.g. dual lands).
    .replace(/\{([^}]+)\}/g, " $1 ")
    .replace(/[^a-z0-9\s]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function lineAbilityMatchScore(lineText, needleText) {
  const line = normalizeAbilityMatchText(lineText);
  const needle = normalizeAbilityMatchText(needleText);
  if (!line || !needle) return 0;
  // A discounted Equip action must not bind to "Equipped creature ..." just
  // because both strings contain "equip". The printed cost can differ.
  if (/^equip(?: |$)/.test(line) !== /^equip(?: |$)/.test(needle)) return 0;
  if (line === needle) return 4;
  if (line.includes(needle) || needle.includes(line)) return 3;

  const words = needle.split(" ").filter((word) => word.length >= 4);
  if (words.length === 0) return 0;
  let matched = 0;
  for (const word of words) {
    // Engine action labels summarize effects ("Put counters") while the
    // displayed rules give quantities and counter kinds ("a +1/+1 counter").
    if (line.includes(word) || (word.endsWith("s") && line.includes(word.slice(0,-1)))) matched += 1;
  }
  const ratio = matched / words.length;
  if (ratio >= 0.66) return 2;
  if (ratio >= 0.4) return 1;
  return 0;
}

function actionAbilityText(action) {
  const explicit = String(
    action?.ability_text
    || action?.effect_text
    || action?.action_ref?.ability_text
    || ""
  ).trim();
  if (explicit) return explicit;
  return String(action?.label || "")
    .replace(/^Activate\s+.*?:\s*/i, "")
    .trim();
}

// Use the same bound reminder as the tooltip. Keyword declarations such as
// Improve {B}, Equip {X}, and Crew 2 can expose an activation without a colon.
function keywordActivationText(line, definitions = []) {
  const first=splitTextWithMtgKeywordRules(String(line).trim(),'en',definitions)[0];
  if(first?.type!=='keyword')return null;
  const help=getMtgKeywordHelp(first.text,first.rule,'en',first.context);
  // Without a bound reminder, help falls back to the CR glossary summary, which
  // never has a cost clause. Classify by the keyword's catalog reminder instead.
  const text=help.reminder||first.rule?.custom ? help.description : getMtgKeywordCatalogReminder(first.text)||help.description;
  if (/\{Cost\}/i.test(text || '')) return null; // An unbound identity has no activation cost.
  // A mechanic can combine casting, entry and activation clauses. Only its
  // activation clause participates in action matching (Advance, for example).
  return String(text || '').split(/\n|(?<=[.!?])\s+/)
    .filter(clause=>/^(?!When\b|Whenever\b|At\b|As\b|If\b)[^.:;\n]+:/i.test(clause.trim()))
    .join('\n') || null;
}

// The engine names the printed line each current ability reads as, keyed by
// the same index its legal actions carry. Modal abilities own several lines and
// bind to their first. Identical lines are claimed in order.
export function abilityLinePositions(lines = [], abilityLines = []) {
  const canonical = lines.map(normalizeAbilityMatchText);
  const byAbility = new Map(), activated = new Set(), claimed = new Set();
  for (const { ability_index, kind, text } of abilityLines || []) {
    const parts = String(text || "").split(/\n+/).map(normalizeAbilityMatchText).filter(Boolean);
    if (!parts.length) continue;
    const starts = canonical.map((_, i) => i).filter(i => parts.every((part, k) => canonical[i + k] === part));
    const at = starts.find(i => !claimed.has(i)) ?? starts[0];
    if (at == null) continue;
    claimed.add(at);
    byAbility.set(ability_index, at);
    if (kind === "activated") activated.add(at);
  }
  return { byAbility, activated };
}

// Only activation actions index the source's ability list. Other payment
// sources (convoke, Treasures, ...) and casting actions do not.
function actionAbilityIndex(action) {
  const ref = action?.action_ref;
  if (ref?.kind === "activate_ability" || ref?.kind === "activate_mana_ability") return ref.ability_index ?? null;
  if (action?.kind === "activate_ability" || action?.kind === "activate_mana_ability") return action.ability_index ?? null;
  if (action?.kind === "plan_payment_source" && action.payment_kind === "mana_ability") return action.ability_index ?? null;
  return null;
}

export function matchInteractiveActionsToRulesLines(lines = [], actions = [], definitions = [], abilityLines = null) {
  const matches = new Map();
  const structured = abilityLines?.length ? abilityLinePositions(lines, abilityLines) : null;
  const activatedLineIndices = [...new Set([...activatedAbilityLineIndices(lines, definitions), ...(structured?.activated || [])])]
    .sort((a, b) => a - b);

  actions.forEach((action, actionIndex) => {
    const exact = structured?.byAbility.get(actionAbilityIndex(action));
    if (exact != null) {
      matches.set(exact, [...(matches.get(exact) || []), action]);
      return;
    }
    const needle = actionAbilityText(action);
    let bestIndex = -1;
    let bestScore = 0;
    lines.forEach((line, lineIndex) => {
      if (!activatedLineIndices.includes(lineIndex)) return;
      const expanded=keywordActivationText(line,definitions);
      const score = Math.max(lineAbilityMatchScore(line, needle), expanded ? lineAbilityMatchScore(expanded,needle) : 0);
      if (score > bestScore) {
        bestScore = score;
        bestIndex = lineIndex;
      }
    });
    if (bestIndex < 0 || bestScore === 0) {
      // A described action with no matching line may belong to a hidden
      // ability (e.g. Forest mana on a fetch land under Yavimaya). Never
      // relabel it as a different ability just because a line is available.
      if (needle) return;
      // A filtered action list is not an ability index. Only use ordering
      // when every activated line has an action, or there is just one line.
      bestIndex = activatedLineIndices.length === 1 ? activatedLineIndices[0]
        : actions.length === activatedLineIndices.length ? activatedLineIndices[actionIndex] : -1;
    }
    if (bestIndex < 0) return;
    const current = matches.get(bestIndex) || [];
    current.push(action);
    matches.set(bestIndex, current);
  });
  return matches;
}

// Equip is an activated ability even though its keyword form has no colon.
function isActivatedAbilityLine(line, definitions) {
  const text = String(line).replace(/\([^)]*\)|（[^）]*）/gu, "");
  return /[:：]/u.test(text) || /^\s*(?:equip|equipar)\b/i.test(text) || Boolean(keywordActivationText(text,definitions));
}

export function activatedAbilityLineIndices(lines = [], definitions = []) {
  return lines
    .map((line, index) => {
      if (!isActivatedAbilityLine(line, definitions)) return null;
      const expanded=keywordActivationText(line,definitions);
      // Compiled output may include a standalone keyword identity followed by
      // the actual cost/effect line. The identity is a tooltip, not a second action.
      if (/^[\p{L}\s'-]+\.?$/u.test(line) && expanded && lines.some((other,i)=>i!==index && /[:：]/u.test(other)
        && lineAbilityMatchScore(other,expanded)>=2)) return null;
      return index;
    })
    .filter((index) => index != null);
}

// These engine markers implement the starting counters; they are not printed
// abilities. Keep actual enter-with-counter abilities in the rules text.
export const isIntrinsicStartingCounterLine = line => /^Enters with (loyalty|defense) counters equal to its printed \1 number\.?$/i.test(String(line).trim());
const rulesLines = text => String(text || "").split(/\n+/).map(line => line.trim()).filter(line => line && !isIntrinsicStartingCounterLine(line));
const symbolSignature = line => (line.match(/\{[^}]+\}/g) || []).join("").toUpperCase();

// Resolve engine actions only against canonical text. Translations retain
// ability order even when reminder text introduces or removes paragraphs.
export function interactiveRulesView(canonicalText, displayedText, actions = [], definitions = [], abilityLines = null) {
  const canonical = rulesLines(canonicalText);
  const displayed = rulesLines(displayedText);
  const matches = matchInteractiveActionsToRulesLines(canonical, actions, definitions, abilityLines);
  const structured = abilityLines?.length ? abilityLinePositions(canonical, abilityLines).activated : null;
  const original = { lines: canonical, actions: matches, sourceLines: canonical.map(line => [line]),
    ...(structured && { activatedLines: structured }) };
  if (canonicalText === displayedText) return original;

  // A translation with the same paragraphs, whose mana symbols agree line by
  // line, is the canonical text in another language. Pair it by position so
  // no wording in the interface language has to be recognized.
  if (structured && canonical.length === displayed.length
    && canonical.every((line, index) => symbolSignature(line) === symbolSignature(displayed[index]))) {
    return { ...original, lines: displayed, translated: true };
  }

  const sourceAbilities = activatedAbilityLineIndices(canonical, definitions);
  const targetAbilities = activatedAbilityLineIndices(displayed, definitions);
  const projection = new Map(), targetOf = new Map();
  const sourceLines = displayed.map(() => []);
  for (const index of new Set([...sourceAbilities, ...matches.keys()])) {
    const lineActions = matches.get(index);
    const ordinal = sourceAbilities.indexOf(index);
    let target = -1;
    if (ordinal >= 0 && sourceAbilities.length === targetAbilities.length) {
      target = targetAbilities[ordinal];
      // Mana symbols are language independent and can also identify reordered
      // lines (including otherwise identical dual-land abilities).
      const signature = symbolSignature(canonical[index]);
      const candidates = signature ? targetAbilities.filter(i => symbolSignature(displayed[i]) === signature) : [];
      if (candidates.length === 1) target = candidates[0];
    }
    // Ambiguous/structurally different translations must not relabel actions.
    // Retain canonical rules so every previously available button still works.
    if (target < 0 || sourceLines[target].length) {
      if (lineActions) return original;
      continue;
    }
    sourceLines[target] = [canonical[index]];
    targetOf.set(index, target);
    if (lineActions) projection.set(target, lineActions);
  }
  // Keyword and static lines carry no actions. When both texts have the same
  // number of them they pair up in order, so registered printings can find the
  // printed paragraph behind a translated line and replace it.
  const restSource = canonical.map((_, index) => index).filter(index => !sourceAbilities.includes(index) && !matches.has(index));
  const restTarget = displayed.map((_, index) => index).filter(index => !targetAbilities.includes(index) && !sourceLines[index].length);
  if (restSource.length === restTarget.length) restSource.forEach((index, k) => { sourceLines[restTarget[k]] = [canonical[index]]; });
  const activatedLines = structured && new Set([...structured].map(index => targetOf.get(index)).filter(index => index != null));
  return { lines: displayed, actions: projection, sourceLines, translated: true, ...(activatedLines && { activatedLines }) };
}

// Empty live rules mean abilities were removed; only absent data may use
// printed text. Keep this distinction before stripping intrinsic mana lines.
export function inspectorRulesLines(compiledText, abilities, fallback = '') {
  const source = Array.isArray(compiledText) ? compiledText
    : Array.isArray(abilities) ? abilities : [fallback];
  return source.flatMap(line => stripInspectorAbilityPrefixes(String(line ?? '')).split(/\n+/))
    .map(line => line.trim()).filter(line => line && !isIntrinsicStartingCounterLine(line));
}

const TURN_FACE_UP_KEYWORDS = {
  "turn-face-up cost": "Morph",
  "megamorph cost": "Megamorph",
  "disguise cost": "Disguise",
};

// A face-down permanent's text box is blank (CR 708.2), yet its controller may
// turn it face up. Each offered turn-face-up action becomes a rules line that
// carries its own action, so the inspector can render it as an ability.
export function turnFaceUpRulesLines(actions = []) {
  const lines = [];
  const mapped = actions.map(action => {
    if (action?.kind !== "turn_face_up") return action;
    const cost = String(action.label || "").match(/^(.*?):\s*Turn this face-down permanent face up/i)?.[1]?.trim();
    const keyword = TURN_FACE_UP_KEYWORDS[String(action.action_ref?.method || "").toLowerCase()];
    const line = `${keyword ? `${keyword} ` : ""}${cost || ""}: Turn this permanent face up.`.replace(/^:\s*/, "");
    if (!lines.includes(line)) lines.push(line);
    return { ...action, ability_text: line };
  });
  return { lines, actions: mapped };
}
