import { mergeMechanics, prepareCustomCard } from './custom-mechanics.js';

export function setMechanics(value) {
  return mergeMechanics(value.customMechanics || [], value.customCards.flatMap(card => card.draft.customMechanics || []));
}

// Set definitions override the copies carried by cards without touching the
// personal library's registry. Compilation stays isolated to this workspace.
export async function prepareSetCard(draft, definitions, compile) {
  const keywords = new Set(definitions.map(def => def.keyword));
  const prepared = await prepareCustomCard({ ...draft, customMechanics: (draft.customMechanics || []).filter(def => !keywords.has(def.keyword)) }, compile, { known: definitions });
  prepared.preview.customMechanics = prepared.draft.customMechanics;
  for (const face of prepared.preview.faces || []) face.customMechanics = prepared.draft.customMechanics;
  return prepared;
}

export async function updateSetMechanic(value, definition, compile) {
  const definitions = mergeMechanics(setMechanics(value).filter(def => def.keyword !== definition.keyword), [definition]);
  const cards = [];
  for (const card of value.customCards) {
    try {
      const prepared = await prepareSetCard(card.draft, definitions, compile);
      cards.push({ ...card, draft: prepared.draft });
    } catch (error) {
      throw new Error(`${card.draft.faces[0].name}: ${error.message}`);
    }
  }
  return { ...value, customMechanics: definitions, customCards: cards };
}
