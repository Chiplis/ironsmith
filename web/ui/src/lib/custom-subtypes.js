// Builder vocabulary is a library, not a game's type universe. Only referenced
// definitions travel with a card; the native compiler validates them again.
export const CUSTOM_SUBTYPES_CHANGED = 'ironsmith-custom-subtypes-changed';
export const SUBTYPE_FAMILIES = ['Creature', 'Land', 'Artifact', 'Enchantment', 'Spell', 'Planeswalker', 'Battle'];
const STORAGE_KEY = 'ironsmith-custom-subtypes-v1';
const key = value => String(value || '').trim().toLowerCase();
export function suggestSubtypePlural(singular) {
  const word = String(singular || '').trim();
  if (/[^aeiou]y$/i.test(word)) return `${word.slice(0, -1)}ies`;
  return `${word}${/(s|x|z|ch|sh)$/i.test(word) ? 'es' : 's'}`;
}
export function validateSubtype(value) {
  const singular = key(value?.singular), plural = key(value?.plural);
  if (![singular, plural].every(word => /^[a-z]{1,32}$/.test(word))) {
    throw new Error('Use 1–32 letters for each subtype form (one word).');
  }
  if (!SUBTYPE_FAMILIES.includes(value?.family)) throw new Error('Choose a subtype family.');
  return { singular, plural, family: value.family };
}
export function mergeSubtypes(...groups) {
  const definitions = [], aliases = new Map();
  for (const input of groups.flat()) {
    const def = validateSubtype(input);
    const prior = definitions.find(item => item.singular === def.singular);
    if (prior && JSON.stringify(prior) !== JSON.stringify(def)) throw new Error(`Conflicting definition for ${def.singular}.`);
    for (const word of [def.singular, def.plural]) {
      const owner = aliases.get(word);
      if (owner && owner !== def.singular) throw new Error(`Ambiguous subtype form: ${word}.`);
      aliases.set(word, def.singular);
    }
    if (!prior) definitions.push(def);
  }
  return definitions.sort((a, b) => a.singular.localeCompare(b.singular));
}
export function listCustomSubtypes(storage = globalThis.localStorage) {
  return mergeSubtypes(JSON.parse(storage?.getItem(STORAGE_KEY) || '[]'));
}
export function storeCustomSubtypes(definitions, storage = globalThis.localStorage) {
  if (!storage) throw new Error('Subtype storage is unavailable.');
  const result = mergeSubtypes(definitions);
  storage.setItem(STORAGE_KEY, JSON.stringify(result));
  globalThis.dispatchEvent?.(new Event(CUSTOM_SUBTYPES_CHANGED));
  return result;
}
export function referencedSubtypes(draft, known = []) {
  const definitions = mergeSubtypes(known, draft.customSubtypes || [],
    ...(draft.customTokens || []).map(token => token.customSubtypes || []));
  const faces = [...(draft.faces || []), ...(draft.customTokenFaces || []),
    ...(draft.customTokens || []).flatMap(token => [token.face, token.input].filter(Boolean))];
  // Names and flavor text are deliberately excluded. Parenthetical reminder
  // prose does not declare a vocabulary dependency either.
  const words = new Set(faces.flatMap(face => [
    ...(face.subtypes || []).map(key),
    ...(String(face.oracleText || '').replace(/\([^()]*\)/g, '').toLowerCase().match(/[a-z]+/g) || []),
  ]));
  return definitions.filter(def => words.has(def.singular) || words.has(def.plural));
}
export function withDraftSubtypes(draft, known = []) {
  return { ...draft, customSubtypes: referencedSubtypes(draft, known) };
}
