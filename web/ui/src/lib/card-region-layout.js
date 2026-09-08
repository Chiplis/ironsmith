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
      return {size:item.size,lineHeight,bounds:{x:Math.max(region.x,bounds.x),y:top,
        width:Math.min(width,region.x+region.width-Math.max(region.x,bounds.x)),
        height:Math.min(Math.max(height,region.y+region.height-top),region.y+region.height-top)}};
    }
    return { size: item.size, lineHeight, bounds: { x: bounds.x, width, y, height } };
  });
}
