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

export function groupManaAbilities(view, locale = 'en') {
  const rows = [];
  const groups = new Map();
  view.lines.forEach((line, index) => {
    const sources = view.sourceLines?.[index] || [line];
    const ability = sources.length === 1 ? simpleManaAbility(sources[0]) : null;
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
  const formatter = new Intl.ListFormat(view.translated ? locale : 'en', { style: 'long', type: 'disjunction' });
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
