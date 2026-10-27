import {printedTypeLine} from '../card-print-order.mjs';
import {cardGlintFace} from './card.mjs';
import {normalizeCardPresentation} from './printing-choices.mjs';

// Build drafts store type parts separately; the painter wants a printed line.
export function draftTypeLine(face) {
  const main = [...(face.supertypes ?? []), ...(face.cardTypes ?? [])].join(' ');
  const subtypes = (face.subtypes ?? []).join(' ');
  return printedTypeLine(main + (main && subtypes ? ' — ' : '') + subtypes);
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

// Paint the draft being edited. Shared-face cards show both panels; other layouts show
// the selected face. A real card's printing supplies its set and default art;
// a new card has none (null).
export function cardGlintDraftFace(draft, faceIndex = 0, printingSource = null) {
  const {printing, imageUrl} = printingSource ?? {};
  const faces = (draft?.faces ?? []).map(draftCardFace);
  if (!faces.length) return null;
  const presentation = normalizeCardPresentation(draft.presentation);
  const art = face => face.image_uris?.art_crop ?? imageUrl;
  const layout = faces.some(face => /\bAdventure\b/.test(face.type_line)) ? 'adventure' : draft.frameLayout ?? draft.layout;
  if (['split', 'aftermath', 'adventure', 'prepare', 'room', 'flip'].includes(layout) && faces.length > 1) {
    return cardGlintFace({layout, keywords: draft.hasFuse ? ['Fuse'] : [], name: faces.map(face => face.name).join(' // '), card_faces: faces},
      {imageUrl: art(faces[0]), printing, presentation, builderCreated:true});
  }
  const face = faces[faceIndex] ?? faces[0];
  return cardGlintFace({card_faces: faces, layout: layout === 'transform_like' ? 'transform' : layout === 'single' ? 'normal' : layout},
    {imageUrl: art(face), faceIndex, printing, presentation, builderCreated:true});
}
