const COLOR_CODES = { white: 'W', blue: 'U', black: 'B', red: 'R', green: 'G', colorless: 'C' };

// Local catalog Scryfall metadata is deliberately sparse. The canonical
// name and printed characteristics live in the compiler's source blocks.
export function localCardBuildSource(payload) {
  const group = payload?.group;
  if (!group) return null;
  const metadata = payload.scryfall || {};
  const sources = group.faces?.length ? group.faces : [group];
  const faces = sources.map(source => {
    const faceMeta = metadata.faces?.find(face => face.name === source.name) || (sources.length === 1 ? metadata : {});
    const face = { ...faceMeta, name: source.name || payload.canonicalName };
    const lines = String(source.block || '').split('\n');
    let i = 0;
    for (; i < lines.length; i++) {
      const match = /^(Mana cost|Type|Color indicator|Power\/Toughness|Loyalty|Defense|First printed set):\s*(.*)$/.exec(lines[i]);
      if (!match) break;
      const [, key, value] = match;
      if (key === 'Mana cost') face.mana_cost = value;
      if (key === 'Type') face.type_line = value;
      if (key === 'Color indicator') face.color_indicator = value.split(/[,\s]+/).filter(Boolean).map(color => COLOR_CODES[color.toLowerCase()] || color);
      if (key === 'Power/Toughness') [face.power, face.toughness] = value.split('/').map(stat => stat.trim());
      if (key === 'Loyalty') face.loyalty = value;
      if (key === 'Defense') face.defense = value;
    }
    face.oracle_text = lines.slice(i).join('\n');
    return face;
  });
  return sources.length === 1 ? faces[0] : {
    ...metadata, name: group.combinedName || payload.canonicalName,
    layout: group.layout, keywords: group.hasFuse ? ['Fuse'] : [], card_faces: faces,
  };
}

export function cardBuildDraft(card) {
  const faces = card?.card_faces?.length ? card.card_faces : [card];
  const supertypes = new Set(['Legendary', 'Basic', 'Snow', 'World', 'Ongoing']);
  return {
    layout: faces.length === 1 ? 'single' : card.layout === 'split' ? 'split' : 'transform_like',
    hasFuse: (card.keywords || []).includes('Fuse'),
    faces: faces.map(face => {
      if (!face?.name || !face?.type_line) throw new Error('Card source is missing its name or type');
      const [types, subtypes = ''] = face.type_line.split(/\s+[—–]\s+/);
      const words = types.split(/\s+/).filter(Boolean);
      return {
        name: face.name, manaCost: face.mana_cost || '',
        supertypes: words.filter(word => supertypes.has(word)),
        cardTypes: words.filter(word => !supertypes.has(word)),
        subtypes: subtypes.split(/\s+/).filter(Boolean),
        colorIndicator: face.color_indicator || [], oracleText: face.oracle_text || '',
        power: String(face.power ?? ''), toughness: String(face.toughness ?? ''),
        loyalty: String(face.loyalty ?? ''), defense: String(face.defense ?? ''),
        artUrl: face.image_uris?.art_crop || card.image_uris?.art_crop || '',
      };
    }),
  };
}
