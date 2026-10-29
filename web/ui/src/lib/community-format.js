import { deckCardBaseName } from './deck-color-notes.js';

const key = name => deckCardBaseName(String(name || '')).trim().toLowerCase().replace(/\s+\/\s+/g, ' // ');
const aliases = name => [key(name), key(name).split(' // ')[0]];
const names = values => [...new Set((values || []).map(value => String(value).trim()).filter(Boolean))];

// A room carries a snapshot, including set membership and card definitions.
// Guests never need the host's local library or a live Scryfall request.
export function normalizeCommunityFormat(value) {
  if (!value) return null;
  if (JSON.stringify(value).length > 4_000_000) throw new Error('Community format is too large');
  const sets = entries => (entries || []).map(set => ({
    code: String(set.code || '').trim(), name: String(set.name || '').trim(), cards: names(set.cards),
  }));
  const result = {
    version: 1,
    name: String(value.name || '').trim().slice(0, 80),
    includedSets: sets(value.includedSets), excludedSets: sets(value.excludedSets),
    includedCards: names(value.includedCards), excludedCards: names(value.excludedCards),
    customSets: (value.customSets || []).map(set => ({ name: String(set.name || '').trim(), cards: structuredClone(set.cards || []) })),
    customCards: structuredClone(value.customCards || []),
  };
  if (!result.name) throw new Error('Give your community format a name');
  const seen = new Map();
  for (const card of communityCustomCards(result)) {
    if (!card?.draft?.faces?.[0]?.name) throw new Error('Custom cards must contain a named face');
    for (const face of card.draft.faces) {
      const existing = seen.get(key(face.name));
      const definition = JSON.stringify(card.draft);
      if (existing && existing !== definition) throw new Error(`Conflicting custom definitions for ${face.name}`);
      seen.set(key(face.name), definition);
    }
  }
  return result;
}

export function communityCustomCards(format) {
  const byDraft = new Map();
  for (const card of [...(format?.customSets || []).flatMap(set => set.cards), ...(format?.customCards || [])]) {
    byDraft.set(JSON.stringify(card.draft), card);
  }
  return [...byDraft.values()];
}

export function validateCommunityDeck(format, deck = [], commanders = [], sideboard = []) {
  const allowed = new Set([
    ...(format.includedSets || []).flatMap(set => set.cards), ...(format.includedCards || []),
    ...communityCustomCards(format).map(card => card.draft.faces.map(face => face.name).join(' // ')),
  ].flatMap(aliases));
  const excluded = new Set([...(format.excludedSets || []).flatMap(set => set.cards), ...(format.excludedCards || [])].flatMap(aliases));
  const errors = [], cardProblems = {};
  for (const name of names([...deck, ...commanders, ...sideboard])) {
    const keys = aliases(name);
    const reason = keys.some(k => excluded.has(k)) ? `${name} is excluded from ${format.name}.`
      : !keys.some(k => allowed.has(k)) ? `${name} is not included in ${format.name}.` : '';
    if (reason) { errors.push(reason); cardProblems[name] = [reason]; }
  }
  return { ready: errors.length === 0, errors, cardProblems };
}

export function assertCommunityMatch(payload) {
  if (!payload.communityFormat) return;
  if (payload.securityMode !== 'trusted' || payload.format !== 'normal') throw new Error('Community rooms require Trusted freeform play');
  const format = normalizeCommunityFormat(payload.communityFormat);
  for (let i = 0; i < (payload.decks || []).length; i++) {
    const result = validateCommunityDeck(format, payload.decks[i], payload.commanders?.[i], payload.sideboards?.[i]);
    if (!result.ready) throw new Error(result.errors.join(' '));
  }
}

export const emptyCommunityFormat = () => ({ name: '', includedSets: [], excludedSets: [], includedCards: [], excludedCards: [], customSets: [], customCards: [] });
