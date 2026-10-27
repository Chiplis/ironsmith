// Printing choices made in Build. They change only the painted card, never the rules.
export const CARD_GLINT_FINISHES = Object.freeze([
  {id:'gloss',label:'Shiny'},
  {id:'etched',label:'Etched'},
  {id:'prismatic',label:'Prismatic'},
  {id:'cold',label:'Cold foil'},
  {id:'opal',label:'Opal'},
  {id:'rain',label:'Rainlit window',tone:'silver'},
  {id:'heat',label:'Heat shimmer',tone:'gold'},
  {id:'pool',label:'Liquid prism',tone:'prismatic'},
  {id:'regular',label:'None'},
]);
// null keeps what the style or card already prints.
export const CARD_BORDER_COLORS = Object.freeze([
  {id:null,label:'Style default'},
  {id:'black',label:'Black'},
  {id:'white',label:'White'},
  {id:'silver',label:'Silver'},
]);
export const CARD_RARITIES = Object.freeze([
  {id:null,label:'Printed'},
  {id:'common',label:'Common'},
  {id:'uncommon',label:'Uncommon'},
  {id:'rare',label:'Rare'},
  {id:'mythic',label:'Mythic'},
]);
// Cardstock relief and irregular process-ink dots.
export const CARD_PRINT_EFFECTS = Object.freeze({
  cardstock:Object.freeze({amount:.2}),
  printDots:Object.freeze({amount:.15,period:3,variation:1,inset:.035}),
});
// "match" keeps the card's own printing, i.e. its current set.
export const DEFAULT_CARD_PRESENTATION = Object.freeze({
  finish:'gloss',printEffects:true,style:'match',frame:null,borderColor:null,rarity:null,
  artist:'',setCode:'',collectorNumber:'',
});
// Footer credits typed in Build. Empty keeps the printing's own credit. The
// set code is printed only; it never selects a set symbol or calibration.
export const CARD_CREDIT_LIMITS = Object.freeze({artist:60,setCode:6,collectorNumber:8});
const credit = (value,limit,pattern) => {
  const text = typeof value === 'string' ? value.replace(/\s+/g,' ').trim() : '';
  return (pattern ? text.replace(pattern,'') : text).slice(0,limit);
};

const pick = (options,value,fallback) => options.some(option => option.id === value) ? value : fallback;
const styleId = value => typeof value === 'string' && /^[-a-z0-9]{1,48}$/.test(value) ? value : null;
export function normalizeCardPresentation(value) {
  const base = DEFAULT_CARD_PRESENTATION;
  const style = styleId(value?.style) ?? base.style;
  return {
    finish: pick(CARD_GLINT_FINISHES,value?.finish,base.finish),
    printEffects: typeof value?.printEffects === 'boolean' ? value.printEffects : base.printEffects,
    style,
    frame: value?.style === style ? styleId(value?.frame) : null,
    borderColor: pick(CARD_BORDER_COLORS,value?.borderColor ?? null,null),
    rarity: pick(CARD_RARITIES,value?.rarity ?? null,null),
    artist: credit(value?.artist,CARD_CREDIT_LIMITS.artist),
    setCode: credit(value?.setCode,CARD_CREDIT_LIMITS.setCode,/[^A-Za-z0-9]/g).toUpperCase(),
    collectorNumber: credit(value?.collectorNumber,CARD_CREDIT_LIMITS.collectorNumber,/[^A-Za-z0-9★/-]/g),
  };
}

export const isDefaultCardPresentation = value =>
  JSON.stringify(normalizeCardPresentation(value)) === JSON.stringify(DEFAULT_CARD_PRESENTATION);

// The style request carried on a painted face; absent means the card's own printing.
export function cardPrintingChoice(value) {
  const {style,frame,borderColor} = normalizeCardPresentation(value);
  if (style === 'match' && !frame && !borderColor) return undefined;
  return {style,...(frame ? {frame} : {}),...(borderColor ? {borderColor} : {})};
}

// Opt into card-glint's automatic printing calibration (0.4.10+).
const calibration = api => api.printingCalibrationApiVersion === 1 ? {calibration:'auto'} : {};

// Resolve a face through card-glint's style catalog. Calibration is applied by
// the resolver, so faces without a choice resolve as "match". Unsupported
// styles or layouts keep the card's own printing.
export function presentCardGlintFace(api, face) {
  if (!face) return face;
  const {printingChoice,...source} = face;
  const request = printingChoice ?? (calibration(api).calibration ? {style:'match'} : null);
  const style = request && api.printingStyles?.find(candidate => candidate.id === request.style);
  if (!style || typeof api.resolvePrintingStyle !== 'function') return source;
  const overrides = {...calibration(api),...(request.borderColor ? {borderColor:request.borderColor} : {})};
  const choice = {...(request.frame ? {frame:request.frame} : {}),...(Object.keys(overrides).length ? {overrides} : {})};
  try { return api.resolvePrintingStyle(source,style,choice).face; }
  catch (error) {
    if (printingChoice) console.warn('[card-glint] Printing style unavailable for',source.name,error?.message);
    return source;
  }
}

// Choices belong to the card copies added from Build, keyed by stable ID. IDs
// restart each match and undo can rewind them, so entries are kept in memory,
// cleared when a match starts, and only apply while the name still matches.
const chosen = new Map();
// An addition waiting on a decision returns no ID; its choice binds to the
// first new copy with a matching name.
let pending = null;
const normalizeName = name => String(name || '').trim().toLowerCase();
const entry = (names,presentation,builderCreated = false) => ({builderCreated,names:new Set(names.map(normalizeName).filter(Boolean)),
  presentation:normalizeCardPresentation(presentation)});
const cardNames = card => [card.name,...(card.card_faces ?? []).map(face => face?.name)].map(normalizeName);

// Stable IDs already in a game snapshot, which a pending choice must skip.
export function snapshotStableIds(value,ids = new Set()) {
  if (Array.isArray(value)) for (const item of value) snapshotStableIds(item,ids);
  else if (value && typeof value === 'object') {
    if (value.stable_id != null) ids.add(String(value.stable_id));
    for (const child of Object.values(value)) if (child && typeof child === 'object') snapshotStableIds(child,ids);
  }
  return ids;
}

export function setCardPresentation(stableId, names, presentation, {existing,builderCreated = false} = {}) {
  pending = null;
  if (!presentation) return;
  if (stableId == null || String(stableId) === '0') pending = {...entry(names,presentation,builderCreated),existing:existing ?? new Set()};
  else chosen.set(String(stableId),entry(names,presentation,builderCreated));
}

function cardPresentationEntry(card) {
  if (card?.stable_id == null) return null;
  const id = String(card.stable_id), names = cardNames(card);
  if (!chosen.has(id) && pending && !pending.existing.has(id) && names.some(name => pending.names.has(name))) {
    chosen.set(id,{names:pending.names,presentation:pending.presentation,builderCreated:pending.builderCreated}); pending = null;
  }
  const found = chosen.get(id);
  return found && names.some(name => found.names.has(name)) ? found : null;
}

export const cardPresentation = card => cardPresentationEntry(card)?.presentation ?? null;
export const isBuilderCreatedCard = card => cardPresentationEntry(card)?.builderCreated === true;

export const clearCardPresentations = () => {chosen.clear(); pending = null;};
