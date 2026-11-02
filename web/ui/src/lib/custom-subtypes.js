// Builder vocabulary is a library, not a game's type universe. Only referenced
// definitions travel with a card; the native compiler validates them again.
export const CUSTOM_SUBTYPES_CHANGED = 'ironsmith-custom-subtypes-changed';
export const SUBTYPE_FAMILIES = ['Creature', 'Land', 'Artifact', 'Enchantment', 'Spell', 'Planeswalker', 'Battle', 'Planar'];
const STORAGE_KEY = 'ironsmith-custom-subtypes-v1';
export const canonicalSubtypeText = value => String(value || '').trim().replace(/\s+/gu, ' ').replace(/[‘’]/g, "'").replace(/−/g, '-').toLowerCase();
const key = canonicalSubtypeText;
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
export function subtypeFormsMentioned(text, definition) {
  const body = canonicalSubtypeText(text);
  return [definition.singular,definition.plural].some(form=>
    [key(form),`non${key(form)}`,`non-${key(form)}`].some(value=>new RegExp(`(?<![\\p{L}\\p{N}])${escape(value)}(?![\\p{L}\\p{N}])`,'u').test(body)));
}
export function suggestSubtypePlural(singular) {
  const word = String(singular || '').trim();
  if (/[^aeiou]y$/i.test(word)) return `${word.slice(0, -1)}ies`;
  return `${word}${/(s|x|z|ch|sh)$/i.test(word) ? 'es' : 's'}`;
}
export function validateSubtype(value) {
  const singular = key(value?.singular), plural = key(value?.plural);
  const word = "[\\p{L}][\\p{L}\\p{N}]*(?:['-][\\p{L}\\p{N}]+)*";
  const form = new RegExp(`^${word}(?: ${word})*$`, 'u');
  if (![singular, plural].every(value => new TextEncoder().encode(value).length <= 32 && form.test(value))) {
    throw new Error('Use subtype names of up to 32 UTF-8 bytes, with letters/numbers and internal hyphens, apostrophes or spaces.');
  }
  if (!SUBTYPE_FAMILIES.includes(value?.family)) throw new Error('Choose a subtype family.');
  return { singular, plural, family: value.family };
}
export function mergeSubtypes(...groups) {
  const definitions = [], aliases = new Map();
  for (const input of groups.flat()) {
    const def = validateSubtype(input);
    const prior = definitions.find(item => item.singular === def.singular);
    if (prior && prior.plural !== def.plural) throw new Error(`Conflicting definition for ${def.singular}.`);
    for (const word of [def.singular, def.plural]) {
      const owner = aliases.get(word);
      if (owner && owner !== def.singular) throw new Error(`Ambiguous subtype form: ${word}.`);
      aliases.set(word, def.singular);
    }
    // Repeated spelling declares another association of the same subtype.
    if (!definitions.some(item => item.singular === def.singular && item.family === def.family)) definitions.push(def);
  }
  return definitions.sort((a, b) => a.singular.localeCompare(b.singular) || SUBTYPE_FAMILIES.indexOf(a.family) - SUBTYPE_FAMILIES.indexOf(b.family));
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
export function suppliedDefinitionFaces(definitions = []) {
  return definitions.map(definition=>{
    const lines=String(definition.text||'').split('\n');
    return {typeLine:lines.find(line=>/^Type:/.test(line))?.slice(5).trim()||'',
      oracleText:lines.filter(line=>!/^(?:Mana cost|Type|Power\/Toughness|Loyalty|Defense|Color indicator):/.test(line)).join('\n')};
  });
}
export function referencedSubtypes(draft, known = []) {
  const supplied=[...(draft.suppliedCardDefinitions||[]),...(draft.faces||[]).flatMap(face=>face.suppliedCardDefinitions||[])];
  const definitions = mergeSubtypes(known, draft.customSubtypes || [], ...supplied.map(card=>card.customSubtypes||[]),
    ...(draft.customTokens || []).map(token => token.customSubtypes || []));
  const faces = [...(draft.faces || []), ...suppliedDefinitionFaces(supplied), ...(draft.customTokenFaces || []),
    ...(draft.customTokens || []).flatMap(token => [token.face, token.input].filter(Boolean))];
  // Names and flavor text are deliberately excluded. Parenthetical reminder
  // prose does not declare a vocabulary dependency either.
  const text = faces.flatMap(face => [
    ...(face.subtypes || []).map(key),
    String(face.typeLine || '').split(/[—–]/).slice(1).join(' '),
    String(face.oracleText || '').replace(/\([^()]*\)/g, ''),
  ]).join('\n');
  return definitions.filter(def => subtypeFormsMentioned(text, def));
}
export function withDraftSubtypes(draft, known = []) {
  const customSubtypes=referencedSubtypes(draft,known);
  const supplied=[...(draft.suppliedCardDefinitions||[]),...(draft.faces||[]).flatMap(face=>face.suppliedCardDefinitions||[])];
  if(!supplied.length)return {...draft,customSubtypes};
  const byName=new Map();
  for(const definition of supplied){
    const name=String(definition.name||'').normalize('NFKC').trim().toLowerCase();
    if(!byName.has(name))byName.set(name,{...definition,customSubtypes:referencedSubtypes({customSubtypes:mergeSubtypes(customSubtypes,definition.customSubtypes||[]),faces:suppliedDefinitionFaces([definition])},known)});
  }
  const suppliedCardDefinitions=[...byName.values()];
  return {...draft,customSubtypes,suppliedCardDefinitions,faces:(draft.faces||[]).map(face=>({...face,suppliedCardDefinitions}))};
}
