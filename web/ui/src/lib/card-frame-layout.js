// These layouts contain independently positioned live text/cost/stat regions.
// Until those regions have their own registered masks, retain the source scan
// and expose live actions in the details disclosure. Never flatten their panels
// into the conventional single rules box just because an art crop matches.
const segmentedLayouts = new Set(['split','flip','adventure','prepare','case','saga','class','leveler','prototype','mutate','planar','scheme','vanguard','augment','art_series']);
export function sourceMaskLayoutGap(printing) {
  if (!printing) return 'printing-metadata';
  if (segmentedLayouts.has(printing.layout)) return `layout-${printing.layout}`;
  if (printing.frame_effects?.includes('shatteredglass')) return 'shattered-glass-material';
  const type = printing.type_line || '';
  if (/\bPlaneswalker\b/.test(type)) return 'loyalty-panels';
  if (/\bBattle\b/.test(type)) return 'horizontal-battle';
  if (/\bSaga\b/.test(type)) return 'layout-saga';
  if (/\bDungeon\b/.test(type)) return 'dungeon-map';
  if (/\bAttraction\b/.test(type)) return 'attraction-lights';
  if (/\bContraption\b/.test(type)) return 'contraption-panels';
  if (printing.keywords?.includes('Station')) return 'station-ranks';
  // Invocations have their own lettering and ornamental text enclosures;
  // Scryfall labels these as otherwise-normal 2015 frames without an effect.
  if (printing.set === 'mp2') return 'invocation-typography';
  return null;
}
