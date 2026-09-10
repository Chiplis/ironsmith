import {normalizeAbilityMatchText} from './inspector-ability-lines.js';

const words = text => normalizeAbilityMatchText(text).split(' ').filter(Boolean);
export function regionTextScore(a, b) {
  const aa=words(a),bb=words(b);
  if (!aa.length || !bb.length) return 0;
  if (aa.join(' ')===bb.join(' ')) return 1;
  const counts=new Map();for(const word of bb)counts.set(word,(counts.get(word)||0)+1);
  let matched=0;
  for(const word of aa)if(counts.get(word)){matched++;counts.set(word,counts.get(word)-1);}
  return 2*matched/(aa.length+bb.length);
}

const scanPath = url => { try { return new URL(url, 'https://cards.scryfall.io').pathname.split('/').slice(-4).join('/'); } catch { return ''; } };
const scanFace = url => /\/back\//.test(String(url || '')) ? 'back' : 'front';

export function registrationForImage(registrations, url) {
  if (!url) return null;
  const path = scanPath(url);
  return registrations.find(item => scanPath(item.source) === path) || null;
}

// The renderer lays out horizontal text. Rotated OCR boxes and ability boxes
// that swallow a header cannot safely drive either masking or replacement.
export function registrationGeometryIsUsable(registration) {
  const fields = (registration?.fields || []).filter(field => !field.unprinted && field.bounds);
  const headers = fields.filter(field => ['name', 'type'].includes(field.kind));
  if (headers.some(({bounds}) => bounds.height > bounds.width)) return false;
  for (const {bounds: rule} of fields.filter(field => field.kind === 'rule')) {
    for (const {bounds: header} of headers) {
      const width = Math.max(0, Math.min(rule.x + rule.width, header.x + header.width) - Math.max(rule.x, header.x));
      const height = Math.max(0, Math.min(rule.y + rule.height, header.y + header.height) - Math.max(rule.y, header.y));
      if (width * height > header.width * header.height * .5) return false;
    }
  }
  return true;
}

// A registration describes the ink on one scan. Another language of the same
// printing (same set and collector number) shares the art and frame but wraps
// its text differently, so it cannot reuse the line boxes; it can borrow the
// registered scan and lay its own translated text over it.
export function registrationForPrinting(registrations, printing, url = '') {
  const set = String(printing?.set || '').toLowerCase();
  const number = String(printing?.collector_number || '').toLowerCase();
  if (!set || !number) return null;
  const face = scanFace(url);
  return registrations.find(item => String(item.set || '').toLowerCase() === set
    && String(item.collector_number || '').toLowerCase() === number
    && scanFace(item.source) === face) || null;
}

// Assign by canonical text, not display language or filtered action-array index.
export function registeredRuleAssignments(fields, rulesView) {
  const candidates=fields.map((field,index)=>({field,index})).filter(({field})=>field.kind==='rule');
  const assignments=new Map();
  for(const [index,line] of rulesView.lines.entries()) {
    const source=(rulesView.sourceLines?.[index]||[line]).join(' ');
    let best=null;
    for(const candidate of candidates) {
      const score=regionTextScore(source,candidate.field.text);
      if(!best || score>best.score || (score===best.score && assignments.has(best.index) && !assignments.has(candidate.index)))best={...candidate,score};
    }
    if(best && best.score>=.2) {
      const current=assignments.get(best.index)||[];
      current.push(index);assignments.set(best.index,current);
    }
  }
  return assignments;
}

// Scryfall "normal" scans are 488x680; registrations store fractions of them.
export const SCAN_ASPECT = 680 / 488;
const median = values => { const sorted = [...values].sort((a, b) => a - b); return sorted.length ? sorted[Math.floor(sorted.length / 2)] : null; };
const letterCount = text => (String(text || '').match(/[\p{L}\p{N}]/gu) || []).length;

// Printed type size, as a fraction of the scan width, from the loaded face's
// advance width against each registered line's box. Whole-line widths are
// stable; box heights change from line to line with ascenders, descenders and
// OCR padding, so they only size lines that are mostly symbols.
export function registeredFieldFontSize(field, measure) {
  const byWidth = [], byHeight = [];
  let parenthetical = 0;
  for (const line of field.lines || []) {
    const text = String(line.text || '').trim();
    // Reminder text is set in italics, which run narrower than roman type.
    const italic = parenthetical > 0 || text.startsWith('(');
    parenthetical = Math.max(0, parenthetical + (text.match(/\(/g) || []).length - (text.match(/\)/g) || []).length);
    const letters = letterCount(text);
    if (letters < 3 || !line.width || !line.height) continue;
    const metrics = measure(text, italic);
    if (!metrics?.width) continue;
    if (letters >= 8) byWidth.push(line.width / (metrics.width / 100));
    else if (metrics.height) byHeight.push(line.height * SCAN_ASPECT / (metrics.height / 100));
  }
  return median(byWidth) ?? median(byHeight);
}

// Baseline pitch between consecutive registered lines, as a fraction of the scan height.
export function registeredLinePitch(field) {
  const tops = (field.lines || []).map(line => line.y).sort((a, b) => a - b);
  return median(tops.slice(1).map((y, index) => y - tops[index]).filter(gap => gap > 0));
}

// Field typography and the box its lines need. OCR bounds hug the ink, so a
// box centred on them keeps the replacement on the printed baseline. Printed
// pitch is tighter than the face's ascent plus descent, so the box also holds
// the first and last lines' full content area or their extremes would clip.
export function registeredFieldLayouts(fields, measureFor, { fallbackLineHeight = 1.2 } = {}) {
  const sized = fields.map(field => {
    if (!field.bounds) return null;
    const measure = measureFor(field.kind);
    const lines = Math.max(1, (field.lines || []).length);
    const measured = registeredFieldFontSize(field, measure);
    const size = measured || field.bounds.height / lines * SCAN_ASPECT / 1.05;
    const pitch = registeredLinePitch(field);
    const ratio = pitch ? pitch * SCAN_ASPECT / size : null;
    const content = (measure('x')?.content || 0) / 100;
    return { field, lines, size, content, lineHeight: ratio >= .9 && ratio <= 1.6 ? ratio : null };
  });
  const shared = median(sized.filter(item => item?.lineHeight && ['rule', 'flavor'].includes(item.field.kind)).map(item => item.lineHeight)) ?? fallbackLineHeight;
  // Translations outgrow the printed ink. Names may run to the mana cost, type
  // lines to the set symbol, and the last paragraph down to the flavor text,
  // stats or the foot of the text box.
  const bottomOf = kind => Math.min(...fields.filter(f => f.kind === kind && f.bounds).map(f => f.bounds.y));
  const rules = sized.filter(item => item?.field.kind === 'rule');
  const lastRule = rules.length ? rules.reduce((a, b) => b.field.bounds.y > a.field.bounds.y ? b : a) : null;
  // Short keyword lines share the paragraph column with the longest lines.
  const columnRight = Math.max(...fields.filter(f => ['rule', 'flavor'].includes(f.kind) && f.bounds).map(f => f.bounds.x + f.bounds.width));
  return sized.map(item => {
    if (!item) return null;
    const lineHeight = item.lineHeight ?? shared;
    const { bounds } = item.field;
    const span = ((item.lines - 1) * lineHeight + Math.max(lineHeight, item.content)) * item.size / SCAN_ASPECT;
    let height = Math.max(bounds.height, span);
    const y = bounds.y + bounds.height / 2 - height / 2;
    let width = bounds.width;
    if (item.field.kind === 'name') width = Math.max(width, (item.field.limit ?? .8) - .012 - bounds.x);
    if (item.field.kind === 'type') width = Math.max(width, .84 - bounds.x);
    if (item.field.kind === 'rule' && Number.isFinite(columnRight)) width = Math.max(width, columnRight - bounds.x);
    if (item === lastRule) {
      const below = [bottomOf('flavor'), bottomOf('stats')].filter(limit => limit > bounds.y + bounds.height);
      height = Math.max(height, Math.min(...below, .875) - .006 - y);
    }
    const region = item.field.region;
    if (region) {
      const top = Math.max(region.y, y);
      return {size:item.size,lineHeight,span,bounds:{x:Math.max(region.x,bounds.x),y:top,
        width:Math.min(width,region.x+region.width-Math.max(region.x,bounds.x)),
        height:Math.min(Math.max(height,region.y+region.height-top),region.y+region.height-top)}};
    }
    return { size: item.size, lineHeight, span, bounds: { x: bounds.x, width, y, height } };
  });
}

// Lay a text box's replaced paragraphs out as one column at the printed type
// size. Each paragraph keeps its printed top unless the one above it grew, in
// which case it moves down by no less than `minGap`. When the column overruns
// `limit`, the printed gaps between paragraphs give up their slack first and
// only then does `shrink` (< 1) ask for smaller type. `displaced` says the
// column no longer matches the registered ink, so every printed paragraph in
// it has to be masked or the moved text would land on printed lettering.
// Items are {index, top, footprint, natural|null} sorted by top; every measure
// is a fraction of the scan height.
export function registeredColumnFlow(items, { limit, minGap, tolerance = 0 }) {
  if (!items.length) return { positions: new Map(), shrink: 1, displaced: false };
  const heights = items.map(item => item.natural ?? item.footprint);
  const tops = [];
  let cursor = -Infinity;
  items.forEach((item, i) => {
    const top = i ? Math.max(item.top, cursor + minGap) : item.top;
    tops.push(top);
    cursor = top + heights[i];
  });
  let shrink = 1;
  if (cursor > limit + tolerance) {
    const overflow = cursor - limit;
    const slack = tops.map((top, i) => i ? Math.max(0, top - (tops[i - 1] + heights[i - 1]) - minGap) : 0);
    const total = slack.reduce((sum, value) => sum + value, 0);
    const consumed = total > 0 ? Math.min(1, overflow / total) : 0;
    for (let i = 1; i < tops.length; i++) tops[i] = tops[i - 1] + heights[i - 1] + minGap + slack[i] * (1 - consumed);
    const bottom = tops[tops.length - 1] + heights[heights.length - 1];
    if (bottom > limit + tolerance && bottom > tops[0]) shrink = Math.max(0, (limit - tops[0]) / (bottom - tops[0]));
  }
  const displaced = items.some((item, i) => Math.abs(tops[i] - item.top) > tolerance);
  return {
    positions: new Map(items.map((item, i) => [item.index, { top: tops[i], bottom: tops[i] + heights[i] }])),
    shrink,
    displaced,
  };
}

const FLOWING_KINDS = ['rule', 'flavor'];
// The text box of each face as one column: where every rules and flavor
// paragraph starts at the printed type size, the room the column has, and
// whether the printed layout still holds. Measures are fractions of the scan
// height; `measured` holds the natural text heights fields have reported.
export function registeredColumns(fields,layouts,texts,measured,{unit,scale}) {
  const height=unit*SCAN_ASPECT;
  if(!height)return null;
  const tolerance=1.5/height;
  const positions=new Map(),forced=new Set();
  let shrink=1;
  const faces=[...new Set(fields.filter(f=>FLOWING_KINDS.includes(f.kind)&&f.bounds).map(f=>f.face))];
  for(const face of faces) {
    const indices=fields.map((field,index)=>index).filter(index=>{const f=fields[index];return FLOWING_KINDS.includes(f.kind)&&f.bounds&&f.face===face&&layouts[index];})
      .sort((a,b)=>layouts[a].bounds.y-layouts[b].bounds.y);
    if(!indices.length)continue;
    // Only a plain column flows: paragraphs stacked over one another. Level
    // bands, split faces and boxes set beside the text keep their registered
    // places and the per-field fitter.
    const widest=indices.map(index=>fields[index].bounds).reduce((a,b)=>b.width>a.width?b:a);
    const plain=indices.every(index=>{
      const b=fields[index].bounds;
      const overlap=Math.min(b.x+b.width,widest.x+widest.width)-Math.max(b.x,widest.x);
      return overlap>=Math.min(b.width,widest.width)*.6;
    });
    if(!plain)continue;
    const first=layouts[indices[0]].bounds.y;
    const regions=indices.map(index=>fields[index].region).filter(Boolean);
    const statsTop=Math.min(...fields.filter(f=>f.kind==='stats'&&f.bounds&&f.face===face).map(f=>f.bounds.y));
    const limit=regions.length?Math.min(...regions.map(r=>r.y+r.height)):Math.min(statsTop>first?statsTop:1,.875)-.006;
    const pitch=median(indices.map(index=>layouts[index].lineHeight*layouts[index].size/SCAN_ASPECT).filter(Boolean))||.03;
    // Footprints are the printed paragraphs' line boxes, measured the same way
    // the browser reports the replacement text, so a translation with the
    // printed line count lands exactly on the printed ink.
    const items=indices.map(index=>{
      const field=fields[index],layout=layouts[index];
      const report=measured.get(index);
      const natural=report&&report.unit===unit&&report.scale===scale&&report.text===texts[index]?report.px/height:null;
      const footprint=layout.span??Math.max(0,field.bounds.y+field.bounds.height-layout.bounds.y);
      return {index,top:layout.bounds.y,footprint,natural};
    });
    // Paragraphs may close up to the tightest gap the printing itself used.
    const printedGaps=items.slice(1).map((item,i)=>item.top-(items[i].top+items[i].footprint)).filter(gap=>gap>0);
    const minGap=Math.min(pitch*.3,...printedGaps);
    // The printed ink itself never overruns its box: the column reaches at
    // least as far as the lowest registered line.
    const floor=Math.max(limit,...items.map(item=>item.top+item.footprint));
    const flow=registeredColumnFlow(items,{limit:floor,minGap,tolerance});
    for(const [index,place] of flow.positions)positions.set(index,{top:place.top,bottom:place.bottom,limit:floor,footprint:items.find(item=>item.index===index).footprint});
    if(flow.displaced)forced.add(face);
    // Smaller type is a last resort, and only once every paragraph has reported
    // its height at the current scale; a stale or missing measurement would
    // otherwise ratchet the shared scale down one notch per render.
    if(items.every(item=>item.natural!=null))shrink=Math.min(shrink,flow.shrink);
  }
  return {positions,forced,shrink};
}


// OCR sometimes runs a name into the generic mana digit beside it ("Yawgmoth,
// Thran Physician 2"), so the name's box covers the digit and the mask erases
// it. Trim such a line back to the card name by the face's advance widths and
// stop translated names where the cost begins.
const MANA_SUFFIX = /^(?:\s*(?:\d+|[xyzwubrgcsp]|\{[^}]*\}))+\s*$/i;
const normalizeLine = text => String(text || '').normalize('NFKC').replace(/\s+/g, ' ').trim();
export function trimRegisteredNameCosts(fields, measure) {
  return fields.map(field => {
    if (field.kind !== 'name' || field.lines?.length !== 1 || !field.bounds) return field;
    const line = field.lines[0];
    const lineText = normalizeLine(line.text), name = normalizeLine(field.text);
    if (!name || lineText.length <= name.length || !lineText.toLowerCase().startsWith(name.toLowerCase())) return field;
    if (!MANA_SUFFIX.test(lineText.slice(name.length))) return field;
    const full = measure(lineText)?.width, kept = measure(name)?.width;
    if (!full || !kept || kept >= full) return field;
    // The OCR box ends at the last mana symbol. Printed pips are discs about
    // .7 of the line height wide (23px in a 34px line on the DMR Yawgmoth
    // scan), set a quarter line after the name, so the name ends well before
    // the digit glyph's advance would suggest; a box that reached the disc
    // would let the mask nibble the first pip, one that stopped short would
    // leave the last letter's edge on the card.
    const pips = lineText.slice(name.length).match(/\d+|[a-z]|\{[^}]*\}/gi)?.length || 1;
    const disc = line.height * SCAN_ASPECT;
    const bySymbols = line.width - pips * disc * .7 - disc * .25;
    const width = Math.max(0, Math.min(line.width * kept / full, bySymbols));
    const limit = line.x + width + disc * .2;
    return {
      ...field,
      limit: Math.min(field.limit ?? 1, limit),
      lines: [{...line, text: name, width}],
      bounds: {...field.bounds, width: Math.min(field.bounds.width, line.x + width - field.bounds.x)},
    };
  });
}
