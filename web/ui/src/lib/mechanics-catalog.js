// The Mechanics tab's view of every keyword: official ones from the rules
// reference plus the lexicon's forms, and saved custom ones with their own.
import { MTG_KEYWORD_RULES } from './mtg-keyword-rules.generated.js';
import { officialKeywordForms, customKeywordForms, deriveParticiple } from './keyword-forms.js';

const officialForms = rule => {
  const lexicon = [rule.title, ...(rule.aliases || [])].map(officialKeywordForms).find(Boolean);
  return lexicon || { name: rule.title.toLowerCase(), participle: null, designation: null, costNoun: null, variants: [] };
};

export function officialMechanics() {
  return MTG_KEYWORD_RULES
    .filter(rule => /^70[12]\./.test(rule.rule))
    .map(rule => ({ id: rule.id, name: rule.title, kind: rule.kind === 'keywordAction' ? 'action' : 'ability', rule: rule.rule, summary: rule.summary, custom: false, forms: officialForms(rule) }));
}

export function customMechanicEntry(def) {
  return { id: `custom:${def.keyword}`, name: def.keyword, kind: 'custom', summary: def.reminder, expansion: def.expansion, sourceCard: def.sourceCard, custom: true, forms: customKeywordForms(def), definition: def };
}

export function mechanicEntries(customDefinitions, { query = '', filter = 'all' } = {}) {
  const needle = query.trim().toLowerCase();
  const entries = [...customDefinitions.map(customMechanicEntry).sort((a, b) => a.name.localeCompare(b.name)), ...officialMechanics()];
  return entries.filter(entry => (filter === 'all' || (filter === 'custom') === entry.custom)
    && (!needle || [entry.name, entry.forms.participle, entry.forms.designation, entry.forms.costNoun, ...entry.forms.variants]
      .some(form => form && form.toLowerCase().includes(needle))));
}

// Real keywords illustrating each form, read from the lexicon.
const example = (name, field, render) => {
  const forms = officialKeywordForms(name);
  const value = field === 'variants' ? forms?.variants?.[0] : forms?.[field];
  return value ? { keyword: name, form: value, text: render(value) } : null;
};

export const FORM_FIELDS = [
  {
    field: 'participle',
    label: 'Past participle',
    help: 'How rules text says the action happened, as in "if it was …".',
    examples: [
      example('Kicker', 'participle', form => `if this spell was ${form}`),
      example('Foretell', 'participle', form => `a card that was ${form}`),
      example('Beam me up', 'participle', form => `can't be ${form}`),
    ].filter(Boolean),
    preview: form => `When this creature enters, if it was ${form}, draw a card.`,
  },
  {
    field: 'designation',
    label: 'Designation',
    help: 'An adjective naming objects marked by the mechanic.',
    examples: [
      example('Monstrosity', 'designation', form => `if this creature is ${form}`),
      example('Renown', 'designation', form => `${form} creatures you control`),
      example('Plot', 'designation', form => `a ${form} card`),
    ].filter(Boolean),
    preview: form => `Whenever you cast a ${form} spell, draw a card.`,
  },
  {
    field: 'costNoun',
    label: 'Cost name',
    help: 'The word before "cost" when the reminder names its price.',
    examples: [
      example('Cumulative upkeep', 'costNoun', form => `pay its ${form} cost`),
      example('Flashback', 'costNoun', form => `its ${form} cost`),
      example('Warp', 'costNoun', form => `cast it for its ${form} cost`),
    ].filter(Boolean),
    preview: form => `You may cast this spell for its ${form} cost.`,
  },
  {
    field: 'variants',
    label: 'Other forms',
    help: 'Phrasal forms that mean the same thing, separated by commas.',
    examples: [
      example('Phasing', 'variants', form => `if it ${form}`),
      example('Soulbond', 'variants', form => `while it's ${form}`),
      example('Cipher', 'variants', form => `an ${form} card`),
    ].filter(Boolean),
    preview: form => `If this spell was ${form}, draw a card.`,
  },
];

export const defaultForms = keyword => ({ participle: deriveParticiple(keyword || 'blink'), designation: '', costNoun: String(keyword || '').trim().toLowerCase(), variants: '' });
