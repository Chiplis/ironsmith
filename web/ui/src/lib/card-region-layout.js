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

export function registrationForImage(registrations, url) {
  if (!url) return null;
  const image = new URL(url,'https://cards.scryfall.io');
  return registrations.find(item=>{
    const source=new URL(item.source);
    return image.pathname.split('/').slice(-4).join('/')===source.pathname.split('/').slice(-4).join('/');
  }) || null;
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
  return sized.map(item => {
    if (!item) return null;
    const lineHeight = item.lineHeight ?? shared;
    const { bounds } = item.field;
    const span = ((item.lines - 1) * lineHeight + Math.max(lineHeight, item.content)) * item.size / SCAN_ASPECT;
    const height = Math.max(bounds.height, span);
    return { size: item.size, lineHeight, bounds: { x: bounds.x, width: bounds.width, y: bounds.y + bounds.height / 2 - height / 2, height } };
  });
}
