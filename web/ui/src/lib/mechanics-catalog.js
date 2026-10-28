// The Mechanics tab's view of every keyword: official ones from the rules
// reference plus the lexicon's forms, and saved custom ones with their own.
import { MTG_KEYWORD_RULES } from './mtg-keyword-rules.generated.js';
import { officialKeywordForms, customKeywordForms, deriveParticiple } from './keyword-forms.js';
import { MTG_KEYWORD_REMINDERS } from './mtg-keyword-reminders.generated.js';
import { COMPOSITE_REMINDERS, COST_REMINDERS, INDIRECT_COSTS, NUMBER_REMINDERS } from './keyword-reminder-context.js';

const officialForms = rule => {
  const lexicon = [rule.title, ...(rule.aliases || [])].map(officialKeywordForms).find(Boolean);
  return lexicon || { name: rule.title.toLowerCase(), participle: null, designation: null, costNoun: null, variants: [] };
};

// Reminder text for a keyword with no card around it: parameterized ones are
// written with N and {Cost} slots, and a printed reminder whose numbers belong
// to one card says which printing it quotes. Falls back to the CR summary.
function officialReminder(rule) {
  const keys = [rule.title, ...(rule.aliases || [])].map(alias => alias.toLowerCase());
  for (const key of keys.map(key => (key === 'web slinging' ? 'web-slinging' : key))) {
    if (COMPOSITE_REMINDERS[key]) return { summary: COMPOSITE_REMINDERS[key]('N', '{Cost}') };
    if (NUMBER_REMINDERS[key]) return { summary: NUMBER_REMINDERS[key]('N') };
    if (COST_REMINDERS[key]) return { summary: COST_REMINDERS[key]('{Cost}') };
    const printed = MTG_KEYWORD_REMINDERS[key];
    if (!printed) continue;
    return printed.example && !INDIRECT_COSTS.has(key)
      ? { summary: printed.text, reminderSource: { example: printed.example, card: printed.card } }
      : { summary: printed.text };
  }
  return { summary: rule.summary };
}

export function officialMechanics() {
  return MTG_KEYWORD_RULES
    .filter(rule => /^70[12]\./.test(rule.rule))
    .map(rule => ({ id: rule.id, name: rule.title, kind: rule.kind === 'keywordAction' ? 'action' : 'ability', rule: rule.rule, ...officialReminder(rule), custom: false, forms: officialForms(rule) }));
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

// Starting templates for a custom reminder, one per kind of rules text. Each
// is compiled by the custom-mechanics wasm test, so every template stays
// valid. {Cost} stands for the price each card writes after the keyword.
export const REMINDER_TEMPLATES = [
  {
    kind: 'Static ability', keyword: 'sneaky',
    reminder: "This creature can't be blocked.",
    usage: 'Sneaky',
    note: 'A passive ability that is always on while the permanent is on the battlefield. Describe what is true, with no "When" or "{Cost}:".',
  },
  {
    kind: 'Activated ability', keyword: 'blink',
    reminder: "{Cost}: Exile this creature, then return it to the battlefield under its owner's control.",
    usage: 'Blink {2}{W}',
    note: 'Start with "{Cost}:" to make an ability you can activate. A card may also use a non-mana cost, as in "Blink—Sacrifice another creature."',
  },
  {
    kind: 'Triggered ability', keyword: 'glean',
    reminder: 'When this creature enters, you may pay {Cost}. If you do, draw a card.',
    usage: 'Glean {1}{U}',
    note: 'Start with "When", "Whenever" or "At" to make it trigger.',
  },
  {
    kind: 'Replacement effect', keyword: 'persevere',
    reminder: 'If this creature would die, you may pay {Cost} instead. If you do, exile it.',
    usage: 'Persevere {2}',
    note: 'Use "If … would …, … instead" to change what an event does.',
  },
  {
    kind: 'Additional cost', keyword: 'empower',
    reminder: 'As an additional cost to cast this spell, you may pay {Cost}.',
    usage: 'Empower {2}',
    note: 'Paid on top of the mana cost while casting. Other text can check it with the past participle, as in "if it was empowered".',
  },
  {
    kind: 'Alternative cost', keyword: 'rush',
    reminder: "You may pay {Cost} rather than pay this spell's mana cost.",
    usage: 'Rush {R}',
    note: 'Paid instead of the mana cost while casting. Other text can check a mana price with the past participle, as in "if it was rushed". A card may also use a non-mana cost, as in "Rush—Sacrifice a creature."',
  },
  {
    kind: 'Timed exile', keyword: 'foresee',
    reminder: 'Rather than cast this card from your hand, you may pay {Cost} and exile it with {Number} progress counters on it. At the beginning of your upkeep, or whenever an artifact or creature you control enters, remove a progress counter. When the last is removed, you may cast it without paying its mana cost.',
    usage: 'Foresee 3—{1}{U}',
    note: 'Write {Number} where an amount goes. Each card then gives the amount and its price after the keyword, as Suspend does: "Foresee 3—{1}{U}". Any counter name works, and "or whenever …" adds more ways to remove one.',
  },
  {
    kind: 'Spell instruction', keyword: 'glimmer',
    reminder: 'Draw a card.',
    usage: 'Glimmer.',
    note: 'Without {Cost}, the keyword is an instruction. It runs when an instant or sorcery resolves, or wherever another ability says to.',
  },
];

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
    help: 'Only needed when other rules text names the price, as in "its … cost". Reminders can simply say {Cost}.',
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
