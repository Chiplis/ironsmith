import { mergeSubtypes } from './custom-subtypes.js';
import { mergeMechanics, prepareCustomCard } from './custom-mechanics.js';

export function sameSetMechanic(a, b) {
  if (a.keyword !== b.keyword) return false;
  try { mergeMechanics([a], [b]); mergeMechanics([b], [a]); return true; }
  catch { return false; }
}
export function setMechanics(value) {
  const definitions=[];
  for(const raw of [...(value.customMechanics||[]), ...value.customCards.flatMap(card=>card.draft.customMechanics||[])]) {
    const [definition]=mergeMechanics([], [raw]); // official keyword restrictions still apply
    if(!definitions.some(prior=>sameSetMechanic(prior,definition)))definitions.push(definition);
  }
  return definitions;
}
export function setSubtypes(value) {
  return mergeSubtypes(value.customSubtypes || [], value.customCards.flatMap(card => card.draft.customSubtypes || []));
}
export async function prepareSetCard(draft, definitions, compile, subtypes = []) {
  const selected=[];
  for(const keyword of new Set(definitions.map(def=>def.keyword))) {
    const variants=definitions.filter(def=>def.keyword===keyword);
    const local=(draft.customMechanics||[]).find(def=>def.keyword===keyword);
    const match=local&&variants.find(def=>sameSetMechanic(def,local));
    if(match)selected.push(match);
    else if(variants.length===1)selected.push(variants[0]);
    // Without a local variant, leave discovery to the card's own reminder.
  }
  const keywords = new Set(selected.map(def => def.keyword));
  const prepared = await prepareCustomCard({ ...draft, customMechanics: (draft.customMechanics || []).filter(def => !keywords.has(def.keyword)) }, compile, { known: selected, knownSubtypes: subtypes });
  prepared.preview.customMechanics = prepared.draft.customMechanics;
  for (const face of prepared.preview.faces || []) face.customMechanics = prepared.draft.customMechanics;
  return prepared;
}
export async function updateSetMechanic(value, definition, compile, previous = null) {
  const all=setMechanics(value);
  const applies=def=>previous?sameSetMechanic(def,previous):def.keyword===definition.keyword;
  const definitions = [...all.filter(def=>!applies(def)),...mergeMechanics([], [definition])];
  const cards = [];
  for (const card of value.customCards) {
    try {
      const own=card.draft.customMechanics||[];
      const affected=own.some(applies);
      if (previous && !affected) { cards.push(structuredClone(card)); continue; }
      const draft=affected?{...card.draft,customMechanics:[...own.filter(def=>!applies(def)),definition]}:card.draft;
      const prepared = await prepareSetCard(draft, definitions, compile, setSubtypes(value));
      cards.push({ ...card, draft: prepared.draft });
    } catch (error) { throw new Error(`${card.draft.faces[0].name}: ${error.message}`); }
  }
  // Card-scoped variants are stored on cards; avoid a conflicting global registry.
  return { ...value, customMechanics: definitions.filter(def=>!cards.some(card=>card.draft.customMechanics.some(local=>sameSetMechanic(def,local)))), customCards: cards };
}
