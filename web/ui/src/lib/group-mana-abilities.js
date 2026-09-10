import { messages } from '../i18n/messages.js';

// Deliberately accept only pure, fixed mana production. Conditions, spending
// restrictions, triggers, and additional effects must retain their own lines.
export function simpleManaAbility(line) {
  const text = String(line || '').trim().replace(/^\((.*)\)$/u, '$1');
  const match = text.match(/^([^:]+):\s*Add\s+((?:\{[WUBRGC]\}\s*)+)\.$/i);
  if (!match) return null;
  return { cost: match[1].trim().replace(/\s+/g, ' '), output: match[2].replace(/\s/g, '').toUpperCase() };
}

function localizedTemplate(line, output) {
  const colon = Math.max(line.indexOf(':'), line.indexOf('：'));
  if (colon < 0) return null;
  const symbols = [...line.slice(colon + 1).matchAll(/\{[^}]+\}/g)];
  if (!symbols.length || symbols.map(match => match[0]).join('').toUpperCase() !== output) return null;
  const start = colon + 1 + symbols[0].index;
  const end = colon + 1 + symbols.at(-1).index + symbols.at(-1)[0].length;
  if (!/^(?:\{[^}]+\}\s*)+$/.test(line.slice(start, end))) return null;
  return { prefix: line.slice(0, start), suffix: line.slice(end) };
}

// Lands produce mana through intrinsic abilities the engine words in English
// and no printing translates ("{T}: Add {U}."), so they stay English even when
// a card's other text is official Spanish. Fixed mana production is a closed
// phrase: swap its verb for the interface language's when the line was never
// translated. Anything already localized, or with conditions, is left alone.
// Oracle wording of dual lands already joins the outputs ("Add {U} or {R}").
const FIXED_MANA_CHOICES = /^(\(?)([^:]+):\s*Add\s+((?:\{[WUBRGC]\}\s*)+(?:,?\s*or\s+(?:\{[WUBRGC]\}\s*)+|,\s*(?:\{[WUBRGC]\}\s*)+)*)\.(\)?)$/i;
export function localizeSimpleManaLine(line, source, locale = 'en') {
  const verb = messages[locale]?.['card.manaAbility.add'];
  if (!verb || locale === 'en' || line !== source) return { line, localized: false };
  const match = String(line).trim().match(FIXED_MANA_CHOICES);
  if (!match) return { line, localized: false };
  const [, open, cost, outputs, close] = match;
  const choices = outputs.split(/\s*,?\s*\bor\b\s*|\s*,\s*/i).map(choice => choice.replace(/\s/g, '')).filter(Boolean);
  const joined = new Intl.ListFormat(locale, { style: 'long', type: 'disjunction' }).format(choices);
  return { line: `${open}${cost.trim()}: ${verb} ${joined}.${close}`, localized: true };
}

export function groupManaAbilities(view, locale = 'en') {
  const rows = [];
  const groups = new Map();
  let localized = false;
  view.lines.forEach((displayed, index) => {
    const sources = view.sourceLines?.[index] || [displayed];
    const ability = sources.length === 1 ? simpleManaAbility(sources[0]) : null;
    const local = sources.length === 1 ? localizeSimpleManaLine(displayed, sources[0], locale) : { line: displayed, localized: false };
    const line = local.line;
    localized ||= local.localized;
    const template = ability && localizedTemplate(line, ability.output);
    const actions = view.actions.get(index) || [];
    // Keep localized cost wording and sentence structure identical, too.
    const key = template && JSON.stringify([ability.cost, template.prefix, template.suffix]);
    let row = key && groups.get(key);
    if (!row) {
      row = { line, actions: [], sources: [], options: [], template };
      rows.push(row);
      if (key) groups.set(key, row);
    }
    row.actions.push(...actions);
    row.sources.push(...sources);
    if (template) {
      let option = row.options.find(option => option.output === ability.output);
      if (!option) {
        option = { output: ability.output, actions: [] };
        row.options.push(option);
      }
      option.actions.push(...actions);
    }
  });
  const formatter = new Intl.ListFormat(view.translated || localized ? locale : 'en', { style: 'long', type: 'disjunction' });
  const actions = new Map(), manaGroups = new Map();
  const lines = rows.map((row, index) => {
    if (row.actions.length) actions.set(index, row.actions);
    if (row.template && row.options.length > 1) {
      const parts = formatter.formatToParts(row.options.map(option => option.output));
      manaGroups.set(index, { ...row.template, options: row.options, parts });
      return row.template.prefix + parts.map(part => part.value).join('') + row.template.suffix;
    }
    return row.line;
  });
  return { lines, actions, manaGroups, sourceLines: rows.map(row => row.sources) };
}
