import {cardGlintFace} from './card.mjs';
import {normalizeCardPresentation} from './printing-choices.mjs';

// Build drafts store type parts separately; the painter wants a printed line.
export function draftTypeLine(face) {
  const main = [...(face.supertypes ?? []), ...(face.cardTypes ?? [])].join(' ');
  const subtypes = (face.subtypes ?? []).join(' ');
  return main + (main && subtypes ? ' — ' : '') + subtypes;
}

function draftCardFace(face) {
  return {
    name: face.name || '',
    mana_cost: face.manaCost || '',
    type_line: draftTypeLine(face),
    // An empty indicator must not override colors implied by the mana cost.
    ...(face.colorIndicator?.length ? {color_indicator: face.colorIndicator} : {}),
    oracle_text: String(face.oracleText ?? ''),
    power: face.power || undefined,
    toughness: face.toughness || undefined,
    loyalty: face.loyalty === '' || face.loyalty == null ? undefined : String(face.loyalty),
    defense: face.defense === '' || face.defense == null ? undefined : String(face.defense),
    ...(face.artUrl ? {image_uris: {art_crop: face.artUrl}} : {}),
  };
}

// Paint the draft being edited. Split cards show both halves; other layouts show
// the selected face. A real card's printing supplies its set and default art.
export function cardGlintDraftFace(draft, faceIndex = 0, {printing, imageUrl} = {}) {
  const faces = (draft?.faces ?? []).map(draftCardFace);
  if (!faces.length) return null;
  const presentation = normalizeCardPresentation(draft.presentation);
  const art = face => face.image_uris?.art_crop ?? imageUrl;
  if (draft.layout === 'split' && faces.length > 1) {
    return cardGlintFace({layout: 'split', name: faces.map(face => face.name).join(' // '), card_faces: faces},
      {imageUrl: art(faces[0]), printing, presentation});
  }
  const face = faces[faceIndex] ?? faces[0];
  return cardGlintFace({...face, layout: draft.layout === 'transform_like' ? 'transform' : 'normal'},
    {imageUrl: art(face), printing, presentation});
}
