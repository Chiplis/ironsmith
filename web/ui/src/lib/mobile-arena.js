import { cardArtColors, normalizeArtColors } from "./card-art-colors.js";

// Arena's battlefield uses art tiles rather than full printed cards. Classification
// follows live characteristics: animated lands and artifact creatures stay in combat.
export function arenaPermanentKind(card = {}) {
  const types = (card.card_types?.length ? card.card_types.join(' ') : card.type_line || card.lane || '').toLowerCase();
  if (/creature/.test(types) || card.power_toughness) return 'creature';
  if (/planeswalker/.test(types) || card.loyalty != null) return 'planeswalker';
  if (/battle/.test(types) || card.defense != null) return 'battle';
  if (/land/.test(types)) return 'land';
  if (/saga/i.test(card.type_line || '') || (/enchantment/.test(types) && /^(?:I|II|III|IV|V)\s*[—,]/m.test(card.oracle_text || ''))) return 'saga';
  if (/enchantment/.test(types)) return 'enchantment';
  if (/artifact/.test(types)) return 'artifact';
  return 'other';
}

export function partitionArenaBattlefield(cards = []) {
  const result = { frontCards: [], backCards: [], specialCards: [], supportCards: [] };
  for (const card of cards) {
    const kind = arenaPermanentKind(card);
    const destination = kind === 'creature' ? 'frontCards' : kind === 'land' ? 'backCards'
      : ['planeswalker', 'battle'].includes(kind) ? 'specialCards' : 'supportCards';
    result[destination].push(card);
  }
  return result;
}

export function arenaFrameColor(card = {}) {
  const liveColors = cardArtColors(card);
  const colors = liveColors.length ? liveColors : arenaPermanentKind(card) === 'land' ? normalizeArtColors(card.produced_mana) : [];
  if (colors.length > 1) return '#b4a26a';
  return ({ W: '#d6ccad', U: '#618ead', B: '#78717c', R: '#ac6655', G: '#63907a' })[String(colors[0]).toUpperCase()] || '#8f979d';
}
