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
  if (line === needle) return 4;
  if (line.includes(needle) || needle.includes(line)) return 3;

  const words = needle.split(" ").filter((word) => word.length >= 4);
  if (words.length === 0) return 0;
  let matched = 0;
  for (const word of words) {
    if (line.includes(word)) matched += 1;
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

export function matchInteractiveActionsToRulesLines(lines = [], actions = []) {
  const matches = new Map();
  const activatedLineIndices = activatedAbilityLineIndices(lines);

  actions.forEach((action, actionIndex) => {
    const needle = actionAbilityText(action);
    let bestIndex = -1;
    let bestScore = 0;
    lines.forEach((line, lineIndex) => {
      const score = lineAbilityMatchScore(line, needle);
      if (score > bestScore) {
        bestScore = score;
        bestIndex = lineIndex;
      }
    });
    if (bestIndex < 0 || bestScore === 0) {
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

export function activatedAbilityLineIndices(lines = []) {
  return lines
    .map((line, index) => (/[:：]/u.test(String(line).replace(/\([^)]*\)|（[^）]*）/gu, "")) ? index : null))
    .filter((index) => index != null);
}

const rulesLines = text => String(text || "").split(/\n+/).map(line => line.trim()).filter(Boolean);
const symbolSignature = line => (line.match(/\{[^}]+\}/g) || []).join("").toUpperCase();

// Resolve engine actions only against canonical text. Translations retain
// ability order even when reminder text introduces or removes paragraphs.
export function interactiveRulesView(canonicalText, displayedText, actions = []) {
  const canonical = rulesLines(canonicalText);
  const displayed = rulesLines(displayedText);
  const matches = matchInteractiveActionsToRulesLines(canonical, actions);
  const original = { lines: canonical, actions: matches, sourceLines: canonical.map(line => [line]) };
  if (canonicalText === displayedText) return original;

  const sourceAbilities = activatedAbilityLineIndices(canonical);
  const targetAbilities = activatedAbilityLineIndices(displayed);
  const projection = new Map();
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
    if (lineActions) projection.set(target, lineActions);
  }
  return { lines: displayed, actions: projection, sourceLines, translated: true };
}
