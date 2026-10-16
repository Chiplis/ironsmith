import {cardArtCropUrl} from '../card-image-variants.js';

const rules = card => {
  const value = card.oracle_text ?? card.effect_text ?? card.ability_text ?? card.compiled_text ?? card.abilities ?? '';
  return Array.isArray(value) ? value.join('\n') : String(value);
};
const colors = value => {
  const names = {white:'W',blue:'U',black:'B',red:'R',green:'G'};
  const values = Array.isArray(value) ? value : String(value ?? '').split(/[,\s]+/);
  return [...new Set(values.map(v => names[String(v).toLowerCase()] ?? String(v).toUpperCase()).filter(v => /^[WUBRG]$/.test(v)))];
};

// Live engine fields take precedence over printing metadata. Printing only supplies decoration.
export function cardGlintFace(card, {imageUrl, faceIndex = 0, printing} = {}) {
  const print = printing ?? card.printing ?? card;
  const face = card.card_faces?.[faceIndex] ?? card;
  const type = face.type_line ?? card.type_line ?? '';
  const signature = String(face.characteristic_signature ?? '').match(/^colors:([^\n]*)/m);
  const cost = face.mana_cost ?? card.mana_cost ?? '';
  const palette = colors(signature ? signature[1] : face.colors ?? face.color_indicator ?? [...cost.matchAll(/[WUBRG]/g)].map(m => m[0]));
  const text = rules(face);
  const [power, toughness] = String(face.power_toughness ?? '').split('/');
  const result = {
    name: face.name ?? card.name ?? '',
    typeLine: face.type_line_display ?? type,
    artifact: /\bArtifact\b/.test(type), land: /\bLand\b/.test(type),
    creature: /\bCreature\b/.test(type), legendary: /\bLegendary\b/.test(type),
    colors: palette,
    manaSymbols: [...cost.matchAll(/\{([^{}]+)\}/g)].map(m => m[1]),
    rulesText: text,
    rulesParagraphs: [
      ...text.split('\n').filter(Boolean).map(text => ({role:'rules',runs:[{text}]})),
      ...(face.flavor_text ? [{role:'flavor',runs:[{text:face.flavor_text,italic:true}]}] : []),
    ],
    wordingMode: 'oracle', symbolStyle: 'modern',
    illustration: cardArtCropUrl(face.image_uris?.art_crop ?? imageUrl) || null,
    power: face.power ?? (power || undefined), toughness: face.toughness ?? toughness,
    loyalty: face.loyalty, defense: face.defense,
    setCode: print.set, rarity: print.rarity ?? card.rarity,
    artist: print.artist, collectorNumber: print.collector_number,
    borderColor: ['black','white','silver'].includes(print.border_color) ? print.border_color : undefined,
    securityStamp: print.security_stamp ?? 'none',
    frameStyle: print.frame ? (Number(print.frame) < 2003 ? 'retro' : 'modern') : undefined,
    layout: card.layout,
  };
  if (result.land && !palette.length) result.frameColors = colors([...(card.color_identity ?? card.colorIdentity ?? []), ...(card.intrinsic_mana ?? [])]);
  if (card.layout === 'flip') result.frameTemplate = 'flip';
  else if (/\bBattle\b/.test(type)) result.frameTemplate = 'battle';
  else if (print.frame === 'future') result.frameTemplate = 'future';
  if (['split','adventure','flip'].includes(card.layout) && card.card_faces?.length > 1) {
    result.faces = card.card_faces.map((part, index) => cardGlintFace({...part, printing:print}, {
      imageUrl:part.image_uris?.art_crop ?? imageUrl, printing:print, faceIndex:index,
    }));
  }
  return result;
}
