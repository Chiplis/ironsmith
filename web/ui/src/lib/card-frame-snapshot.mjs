import {isBuilderCreatedCard} from './card-glint/printing-choices.mjs';
import {printableCardOrder} from './card-print-order.mjs';
import {MSE_PRINTING_FIELDS} from './mse/printing-style.mjs';
import {cardIntrinsicManaColors,withoutIntrinsicManaLines} from './land-mana-choices.js';

const fields = ['id','stable_id','name','source_kind','source_ability_text','oracle_id','oracleId','oracle_text','flavor_text','rarity',
  'card_types','supertypes','subtypes','effect_text','ability_text','compiled_text','abilities','mana_cost','display_mana_cost','type_line','type_line_display',
  'security_stamp','layout','color_identity','colorIdentity','color_indicator','card_faces','colors','characteristic_signature',
  'keywords','power','toughness','power_toughness','loyalty','defense','printing','mse_style',...MSE_PRINTING_FIELDS];

// Presentation never serializes the game object, action callbacks, or caches.
export function cardFrameSnapshot(card) {
  const snapshot = Object.fromEntries(fields.map(key => [key,card?.[key]]));
  // Intrinsic basic-land mana is never printed; the mana strip uses it.
  if (typeof snapshot.oracle_text === 'string') {
    const lines = snapshot.oracle_text.split('\n');
    const kept = withoutIntrinsicManaLines(lines, cardIntrinsicManaColors(card));
    if (kept.length !== lines.length) snapshot.oracle_text = kept.join('\n');
  }
  // A land's frame follows the mana it makes, so a land that an effect such as
  // Yavimaya turned into a Forest renders green even with a colorless identity.
  const intrinsic = cardIntrinsicManaColors(card);
  if (intrinsic.length) snapshot.intrinsic_mana = intrinsic;
  return JSON.stringify(isBuilderCreatedCard(card) ? printableCardOrder(snapshot) : snapshot);
}

export function cardFrameFaceIndex(card, imageUrl) {
  const faces=card?.card_faces;
  if(!Array.isArray(faces)||faces.length<2||['split','aftermath','adventure','prepare','room','flip'].includes(card.layout))return 0;
  if(/\/back\//.test(String(imageUrl)))return 1;
  const index=faces.findIndex(face=>face.name===card.name);
  return Math.max(0,index);
}

export function cardFrameRules(card) {
  const value=card?.oracle_text??card?.effect_text??card?.ability_text??card?.compiled_text??card?.abilities??'';
  return Array.isArray(value)?value.join('\n'):String(value);
}
