import { FX } from "./keyword-fx/flags.js";

// Semantic colors are shared by keyword shields and impact rings.
export const KEYWORD_COLORS = {
  hexproof: '#58e99a', shroud: '#65cab5', ward: '#61d9cd',
  indestructible: '#ffe296', protection: '#fff1ce', flying: '#8edfff',
  reach: '#b2ce73', vigilance: '#fff6ad', lifelink: '#ff9ed2',
  deathtouch: '#bd83ec', menace: '#e46fba', haste: '#ffad57',
  trample: '#a5d36b', 'first strike': '#ffd080', 'double strike': '#ff926b',
};

function keywordLines(card) {
  return [...(card?.keywords || []), ...(card?.abilities?.length ? card.abilities : [card?.oracle_text || ""])]
    .flatMap(line => String(line).toLowerCase().split(/[,\n]/))
    .map(line => line.trim().replace(/\s*\(.*$/, '').replace(/\.$/, ''));
}

// Only standalone keyword lines; never match a grant mentioned in rules text.
export function cardKeywords(card) {
  const lines = keywordLines(card);
  return new Set(Object.keys(KEYWORD_COLORS)
    .filter(keyword => lines.some(line => line === keyword
      || (keyword === 'ward' && line.startsWith('ward '))
      || (keyword === 'protection' && line.startsWith('protection from ')))));
}

const PROTECTION_COLORS = {
  white: '#fff4c8', blue: '#7cc4ff', black: '#b88cff', red: '#ff7a5c', green: '#7be38a',
};

// The color a protection bubble takes: the single protected-from color when
// there is exactly one, otherwise the neutral protection tint.
export function protectionTint(card) {
  const found = new Set();
  for (const line of keywordLines(card)) {
    if (!line.startsWith('protection from ')) continue;
    for (const color of Object.keys(PROTECTION_COLORS)) if (line.includes(color)) found.add(color);
  }
  return found.size === 1 ? PROTECTION_COLORS[[...found][0]] : KEYWORD_COLORS.protection;
}

export function battlefieldKeywords(state) {
  const result = new Map();
  for (const player of state?.players || []) {
    for (const card of player.battlefield || []) {
      if (card.stable_id == null) continue;
      result.set(String(card.stable_id), cardKeywords(card));
    }
  }
  return result;
}

export function keywordGainEvents(previous, current) {
  const events = [];
  for (const [stableId, keywords] of current) {
    if (!previous.has(stableId)) continue;
    for (const keyword of keywords) {
      if (!previous.get(stableId).has(keyword)) events.push({
        type: 'keyword-gain', id: `keyword:${stableId}:${keyword}`, stableId,
        label: keyword, color: KEYWORD_COLORS[keyword],
      });
    }
  }
  return events;
}

const AURA_KEYWORD_BITS = {
  flying: FX.flying, hexproof: FX.hexproof, shroud: FX.shroud, ward: FX.ward,
  indestructible: FX.indestructible, protection: FX.protection,
};

// A well-mixed [0, 1) seed per object: neighbouring ids ("70", "71") must land
// far apart, or their animations run in lockstep.
export function auraSeed(key) {
  let h = 0x811c9dc5;
  for (const char of String(key)) h = Math.imul(h ^ char.charCodeAt(0), 0x01000193);
  h ^= h >>> 16; h = Math.imul(h, 0x85ebca6b);
  h ^= h >>> 13; h = Math.imul(h, 0xc2b2ae35);
  h ^= h >>> 16;
  return (h >>> 0) / 4294967296;
}

// What a battlefield card's keyword aura shows, or null when nothing.
export function keywordAuraSpec(card, combat) {
  if (!card || card.face_down) return null;
  let flags = 0;
  const keywords = cardKeywords(card);
  for (const keyword of keywords) flags |= AURA_KEYWORD_BITS[keyword] || 0;
  // Shroud already covers everything hexproof does, so its fog wins.
  if (flags & FX.shroud) flags &= ~FX.hexproof;
  const ids = new Set([card.id, ...(card.member_ids || [])].filter(id => id != null).map(String));
  if ((combat?.attackers || []).some(entry => ids.has(String(entry?.creature)))) flags |= FX.attacking;
  if ((combat?.blockers || []).some(entry => ids.has(String(entry?.blocker)))) flags |= FX.blocking;
  if (!flags) return null;
  return { flags, seed: auraSeed(card.stable_id ?? card.id ?? ""), protection: flags & FX.protection ? protectionTint(card) : null };
}
