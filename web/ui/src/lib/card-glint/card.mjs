import {cardFlavorSource} from '../card-flavor-source.js';
import {cardDisplayTypeLine} from '../card-display-type-line.js';
import {cardArtCropUrl} from '../card-image-variants.js';
import {cardPresentation,cardPrintingChoice} from './printing-choices.mjs';

const rules = card => {
  const value = card.oracle_text ?? card.effect_text ?? card.ability_text ?? card.compiled_text ?? card.abilities ?? '';
  return Array.isArray(value) ? value.join('\n') : String(value);
};
const colors = value => {
  const names = {white:'W',blue:'U',black:'B',red:'R',green:'G'};
  const values = Array.isArray(value) ? value : String(value ?? '').split(/[,\s]+/);
  return [...new Set(values.map(v => names[String(v).toLowerCase()] ?? String(v).toUpperCase()).filter(v => /^[WUBRG]$/.test(v)))];
};

// Missing printings use the standard modern component frames. Unlike scanned
// frame plates, these retain color-pair pinlines, legend crowns and special
// layouts without borrowing another card's printing or artwork.
export const CARD_GLINT_DEFAULT_FRAME = Object.freeze({
  frameStyle: 'modern', borderColor: 'black', securityStamp: 'none',
  wordingMode: 'oracle', symbolStyle: 'modern',
});

function cardTypeLine(face, card) {
  if (face.type_line ?? card.type_line) return face.type_line ?? card.type_line;
  return cardDisplayTypeLine({...card,...face,type_line_display:undefined});
}

// Live engine fields take precedence over printing metadata. Printing only supplies decoration.
// Cards added from Build carry their own printing choices.
export function cardGlintFace(card, {imageUrl, faceIndex = 0, printing, presentation} = {}) {
  const print = printing ?? card.printing ?? card;
  const face = card.card_faces?.[faceIndex] ?? card;
  const type = cardTypeLine(face, card);
  const signature = String(face.characteristic_signature ?? '').match(/^colors:([^\n]*)/m);
  const cost = face.mana_cost ?? card.mana_cost ?? '';
  const palette = colors(signature ? signature[1] : face.colors ?? face.color_indicator ?? [...cost.matchAll(/[WUBRG]/g)].map(m => m[0]));
  const text = rules(face);
  // Imported custom-card catalogs may store a source URL as flavor text.
  // Attribution links are shown below the inspector and can overflow the painter's
  // unbroken-word layout, rejecting an otherwise valid default frame.
  const flavor = String(face.flavor_text ?? print.flavor_text ?? '').trim();
  const printableFlavor = cardFlavorSource(flavor) ? '' : flavor;
  const [power, toughness] = String(face.power_toughness ?? '').split('/');
  const result = {
    ...CARD_GLINT_DEFAULT_FRAME,
    name: face.name ?? card.name ?? '',
    typeLine: face.type_line_display ?? type,
    artifact: /\bArtifact\b/.test(type), land: /\bLand\b/.test(type),
    creature: /\bCreature\b/.test(type), legendary: /\bLegendary\b/.test(type),
    colors: palette,
    manaSymbols: [...cost.matchAll(/\{([^{}]+)\}/g)].map(m => m[1]),
    rulesText: text,
    rulesParagraphs: [
      ...text.split('\n').filter(Boolean).map(text => ({role:'rules',runs:[{text}]})),
      ...(printableFlavor ? [{role:'flavor',runs:[{text:printableFlavor,italic:true}]}] : []),
    ],
    illustration: cardArtCropUrl(face.image_uris?.art_crop ?? imageUrl) || null,
    power: face.power ?? (power || undefined), toughness: face.toughness ?? toughness,
    loyalty: face.loyalty, defense: face.defense,
    setCode: print.set, rarity: print.rarity ?? card.rarity,
    artist: print.artist, collectorNumber: print.collector_number,
    borderColor: ['black','white','silver'].includes(print.border_color) ? print.border_color : CARD_GLINT_DEFAULT_FRAME.borderColor,
    securityStamp: print.security_stamp ?? 'none',
    frameStyle: print.frame ? (Number(print.frame) < 2003 ? 'retro' : 'modern') : CARD_GLINT_DEFAULT_FRAME.frameStyle,
    layout: face.layout ?? card.layout ?? 'normal',
  };
  const chosen = presentation === undefined ? cardPresentation(card) : presentation;
  if (chosen?.rarity) result.rarity = chosen.rarity;
  if (result.land && !palette.length) result.frameColors = colors([...(card.color_identity ?? card.colorIdentity ?? []), ...(card.intrinsic_mana ?? [])]);
  if (result.layout === 'flip') result.frameTemplate = 'flip';
  else if (/\bBattle\b/.test(type)) result.frameTemplate = 'battle';
  else if (print.frame === 'future') result.frameTemplate = 'future';
  if (['split','adventure','flip'].includes(result.layout) && card.card_faces?.length > 1) {
    result.faces = card.card_faces.map((part, index) => cardGlintFace({...part, printing:print}, {
      imageUrl:part.image_uris?.art_crop ?? imageUrl, printing:print, faceIndex:index, presentation:chosen && {rarity:chosen.rarity},
    }));
  }
  // The painter resolves this with card-glint's style catalog.
  const printingChoice = cardPrintingChoice(chosen);
  if (printingChoice) result.printingChoice = printingChoice;
  return result;
}
