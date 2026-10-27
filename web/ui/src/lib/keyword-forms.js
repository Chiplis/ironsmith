// Keyword surface forms: name, participle/designation, cost noun, variants.
// Official forms come from the compiler's lexicon (keyword-forms.generated.js);
// custom forms follow the same derivation rule unless a definition overrides
// them. Forms are vocabulary only: they never confer keyword identity.
import { OFFICIAL_KEYWORD_FORMS, KEYWORD_IRREGULAR_HEADS } from './keyword-forms.generated.js';

export { OFFICIAL_KEYWORD_FORMS };

const key = value => String(value || '').normalize('NFKC').replace(/[‘’]/g, "'").replace(/\s+/g, ' ').trim().toLowerCase();
const byName = new Map(OFFICIAL_KEYWORD_FORMS.map(forms => [forms.name, forms]));
const vowel = character => 'aeiou'.includes(character);

function inflectHead(word) {
  if (Object.hasOwn(KEYWORD_IRREGULAR_HEADS, word)) return KEYWORD_IRREGULAR_HEADS[word];
  const n = word.length;
  if (word.endsWith('ed')) return word;
  if (word.endsWith('e')) return `${word}d`;
  if (n >= 2 && word[n - 1] === 'y' && !vowel(word[n - 2])) return `${word.slice(0, -1)}ied`;
  if (word.endsWith('c')) return `${word}ked`;
  const vowelGroups = [...word].filter((character, index) => vowel(character) && (index === 0 || !vowel(word[index - 1]))).length;
  if (n >= 3 && vowelGroups === 1 && !vowel(word[n - 3]) && vowel(word[n - 2]) && !vowel(word[n - 1]) && !'wxy'.includes(word[n - 1])) {
    return `${word}${word[n - 1]}ed`;
  }
  return `${word}ed`;
}

// The custom-keyword participle rule: inflect the first word, keep the rest.
export function deriveParticiple(name) {
  const [head, ...rest] = key(name).split(' ');
  return [inflectHead(head), ...rest].join(' ');
}

export const officialKeywordForms = name => byName.get(key(name)) || null;
export const officialFormsWithCostNoun = noun => OFFICIAL_KEYWORD_FORMS.filter(forms => forms.costNoun === key(noun));

export function surfaceForms(forms) {
  return [...new Set([forms.name, forms.participle, forms.designation, ...(forms.variants || [])].filter(Boolean))];
}

// Every official name-derived form (participles, designations, variants).
export const officialSurfaceForms = () => OFFICIAL_KEYWORD_FORMS.flatMap(surfaceForms);

const FORM_FIELDS = ['participle', 'designation', 'costNoun'];

// Explicit custom overrides: { participle?, designation?, costNoun? }.
export function normalizeFormOverrides(forms) {
  if (forms == null) return undefined;
  if (typeof forms !== 'object' || Array.isArray(forms)) throw new Error('Custom keyword forms must be an object of participle, designation, and costNoun.');
  const result = {};
  for (const [field, value] of Object.entries(forms)) {
    if (!FORM_FIELDS.includes(field)) throw new Error(`Unknown custom keyword form "${field}". Use participle, designation, or costNoun.`);
    if (value == null || value === '') continue;
    const form = key(value);
    if (!/^[a-z][a-z '-]{0,63}$/.test(form)) throw new Error(`Custom keyword form "${value}" must be plain words.`);
    result[field] = form;
  }
  return Object.keys(result).length ? result : undefined;
}

// A custom keyword's forms, derived from its name unless overridden.
export function customKeywordForms(definition) {
  const name = key(definition.keyword);
  const overrides = normalizeFormOverrides(definition.forms) || {};
  return {
    name,
    participle: overrides.participle || deriveParticiple(name),
    designation: overrides.designation || null,
    costNoun: overrides.costNoun || name,
    variants: [],
  };
}
