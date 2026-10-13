// Match printing metadata, never card names. Package names and frame options
// are from the pinned Full Magic Pack. A match selects a treatment, not a claim
// of pixel-perfect reproduction of every printing using that treatment.
export const MSE_PRINTING_FIELDS = ['set', 'collector_number', 'frame', 'frame_effects',
  'border_color', 'full_art', 'promo_types', 'lang', 'released_at'];

const packageName = name => `${name}.mse-style`;
const choice = (name, styling = {}) => ({style: packageName(name), styling});

export function printingMseCardFields(snapshot, printing) {
  const metadata = printing ?? snapshot?.printing ?? snapshot ?? {};
  const border = metadata.border_color;
  // These names are native Magic game color choices. "borderless" is a style
  // option, not a color value, and must never be sent to the color parser.
  return ['black', 'white', 'silver', 'gold'].includes(border) ? {border_color: border} : {};
}

// These sets have a single identifiable showcase family for ordinary cards.
// Sets with multiple ambiguous showcase families deliberately stay unmapped.
const showcaseSets = {
  thb: 'theros-constellation', iko: 'ikoria-crystal', znr: 'zendikar',
  khm: 'kaldheim', dmu: 'dominaria-stained-glass',
  clb: 'monster-manual', who: 'doctor-who-tardis', pip: 'fallout-pipboy',
  mkm: 'ravnica-dossier', dsk: 'duskmourn-paranormal',
};

export function printingMseTreatment(snapshot, {printing, faceIndex = 0} = {}) {
  // Resolved metadata belongs to the displayed image and wins over cached card
  // metadata, including the absence of optional fields such as frame_effects.
  // The live snapshot still owns the card's text and layout.
  const metadata = printing ?? snapshot?.printing ?? snapshot ?? {};
  const face = snapshot?.card_faces?.[faceIndex] ?? snapshot;
  const type = String(face?.type_line ?? face?.type_line_display ?? '');
  const layout = snapshot?.layout ?? metadata.layout;
  const set = String(metadata.set ?? '').toLowerCase();
  const effects = new Set(Array.isArray(metadata.frame_effects) ? metadata.frame_effects : []);
  const frame = String(metadata.frame ?? '');
  const walker = /\bPlaneswalker\b/.test(type);
  const saga = /\bSaga\b/.test(type);
  // A decorative single-face package cannot replace a multipart layout. DFCs
  // also require linked-face support, which varies between package families.
  if (layout && !['normal', 'leveler'].includes(layout)) return null;
  if (/\bBattle\b/.test(type)) return null;
  if (walker || saga) {
    if (effects.has('showcase') || metadata.full_art || metadata.border_color === 'borderless'
      || effects.has('extendedart')) return null;
    if (['1993', '1997'].includes(frame)) return choice(walker ? 'magic-old-planeswalker' : 'magic-old-saga');
    if (walker && frame === '2003') return choice('magic-new-planeswalker');
    return null;
  }
  if (layout === 'leveler') return null;

  // Dedicated bonus-sheet set codes identify these frames without guessing a
  // collector-number range. Japanese-language Archives have multiple art
  // treatments, so language alone is insufficient to choose their template.
  if (set === 'mps') return choice('magic-m15-Kaladesh');
  if (set === 'mp2') return choice('magic-m15-invocation');
  if (set === 'exp') return choice('magic-m15-expedition');
  if (set === 'sta' && metadata.lang !== 'ja') return choice('magic-m15-showcase-strixhaven');
  if (effects.has('showcase') && Object.hasOwn(showcaseSets, set))
    return choice(`magic-m15-showcase-${showcaseSets[set]}`);

  // Unknown showcase/full-art designs must not be misidentified as ordinary
  // era frames. Generic borderless/extended-art modes remain usable, however.
  if (effects.has('showcase')) return null;
  if (metadata.border_color === 'borderless' || effects.has('borderless'))
    return choice('magic-m15-altered', {frames: 'borderless art'});
  if (effects.has('extendedart'))
    return choice('magic-m15-altered', {frames: 'frameless art'});
  if (metadata.full_art) return null;
  if (frame === 'future') return choice('magic-future');
  if (frame === '2003') return choice('magic-new');
  if (['1993', '1997'].includes(frame)) {
    if (['lea', 'leb', '2ed', 'ced', 'cei'].includes(set)) return choice('magic-old-abu');
    return choice('magic-old');
  }
  return null;
}
