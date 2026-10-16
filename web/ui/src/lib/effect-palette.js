// Semantic colors are shared by keyword shields and impact rings.
export const KEYWORD_COLORS = {
  hexproof: '#58e99a', shroud: '#65cab5', ward: '#61d9cd',
  indestructible: '#ffe296', protection: '#fff1ce', flying: '#8edfff',
  reach: '#b2ce73', vigilance: '#fff6ad', lifelink: '#ff9ed2',
  deathtouch: '#bd83ec', menace: '#e46fba', haste: '#ffad57',
  trample: '#a5d36b', 'first strike': '#ffd080', 'double strike': '#ff926b',
};

export function battlefieldKeywords(state) {
  const result = new Map();
  for (const player of state?.players || []) {
    for (const card of player.battlefield || []) {
      if (card.stable_id == null) continue;
      // Only standalone keyword lines; never match a grant mentioned in rules text.
      const lines = [...(card.keywords || []), ...(card.abilities?.length ? card.abilities : [card.oracle_text || ""])]
        .flatMap(line => String(line).toLowerCase().split(/[,\n]/))
        .map(line => line.trim().replace(/\s*\(.*$/, '').replace(/\.$/, ''));
      result.set(String(card.stable_id), new Set(Object.keys(KEYWORD_COLORS)
        .filter(keyword => lines.some(line => line === keyword || (keyword === 'ward' && line.startsWith('ward '))))));
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
