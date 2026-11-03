import { sameSetMechanic, setMechanics, setSubtypes } from './set-workspace.js';

export const buildCardName = card => card.draft.faces.map(face => face.name).join(' // ');

// Both the set workspace and the cross-set library use these rows. Definitions
// remain attached to their source; browsing never installs or merges registries.
export function setCatalogRows(value, source = value.name) {
  const cards = value.customCards || [];
  return {
    cards: cards.map((card, index) => ({ name: buildCardName(card), source, index, card,
      detail: card.draft.faces.map(face => [face.name, [...(face.cardTypes || []), ...(face.subtypes || [])].join(' '), face.oracleText].filter(Boolean).join('\n')).join('\n\n') })),
    mechanics: setMechanics({ ...value, customCards: cards }).map((definition, index) => ({ name: definition.keyword, source, index, definition,
      detail: definition.reminder || definition.expansion || '',
      cards: cards.filter(card => card.draft.customMechanics?.some(local => sameSetMechanic(local, definition))).map(buildCardName) })),
    subtypes: setSubtypes({ ...value, customCards: cards }).map((definition, index) => ({ name: definition.singular, source, index, definition,
      detail: `${definition.plural} · ${definition.family}`,
      cards: cards.filter(card => card.draft.faces.some(face => face.subtypes?.some(name => name.toLowerCase() === definition.singular.toLowerCase()))).map(buildCardName) })),
  };
}

export function filterCatalogRows(rows, query) {
  const needle = query.trim().toLowerCase();
  return rows.filter(row => !needle || [row.name, row.source, row.detail, ...(row.cards || [])].join('\n').toLowerCase().includes(needle));
}
