// Custom named tokens. A token is authored in the Card tab like any card and
// saved by name; another card refers to it the way printed cards refer to a
// predefined token ("Create two Bobble tokens", "Sacrifice a Bobble").
//
// References are rewritten to the compiler's explicitly named token shape
// ("two 1/1 colorless creature tokens named Bobble"). The engine replaces that
// named shape with the token's own strictly compiled definition, which every
// referencing card carries with it (`customTokenFaces`), so saved cards,
// community formats and replays never depend on this browser's library.

export const tokenKey = value => String(value || '').normalize('NFKC').replace(/[‘’]/g, "'").replace(/\s+/g, ' ').trim().toLowerCase();
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

// Predefined tokens (CR 111.10) keep their rules meaning.
const RESERVED = new Set([
  'treasure', 'clue', 'food', 'blood', 'gold', 'map', 'powerstone', 'incubator', 'junk', 'shard',
  'lander', 'mutagen', 'walker', 'heartwood', 'vibranium', 'eldrazi spawn', 'eldrazi scion', 'role',
  'wicked role', 'young hero role', 'monster role', 'sorcerer role', 'royal role', 'virtuous role',
  'cursed role', 'token', 'copy',
]);
const NUMBER_WORDS = 'a|an|one|two|three|four|five|six|seven|eight|nine|ten|that many|X|\\d+';

export function validateToken(value) {
  const face = value?.face;
  const input = value?.input;
  if (!face || !input) throw new Error('A custom token needs its card face.');
  const name = String(face.name || '').normalize('NFKC').replace(/\s+/g, ' ').trim();
  if (!/^[A-Za-z][A-Za-z' -]{0,47}$/.test(name)) {
    throw new Error(`"${name}" cannot name a token. Use 1–48 letters, spaces, hyphens or apostrophes, starting with a letter.`);
  }
  if (RESERVED.has(tokenKey(name))) throw new Error(`"${name}" is a predefined token and cannot be redefined.`);
  if (tokenKey(input.name) !== tokenKey(name)) throw new Error(`The compiled token must keep the name "${name}".`);
  if (String(input.manaCost || '').trim()) throw new Error(`${name} is a token and cannot have a mana cost.`);
  return { key: tokenKey(name), name, face: structuredClone(face), input: structuredClone(input), version: 1 };
}

const outsideQuotation = (text, offset) => (text.slice(0, offset).match(/["“”]/g) || []).length % 2 === 0;
const semanticText = face => String(face?.oracleText || '').replace(/\([^()]*\)/g, '');

// "Bobble token(s)" anywhere, or a bare "a Bobble" / "each Bobble" /
// "Bobbles" after a determiner, as printed cards name Treasures.
function referencePattern(name) {
  const word = escape(name);
  return new RegExp(`\\b(?:(${NUMBER_WORDS}|another|each|target|that|those|all|any number of)\\s+)?(tapped\\s+)?(${word})(s)?(?:\\s+(tokens?))?\\b`, 'gi');
}

function referencesToken(face, token) {
  const text = semanticText(face);
  for (const match of text.matchAll(referencePattern(token.name))) {
    if (!outsideQuotation(text, match.index)) continue;
    if (match[1] || match[5]) return true;
  }
  return false;
}

// The least shape that parses as a named token; the engine substitutes the
// token's full definition, so its types, colors and abilities are not
// repeated. Creatures keep a creature shape for "tapped and attacking"; every
// other token uses the artifact shape (a land shape drops its name).
function tokenShape(token) {
  if (!(token.input.cardTypes || []).includes('Creature')) return 'colorless artifact';
  const stat = value => (/^\d+$/.test(String(value ?? '').trim()) ? String(value).trim() : '0');
  return `${stat(token.input.power)}/${stat(token.input.toughness)} colorless creature`;
}

function rewriteReferences(text, token) {
  const before = text;
  return text.replace(referencePattern(token.name), (match, determiner, tapped, _name, plural, noun, offset) => {
    if (!outsideQuotation(before, offset) || (!determiner && !noun)) return match;
    const many = Boolean(plural) || /s$/i.test(noun || '') || (determiner && !/^(a|an|one|another|each|target|that)$/i.test(determiner));
    // A creation names the token's characteristics; any other reference
    // ("sacrifice a Bobble") only its name.
    const creating = /\bcreates?\b[^.!?"]*$/i.test(before.slice(0, offset));
    const head = determiner ? `${/^an$/i.test(determiner) ? 'a' : determiner} ` : '';
    const tokens = many ? 'tokens' : 'token';
    return `${head}${tapped || ''}${creating ? `${tokenShape(token)} ` : ''}${tokens} named ${token.name}`;
  });
}

// The tokens a draft's faces refer to, in dependency order: a token one
// referenced token creates is compiled before it.
export function draftTokens(draft, known = []) {
  const byKey = new Map();
  // This browser's current library wins over the copy a saved card carries,
  // so editing a token updates the cards that create it.
  for (const value of [...(draft.customTokens || []), ...known]) {
    const token = validateToken(value);
    byKey.set(token.key, token);
  }
  const ordered = [];
  const visit = (face, seen) => {
    for (const token of byKey.values()) {
      if (seen.has(token.key) || !referencesToken(face, token)) continue;
      visit(token.input, new Set([...seen, token.key]));
      if (!ordered.includes(token)) ordered.push(token);
    }
  };
  for (const face of draft.faces || []) visit(face, new Set());
  return ordered;
}

// Rewrite references on compiler-ready faces and attach the token faces.
export function applyCustomTokens(input, tokens) {
  if (!tokens.length) {
    const result = { ...input };
    delete result.customTokenFaces;
    return result;
  }
  const rewrite = face => {
    const used = tokens.filter(token => referencesToken(face, token));
    if (!used.length) return face;
    let text = String(face.oracleText || '');
    for (const token of used) text = rewriteReferences(text, token);
    const displayOracleText = face.displayOracleText ?? String(face.oracleText || '').split('\n').map(line => line.trim()).filter(Boolean).join('\n');
    return { ...face, oracleText: text, displayOracleText };
  };
  return {
    ...input,
    faces: input.faces.map(rewrite),
    customTokenFaces: tokens.map(token => rewrite(token.input)),
  };
}

const active = new Map();
let revision = 0;
export const customTokenRevision = () => revision;
export function installCustomTokens(values) {
  for (const value of values) {
    const token = validateToken(value);
    if (JSON.stringify(active.get(token.key)) !== JSON.stringify(token)) {
      active.set(token.key, token);
      revision++;
    }
  }
}
export function uninstallCustomToken(name) {
  if (active.delete(tokenKey(name))) revision++;
}
export const activeCustomTokens = () => [...active.values()];
