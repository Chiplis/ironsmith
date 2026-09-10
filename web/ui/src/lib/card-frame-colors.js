import { sourceMaskLayoutGap } from './card-frame-layout.js';
import {manaTemplates,locateManaSymbols} from './card-mana-match.js';
import { locateSetSymbol } from './card-set-symbol.js';
import { fontGuidedPanel } from './card-frame-font-mask.js';
import { maskSourceFrame } from './card-frame-source.js';

// Printing materials, source panel reconstruction, and frame geometry.
// Panel reconstruction is independent of the typography selection.
const cache = new Map();

export function fullCardImageUrl(artUrl) {
  return /^https:\/\/cards\.scryfall\.io\/art_crop\//.test(artUrl || '')
    ? artUrl.replace('/art_crop/', '/normal/') : '';
}

export function materialColor(data) {
  const bins = new Map();
  for (let i = 0; i < data.length; i += 4) {
    if (data[i + 3] < 128) continue;
    const key = [data[i], data[i + 1], data[i + 2]].map(v => Math.floor(v / 32)).join(',');
    const bin = bins.get(key) || { count: 0, rgb: [0, 0, 0] };
    bin.count++;
    for (let c = 0; c < 3; c++) bin.rgb[c] += data[i + c];
    bins.set(key, bin);
  }
  const bin = [...bins.values()].sort((a, b) => b.count - a.count)[0];
  return bin ? bin.rgb.map(v => Math.round(v / bin.count)) : [150, 150, 150];
}

function luminance(rgb) {
  const channels = rgb.map(v => {
    const c = v / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}

function analyzeSection({ data, width, height }, {minGlyphHeight=5} = {}) {
  const paper = materialColor(data), light = luminance(paper);
  const mask = new Uint8Array(width * height), ink = [];
  for (let p = 0; p < mask.length; p++) {
    const value = luminance([data[p * 4], data[p * 4 + 1], data[p * 4 + 2]]);
    mask[p] = data[p * 4 + 3] >= 128 && (Math.max(light, value) + 0.05) / (Math.min(light, value) + 0.05) >= 2.5 ? 1 : 0;
  }
  let glyphs = 0;
  const heights = [], boxes = [];
  for (let p = 0; p < mask.length; p++) {
    if (!mask[p]) continue;
    const pending = [p], component = [];
    mask[p] = 0;
    let minX = width, maxX = 0, minY = height, maxY = 0;
    while (pending.length) {
      const at = pending.pop(), x = at % width, y = Math.floor(at / width);
      component.push(at);
      minX = Math.min(minX, x); maxX = Math.max(maxX, x);
      minY = Math.min(minY, y); maxY = Math.max(maxY, y);
      for (const [dx, dy] of [[-1, 0], [1, 0], [0, -1], [0, 1]]) {
        const nx = x + dx, ny = y + dy, next = ny * width + nx;
        if (nx >= 0 && nx < width && ny >= 0 && ny < height && mask[next]) {
          mask[next] = 0; pending.push(next);
        }
      }
    }
    const w = maxX - minX + 1, h = maxY - minY + 1;
    if (component.length < 3 || h < 2 || h > Math.min(36, height * 0.95)
      || w > Math.min(width * 0.6, h * 10) || minX === 0 || minY === 0 || maxX === width - 1 || maxY === height - 1) continue;
    glyphs++;
    if (h >= minGlyphHeight && w <= h * 2.5 && component.length >= h) {heights.push(h);boxes.push({x:minX,y:minY,width:w,height:h});}
    for (const at of component) ink.push(data[at * 4], data[at * 4 + 1], data[at * 4 + 2], 255);
  }
  // Use a repeated glyph-height cluster, not punctuation, borders, or symbols.
  let cluster = [];
  for (const h of heights) {
    const similar = heights.filter(value => Math.abs(value - h) <= 2);
    if (similar.length > cluster.length) cluster = similar;
  }
  cluster.sort((a, b) => a - b);
  const matched=boxes.filter(box=>cluster.includes(box.height));
  const glyphBounds=matched.length>=4?{
    x:Math.min(...matched.map(b=>b.x)),y:Math.min(...matched.map(b=>b.y)),
    right:Math.max(...matched.map(b=>b.x+b.width)),bottom:Math.max(...matched.map(b=>b.y+b.height)),
  }:null;
  const letters = glyphBounds ? boxes.filter(b => b.height >= cluster[0] * .6
    && b.y < glyphBounds.bottom && b.y + b.height > glyphBounds.y
    && b.height <= cluster.at(-1) * 2.2) : [];
  const textBounds = letters.length >= 4 ? {
    x: Math.min(...letters.map(b => b.x)), y: Math.min(...letters.map(b => b.y)),
    right: Math.max(...letters.map(b => b.x + b.width)), bottom: Math.max(...letters.map(b => b.y + b.height)),
  } : null;

  return {
    ink: glyphs >= 2 && ink.length >= 24 ? materialColor(ink) : light > .35 ? [23, 24, 25] : [245, 241, 230],
    glyphBounds,
    textBounds,
    glyphHeight: cluster.length >= 4 ? cluster[Math.floor((cluster.length - 1) * .8)] : null,
  };
}

export function sectionInk(region) {
  const ink = analyzeSection(region).ink;
  return luminance(ink) > luminance(materialColor(region.data))
    ? [255, 255, 255] : [0, 0, 0];
}
export function printedGlyphHeight(region) { return analyzeSection(region).glyphHeight; }
export function printedTextBounds(region) { return analyzeSection(region).textBounds; }

export function measureRulesFirstLine(ctx, box, text, family, { italic = false, bandIndex = 0 } = {}) {
  const x = Math.ceil(box.x + 9), y = Math.ceil(box.y + 8);
  const scan = ctx.getImageData(x, y, Math.floor(box.width - 18), Math.floor(box.height - 16));
  const paper = luminance(materialColor(scan.data));
  const ink = new Uint8Array(scan.width * scan.height), rows = [];
  for (let py = 0; py < scan.height; py++) {
    let count = 0;
    for (let px = 0; px < scan.width; px++) {
      const p = py * scan.width + px, value = luminance(Array.from(scan.data.subarray(p * 4, p * 4 + 3)));
      if ((Math.max(paper, value) + .05) / (Math.min(paper, value) + .05) > 2.5) { ink[p] = 1; count++; }
    }
    if (count >= 3) rows.push(py);
  }
  if (!rows.length) return null;
  const bands = [];
  for (const row of rows) {
    const band = bands.at(-1);
    if (!band || row - band.bottom > 2) bands.push({top:row,bottom:row});
    else band.bottom = row;
  }
  const lineBand = bands.filter(b => b.bottom - b.top >= 7 && b.bottom - b.top <= 32)[bandIndex];
  if (!lineBand) return null;
  const {top,bottom} = lineBand;
  const nextLine = bands.find(b => b.top > bottom && b.bottom - b.top >= 7);
  const lineHeight = nextLine ? nextLine.top - top : null;
  let left = scan.width, right = 0;
  for (let py = top; py <= bottom; py++) for (let px = 0; px < scan.width; px++) if (ink[py * scan.width + px]) {
    left = Math.min(left, px); right = Math.max(right, px);
  }
  const width = right - left + 1, height = bottom - top + 1;
  const canvas = document.createElement('canvas'); canvas.width = width; canvas.height = height;
  const template = canvas.getContext('2d', {willReadFrequently:true});
  const words = String(text || '').split(/\n/)[0].split(/\s+/).filter(Boolean);
  const candidates = [];
  for (let n = 1; n <= Math.min(24, words.length); n++) {
    const line = words.slice(0, n).join(' '); if (line.includes('{')) break;
    template.font = `${italic ? "italic " : ""}400 100px ${family}`;
    const m = template.measureText(line), size = width / (m.actualBoundingBoxLeft + m.actualBoundingBoxRight) * 100;
    const expectedHeight = (m.actualBoundingBoxAscent + m.actualBoundingBoxDescent) * size / 100;
    if (size < 12 || size > 36 || expectedHeight < height * .8 || expectedHeight > height * 1.2) continue;
    template.clearRect(0, 0, width, height);
    template.save(); template.scale(size / 100, height / (m.actualBoundingBoxAscent + m.actualBoundingBoxDescent));
    template.fillStyle = 'white'; template.fillText(line, m.actualBoundingBoxLeft, m.actualBoundingBoxAscent); template.restore();
    const pixels = template.getImageData(0, 0, width, height).data;
    let intersection = 0, union = 0;
    for (let py = 0; py < height; py++) for (let px = 0; px < width; px++) {
      const a = ink[(py + top) * scan.width + px + left], b = pixels[(py * width + px) * 4 + 3] > 80;
      if (a && b) intersection++; if (a || b) union++;
    }
    candidates.push({size, confidence:intersection / union, line, x:x+left, y:y+top, width, height, lineHeight});
  }
  // Symbol-led lines cannot be compared as plain canvas text. Their printed
  // letter height still gives a font size without treating {T} as three glyphs.
  const first = String(text || '').split('\n')[0];
  if (first.includes('{')) {
    const letters = first.replace(/\{[^}]+\}/g, '').trim();
    if (letters) {
      template.font = `${italic ? "italic " : ""}400 100px ${family}`;
      const metrics = template.measureText(letters);
      const size = height / (metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent) * 100;
      if (size >= 12 && size <= 36) return {size,line:letters,x:x+left,y:y+top,width,height,lineHeight};
    }
  }
  candidates.sort((a,b) => b.confidence - a.confidence);
  const best = candidates[0];
  return best?.confidence > .4 && (!candidates[1] || best.confidence - candidates[1].confidence > .06) ? best : null;
}

// Flavor can start below several rules lines. Match its own italic text against
// each printed band; never inherit the size of an unrelated activated ability.
export function measureFlavorFirstLine(ctx, box, text, family) {
  if (!text) return null;
  const candidates = [];
  for (let bandIndex = 0; bandIndex < 18; bandIndex++) {
    const match = measureRulesFirstLine(ctx, box, text, family, {italic:true, bandIndex});
    if (match) candidates.push(match);
  }
  return candidates.sort((a,b) => b.confidence - a.confidence)[0] || null;
}

const sourceImages = new Map();

function loadImage(url) {
  if (sourceImages.has(url)) return sourceImages.get(url);
  const request = new Promise((resolve, reject) => {
    const image = new Image();
    image.crossOrigin = 'anonymous'; image.referrerPolicy = 'no-referrer';
    const timer = setTimeout(() => { image.src = ''; reject(new Error('Card colors timed out')); }, 12000);
    image.onload = () => { clearTimeout(timer); resolve(image); };
    image.onerror = () => { clearTimeout(timer); reject(new Error('Card colors unavailable')); };
    image.src = url;
  }).catch(error => { sourceImages.delete(url); throw error; });
  sourceImages.set(url, request);
  if (sourceImages.size > 48) sourceImages.delete(sourceImages.keys().next().value);
  return request;
}

// Fetch the masking source while printing metadata and fonts are loading.
export function preloadCardFrameSource(imageUrl) {
  const url = fullCardImageUrl(imageUrl);
  return url ? loadImage(url).catch(() => null) : Promise.resolve(null);
}

// A real bottom rail has a horizontal transition across most of the crop.
export function artBottomRail({ data, width, height }) {
  let best = null;
  for (let offset = 3; offset < height * .045; offset++) {
    const y = height - 1 - offset;
    const changes = Array.from({ length: 24 }, (_, i) => {
      const x = Math.floor(width * (.1 + i * .8 / 23));
      const a = ((y - 1) * width + x) * 4, b = ((y + 1) * width + x) * 4;
      return Math.hypot(...[0, 1, 2].map(c => data[a + c] - data[b + c]));
    });
    const support = changes.filter(v => v > 28).length / changes.length;
    const score = changes.reduce((sum, v) => sum + Math.min(v, 100), 0) / changes.length;
    if (support >= .75 && score > 38 && (!best || score > best.score)) best = { height: offset, score };
  }
  return best?.height || 0;
}

export function reconstructPanel({ data, width, height }, { removeSeparators = false, minimumCleanFraction = .2 } = {}) {
  const count = width * height, mask = new Uint8Array(count);
  const bands = Math.max(1, Math.ceil(width / 64));
  const paper = Array.from({ length: bands }, (_, band) => {
    const pixels = [];
    for (let y = 0; y < height; y++) for (let x = Math.floor(band * width / bands); x < Math.floor((band + 1) * width / bands); x++) {
      const p = (y * width + x) * 4;
      pixels.push(...data.subarray(p, p + 4));
    }
    return materialColor(pixels);
  });
  for (let p = 0; p < count; p++) {
    const background = paper[Math.min(bands - 1, Math.floor((p % width) * bands / width))];
    const rgb = Array.from(data.subarray(p * 4, p * 4 + 3));
    const a = luminance(background), b = luminance(rgb);
    const contrast = (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
    const difference = Math.hypot(...rgb.map((v, c) => v - background[c]));
    // Pale panels need a lower threshold for faint printed ink; dark
    // textured frames need more tolerance for their natural highlights.
    if (contrast > (a > .4 ? 1.3 : 1.7) && difference > (a > .4 ? 30 : 55)) mask[p] = 1;
  }
  // Rules dividers can be much paler than lettering. Look for a thin,
  // sustained horizontal valley against nearby paper, then mask its full row.
  if (removeSeparators) for (let y=3;y<height-3;y++) {
    let support=0;
    for(let x=0;x<width;x++) {
      const light=dy=>luminance(Array.from(data.subarray(((y+dy)*width+x)*4,((y+dy)*width+x)*4+3)));
      const valley=Math.min(light(-3),light(3))-light(0);
      if (valley>.012 && valley<.12) support++;
    }
    if (support>width*.4) for(let x=0;x<width;x++) mask[y*width+x]=1;
  }
  // Include anti-aliasing and printed outlines around the detected ink.
  const expanded = mask.slice();
  for (let p = 0; p < count; p++) if (mask[p]) {
    const x = p % width, y = Math.floor(p / width);
    for (let dy = -2; dy <= 2; dy++) for (let dx = -2; dx <= 2; dx++) {
      if (x + dx >= 0 && x + dx < width && y + dy >= 0 && y + dy < height) expanded[(y + dy) * width + x + dx] = 1;
    }
  }
  const clean = [];
  for (let p = 0; p < count; p++) if (!expanded[p]) clean.push(p);
  if (!clean.length || clean.length < count * minimumCleanFraction) return null;
  const output = data.slice();
  // Copy only original unmasked pixels. Nearby donors preserve local material;
  // deterministic variation prevents the long streaks of nearest-pixel filling.
  for (let p = 0; p < count; p++) if (expanded[p]) {
    const x = p % width, y = Math.floor(p / width);
    let donor = -1, score = Infinity;
    const seed = Math.imul(p + 1, 2654435761) >>> 0;
    for (let i = 0; i < 96; i++) {
      const candidate = clean[(seed + Math.imul(i, 15485863) >>> 0) % clean.length];
      const dx = candidate % width - x, dy = Math.floor(candidate / width) - y;
      const distance = dx * dx * 3 + dy * dy;
      if (distance < score) { donor = candidate; score = distance; }
    }
    output.set(data.subarray(donor * 4, donor * 4 + 4), p * 4);
  }
  return { data: output, width, height, mask: expanded };
}

function classifyFramePanel({ data, width, height }, section) {
  const type = section === 'type';
  const patch = (x, y, w, h) => {
    const pixels = [];
    for (let py = Math.floor(y * height); py < Math.floor((y + h) * height); py++) {
      for (let px = Math.floor(x * width); px < Math.floor((x + w) * width); px++) {
        const p = (py * width + px) * 4;
        pixels.push(...data.subarray(p, p + 4));
      }
    }
    return materialColor(pixels);
  };
  const paper = patch(.12, type ? .573 : .055, .57, type ? .025 : .035), paperLight = luminance(paper);
  const edge = right => {
    let best = { support: 0, rgb: paper, x: Math.floor(width * .065), stroke: 1 };
    const columns = [];
    for (let x = Math.floor(width * .052); x <= width * .085; x++) {
      const colors = [], contrasts = [];
      for (let y = Math.floor(height * (type ? .575 : .06)); y <= height * (type ? .592 : .087); y++) {
        const p = (y * width + (right ? width - 1 - x : x)) * 4;
        const rgb = Array.from(data.subarray(p, p + 3));
        const light = luminance(rgb);
        colors.push(...rgb, 255);
        contrasts.push((Math.max(light, paperLight) + .05) / (Math.min(light, paperLight) + .05) > 1.8 && Math.hypot(...rgb.map((v, c) => v - paper[c])) > 55);
      }
      const support = contrasts.filter(Boolean).length / contrasts.length;
      columns.push({ x, support });
      if (support > best.support) best = { support, rgb: materialColor(colors), x, stroke: 1 };
    }
    // Measure only the connected stroke around the best column, not nearby
    // ornament or a second outline separated by a highlight.
    for (const direction of [-1, 1]) {
      for (let x = best.x + direction; ; x += direction) {
        if ((columns.find(column => column.x === x)?.support || 0) < .85) break;
        best.stroke++;
      }
    }
    return best;
  };
  const left = edge(false), right = edge(true);
  const enclosure = Math.min(left.support, right.support);
  // Find the upper horizontal outline and sample its inner highlight. This
  // gives metallic panels a scan-derived bevel instead of a generic white line.
  let top = null;
  for (let y = Math.floor(height * (type ? .552 : .035)); y <= height * (type ? .575 : .055); y++) {
    const colors = [], differences = [];
    for (let i = 0; i < 32; i++) {
      const x = Math.floor(width * (.14 + i * .64 / 31));
      const p = (y * width + x) * 4;
      const rgb = Array.from(data.subarray(p, p + 3)), light = luminance(rgb);
      colors.push(...rgb, 255);
      differences.push((Math.max(light, paperLight) + .05) / (Math.min(light, paperLight) + .05));
    }
    const support = differences.filter(value => value > 1.8).length / differences.length;
    if (!top || support > top.support) top = { y, support, rgb: materialColor(colors) };
  }
  const stroke = Math.max(1, Math.min(3, (left.stroke + right.stroke) / 2));
  // Scan-scale dimensions follow the frame's line weight and scale with the UI.
  const radius = Math.max(6, Math.min(12, stroke * 2 + 5));
  return {
    kind: enclosure >= .9 ? 'panel' : 'integrated',
    confidence: enclosure >= .9 ? enclosure : 1 - enclosure,
    border: left.rgb.map((value, c) => Math.round((value + right.rgb[c]) / 2)),
    highlight: top?.support >= .8 ? patch(.14, (top.y + Math.ceil(stroke) + 1) / height, .64, .003) : patch(.14, type ? .568 : .047, .64, .005),
    stroke: stroke / width * 100,
    radius: radius / width * 100,
  };
}

export function classifyTitlePanel(image) {
  return classifyFramePanel(image, 'title');
}

export function classifyTypePanel(image) {
  return classifyFramePanel(image, 'type');
}

// Detect a straight paper-to-frame transition below the rules, excluding
// collector text and curved/decorative boxes that cannot use a straight splice.
export function rulesBottomEdge({ data, width, height }) {
  const rgb = (x, y) => Array.from(data.subarray((y * width + x) * 4, (y * width + x) * 4 + 3));
  const distance = (a, b) => Math.hypot(...a.map((v, c) => v - b[c]));
  let best = null;
  for (let y = Math.floor(height * .85); y < height * .915; y++) {
    const changes = Array.from({ length: 24 }, (_, i) => {
      const x = Math.floor(width * (.16 + i * .65 / 23));
      return distance(rgb(x, y - 3), rgb(x, y + 3));
    });
    const support = changes.filter(v => v > 65).length / changes.length;
    const score = changes.reduce((sum, v) => sum + Math.min(v, 160), 0) / changes.length;
    if (support >= .95 && (!best || score > best.score)) best = { y, score };
  }
  if (!best) return null;
  const side = right => {
    let edge = null;
    for (let offset = Math.floor(width * .07); offset < width * .14; offset++) {
      const x = right ? width - 1 - offset : offset;
      const score = distance(rgb(x - 2, best.y - 8), rgb(x + 2, best.y - 8));
      if (!edge || score > edge.score) edge = { x, score };
    }
    return edge;
  };
  const left = side(false), right = side(true);
  if (left.score < 65 || right.score < 65) return null;
  return { x: left.x - 3, y: best.y - 5, width: right.x - left.x + 7, height: 10, corner: 12 };
}

export function detectPanelBounds({data, width, height}, section) {
  const regions = {
    title: {top: [.043, .06], bottom: [.095, .118], sides: [.063, .085]},
    type: {top: [.56, .58], bottom: [.598, .628], sides: [.578, .60]},
    rules: {top: [.593, .65], bottom: [.855, .935], sides: [.66, .84]},
  };
  const ranges = regions[section];
  if (section === 'rules' && classifyTypePanel({data,width,height}).kind === 'panel') ranges.top = [.615, .65];
  if (!ranges) return null;
  const difference = (x1,y1,x2,y2) => {
    const a = (y1 * width + x1) * 4, b = (y2 * width + x2) * 4;
    return Math.hypot(...[0,1,2].map(c => data[a+c] - data[b+c]));
  };
  const horizontal = range => {
    let best = null;
    for (let y = Math.floor(range[0] * height); y <= range[1] * height; y++) {
      const changes = Array.from({length:32}, (_,i) => {
        const x = Math.floor(width * (.15 + i * .65 / 31));
        return difference(x,y-2,x,y+2);
      });
      const support = changes.filter(v => v > 24).length / changes.length;
      const score = changes.reduce((n,v) => n + Math.min(v,120),0) / changes.length;
      if (support >= .65 && (!best || score > best.score)) best = {position:y, score};
    }
    return best;
  };
  const vertical = (right, matchingLeft = null) => {
    let best = null;
    for (let offset = Math.floor(width * .057); offset <= width * .125; offset++) {
      if (matchingLeft && Math.abs(offset - matchingLeft.position) > 10) continue;
      const x = right ? width - 1 - offset : offset;
      const changes = Array.from({length:16}, (_,i) => {
        const y = Math.floor(height * (ranges.sides[0] + i * (ranges.sides[1] - ranges.sides[0]) / 15));
        return difference(x-2,y,x+2,y);
      });
      const support = changes.filter(v => v > 24).length / changes.length;
      const score = changes.reduce((n,v) => n + Math.min(v,120),0) / changes.length;
      if (support >= .65 && (!best || score > best.score)) best = {position:x,score};
    }
    return best;
  };
  const top=horizontal(ranges.top), bottom=horizontal(ranges.bottom), left=vertical(false), right=vertical(true, left);
  if (!top || !bottom || !left || !right) return null;
  // A transition locates the beginning of a dark lower rail, not its far
  // edge. Follow its sustained stroke so the crop cannot cut the rim off.
  const darkRow=y=>{
    let support=0;
    for(let i=0;i<32;i++) {
      const x=Math.floor(width*(.17+i*.64/31)),at=(y*width+x)*4;
      if((data[at]+data[at+1]+data[at+2])/3<110)support++;
    }
    return support>=25;
  };
  let far=bottom.position,started=false;
  for(let d=0;d<=7 && section==='title';d++) {
    if(darkRow(bottom.position+d)){started=true;far=bottom.position+d;}
    else if(started||d>=3)break;
  }
  const x=left.position-1, y=top.position-1, w=right.position-left.position+3, h=far-top.position+3;
  if (w < width * .7 || h < 20) return null;
  return {x,y,width:w,height:h};
}

// Follow connected enclosure strokes rather than choosing unrelated strong
// horizontal/vertical edges. This keeps curved corners and the lower bevel
// inside the same crop on modern and Mirrodin-style frames.
export function detectEnclosedPanelBounds(scan, section) {
  if (!['title','type'].includes(section)) return null;
  const {data,width,height}=scan;
  const y0=Math.floor(height*(section==='title'?.032:.545));
  const y1=Math.ceil(height*(section==='title'?.12:.635));
  const x0=Math.floor(width*.035), x1=Math.ceil(width*.965);
  const w=x1-x0,h=y1-y0;
  const paperPixels=[];
  const sampleTop=Math.floor(height*(section==='title'?.055:.573));
  for(let y=sampleTop;y<sampleTop+14;y++) for(let x=Math.floor(width*.14);x<width*.7;x++) {
    const p=(y*width+x)*4;paperPixels.push(...data.subarray(p,p+4));
  }
  const paper=materialColor(paperPixels), light=luminance(paper);
  const ink=new Uint8Array(w*h);
  for(let y=0;y<h;y++) for(let x=0;x<w;x++) {
    const p=((y+y0)*width+x+x0)*4;
    const value=luminance(Array.from(data.subarray(p,p+3)));
    if((Math.max(light,value)+.05)/(Math.min(light,value)+.05)>1.8) ink[y*w+x]=1;
  }
  // Close one-pixel anti-aliasing gaps, without merging separate text lines.
  const joined=ink.slice();
  for(let y=1;y<h-1;y++) for(let x=1;x<w-1;x++) if(ink[y*w+x]) {
    for(const [dx,dy] of [[1,0],[-1,0],[0,1],[0,-1]]) joined[(y+dy)*w+x+dx]=1;
  }
  let best=null;
  for(let p=0;p<joined.length;p++) if(joined[p]) {
    const pending=[p];joined[p]=0;
    let left=w,right=0,top=h,bottom=0,area=0;
    while(pending.length){
      const at=pending.pop(),x=at%w,y=Math.floor(at/w);area++;
      left=Math.min(left,x);right=Math.max(right,x);top=Math.min(top,y);bottom=Math.max(bottom,y);
      for(const [dx,dy] of [[1,0],[-1,0],[0,1],[0,-1]]) {
        const nx=x+dx,ny=y+dy,next=ny*w+nx;
        if(nx>=0&&nx<w&&ny>=0&&ny<h&&joined[next]){joined[next]=0;pending.push(next);}
      }
    }
    const cw=right-left+1,ch=bottom-top+1;
    if(left===0||right===w-1||top===0||bottom===h-1||cw<width*.75||ch<height*.035||area>cw*ch*.55) continue;
    if(!best||cw>best.width) best={x:x0+left,y:y0+top,width:cw,height:ch};
  }
  const rails=best&&detectPanelBounds(scan,section);
  if(rails && best.x-rails.x>=4 && best.x-rails.x<=10
    && Math.abs(best.y-rails.y)<=4) {
    const right=Math.max(best.x+best.width,rails.x+rails.width);
    best={...best,x:rails.x,width:right-rails.x};
  }
  return best;
}

// Trace the narrow source rim into color-grouped vector paths. Unlike a
// generic rounded rectangle this preserves asymmetric corners and stacked
// highlight/shadow strokes without carrying the panel's printed content.
export function tracePanelRim(scan, bounds, section) {
  const {width,height}=bounds, groups=new Map();
  const corner=section==='rules'?3:Math.min(12,Math.floor(height/3));
  const included=(x,y)=>x<6||x>=width-6||y<6||y>=height-6
    || ((x<corner||x>=width-corner)&&(y<8||y>=height-8));
  for(let y=0;y<height;y++) {
    let x=0;
    while(x<width) {
      if(!included(x,y)){x++;continue;}
      const colorAt=x=>{
        const at=((bounds.y+y)*scan.width+bounds.x+x)*4;
        return [0,1,2].map(c=>Math.min(255,Math.round(scan.data[at+c]/8)*8)).join(',');
      };
      const start=x,color=colorAt(x++);
      while(x<width&&included(x,y)&&colorAt(x)===color)x++;
      groups.set(color,(groups.get(color)||'')+`M${start} ${y}h${x-start}v1h-${x-start}z`);
    }
  }
  return [...groups].map(([color,path])=>`<path shape-rendering="crispEdges" fill="rgb(${color})" d="${path}"/>`).join('');
}

export function matchArtBounds(scan, art) {
  if (!art || scan.height/scan.width<1.2) return null;
  const samples=[];
  for(let gy=0;gy<8;gy++) for(let gx=0;gx<10;gx++) {
    const u=.12+gx*.76/9,v=.12+gy*.76/7;
    const p=(Math.floor(v*art.height)*art.width+Math.floor(u*art.width))*4;
    samples.push({u,v,rgb:Array.from(art.data.subarray(p,p+3))});
  }
  const score=(x,y,w)=>{
    const h=w*art.height/art.width;
    let error=0;
    for(const {u,v,rgb} of samples) {
      const p=(Math.round(y+v*h)*scan.width+Math.round(x+u*w))*4;
      for(let c=0;c<3;c++) error+=Math.abs(scan.data[p+c]-rgb[c]);
    }
    return error/(samples.length*3);
  };
  let best={error:Infinity};
  for(let w=Math.round(scan.width*.70);w<=scan.width*.98;w+=3) {
    for(let x=Math.round((scan.width-w)/2)-12;x<=(scan.width-w)/2+12;x+=3) {
      for(let y=Math.round(scan.height*.08);y<=scan.height*.14;y+=3) {
        const error=score(x,y,w);if(error<best.error) best={x,y,width:w,error};
      }
    }
  }
  const coarse=best;
  for(let w=coarse.width-2;w<=coarse.width+2;w++) for(let x=coarse.x-2;x<=coarse.x+2;x++) for(let y=coarse.y-2;y<=coarse.y+2;y++) {
    const error=score(x,y,w);if(error<best.error) best={x,y,width:w,error};
  }
  if(best.error>22) return null;
  const edge=(from,to,fallback)=>{
    let found=null;
    for(let y=Math.round(from);y<=to;y++) {
      let support=0,total=0;
      for(let i=0;i<32;i++) {
        const x=Math.round(best.x+best.width*(.08+i*.84/31));
        const a=((y-2)*scan.width+x)*4,b=((y+2)*scan.width+x)*4;
        const d=Math.hypot(...[0,1,2].map(c=>scan.data[a+c]-scan.data[b+c]));
        if(d>24) support++;total+=Math.min(d,100);
      }
      if(support>=24&&(!found||total>found.score)) found={y,score:total};
    }
    return found?.y ?? fallback;
  };
  const bottom=best.y+best.width*art.height/art.width;
  const top=edge(best.y-scan.height*.012,best.y+2,best.y);
  const end=edge(bottom-4,bottom+5,bottom);
  return {...best,y:top,height:end-top};
}

// P/T is a small, aligned cluster of glyphs in the lower right. Try both ink
// polarities so bare white retro numerals and black inset numerals both work.
export function detectPrintedStats(scan) {
  const {data,width,height}=scan;
  if(height/width<1.2) return null;
  const x0=Math.floor(width*.75),y0=Math.floor(height*.875),w=Math.floor(width*.21),h=Math.floor(height*.09);
  let best=null;
  for(const lightInk of [false,true]) {
    const mask=new Uint8Array(w*h);
    for(let y=0;y<h;y++) for(let x=0;x<w;x++) {
      const p=((y0+y)*width+x0+x)*4;
      const value=luminance(Array.from(data.subarray(p,p+3)));
      mask[y*w+x]=lightInk?value>.4:value<.19;
    }
    const glyphs=[];
    for(let p=0;p<mask.length;p++) if(mask[p]) {
      const queue=[p];mask[p]=0;let l=w,r=0,t=h,b=0,area=0;
      while(queue.length) {
        const at=queue.pop(),x=at%w,y=Math.floor(at/w);area++;
        l=Math.min(l,x);r=Math.max(r,x);t=Math.min(t,y);b=Math.max(b,y);
        for(const [dx,dy] of [[-1,0],[1,0],[0,-1],[0,1],[-1,-1],[-1,1],[1,-1],[1,1]]) {
          const nx=x+dx,ny=y+dy,n=ny*w+nx;
          if(nx>=0&&nx<w&&ny>=0&&ny<h&&mask[n]){mask[n]=0;queue.push(n);}
        }
      }
      const gh=b-t+1,gw=r-l+1;
      if(gh>=height*.017&&gh<=height*.043&&gw<=gh*1.4&&area>=gh&&l>0&&r<w-1&&t>0&&b<h-1) glyphs.push({l,r,t,b,gh});
    }
    for(const g of glyphs) {
      const line=glyphs.filter(v=>Math.abs((v.t+v.b-g.t-g.b)/2)<3&&Math.abs(v.gh-g.gh)<6).sort((a,b)=>a.l-b.l);
      if(line.length<3||line.length>5) continue;
      if(line.some((v,i)=>i&&v.l-line[i-1].r>g.gh*.9)) continue;
      const l=line[0].l,r=line.at(-1).r,t=Math.min(...line.map(v=>v.t)),b=Math.max(...line.map(v=>v.b));
      if(r-l>width*.15) continue;
      const score=line.length*10+g.gh;
      if(!best||score>best.score) best={x:x0+l,y:y0+t,width:r-l+1,height:b-t+1,score};
    }
  }
  return best;
}

// A P/T panel has sustained side strokes and a horizontal enclosure. Bare
// numerals on the frame do not; their surrounding texture stays uninterrupted.
export function printedStatsTreatment(scan,stats) {
  if(!stats) return 'text';
  const {data,width,height}=scan,pixels=[];
  const pixel=(x,y)=>Array.from(data.subarray((Math.max(0,Math.min(height-1,y))*width+Math.max(0,Math.min(width-1,x)))*4,(Math.max(0,Math.min(height-1,y))*width+Math.max(0,Math.min(width-1,x)))*4+3));
  for(let y=stats.y-3;y<stats.y+stats.height+3;y++) for(let x=stats.x-5;x<stats.x+stats.width+5;x++) pixels.push(...pixel(x,y),255);
  const paper=materialColor(pixels),light=luminance(paper);
  const stroke=(x,y)=>{
    const rgb=pixel(x,y),v=luminance(rgb);
    return (Math.max(light,v)+.05)/(Math.min(light,v)+.05)>1.65 && Math.hypot(...rgb.map((c,i)=>c-paper[i]))>35;
  };
  const side=right=>{
    const edge=right?stats.x+stats.width:stats.x;
    for(let d=3;d<width*.085;d++) {
      const x=edge+(right?d:-d);
      if(x<width*.05||x>width*.95) continue;
      let support=0;
      for(let i=0;i<12;i++) if(stroke(x,Math.round(stats.y+stats.height*(.15+i*.7/11)))) support++;
      if(support>=10) return true;
    }
    return false;
  };
  let horizontal=false;
  for(const bottom of [false,true]) for(let d=2;d<=stats.height*.75;d++) {
    const y=bottom?stats.y+stats.height+d:stats.y-d;let support=0;
    for(let i=0;i<16;i++) if(stroke(Math.round(stats.x+stats.width*(.1+i*.8/15)),Math.round(y))) support++;
    if(support>=13) horizontal=true;
  }
  return side(false)&&side(true)&&horizontal?'panel':'text';
}

export function detectStatsPanelBounds(scan, stats) {
  if(!stats || printedStatsTreatment(scan,stats)!=='panel') return null;
  const {width,height,data}=scan;
  const dark=(x,y)=>{const at=(y*width+x)*4;return (data[at]+data[at+1]+data[at+2])/3<100;};
  const edge=(axis,start,direction,limit)=>{
    for(let d=3;d<limit;d++) {
      const p=Math.round(start+direction*d);let count=0;
      if(p<1||p>=(axis==='x'?width:height)-1)continue;
      for(let i=0;i<12;i++) {
        const x=axis==='x'?p:Math.round(stats.x+stats.width*(.15+i*.7/11));
        const y=axis==='y'?p:Math.round(stats.y+stats.height*(.15+i*.7/11));
        if(dark(x,y))count++;
      }
      if(count>=10)return p;
    }
    return null;
  };
  const left=edge('x',stats.x,-1,width*.085),right=edge('x',stats.x+stats.width,1,width*.085);
  const top=edge('y',stats.y,-1,stats.height),bottom=edge('y',stats.y+stats.height,1,stats.height);
  if([left,right,top,bottom].some(v=>v===null))return null;
  return {x:left-3,y:top-3,width:right-left+7,height:bottom-top+7};
}

export function measureFrameGeometry(scan,art,conventional=true) {
  const stats=detectPrintedStats(scan);
  let rules=detectPanelBounds(scan,'rules');
  const style={'--printed-scan-width':scan.width, '--printed-scan-height':scan.height};
  if(stats) {
    // Place the center relative to the rules box, retaining the original
    // overlap or drop below its lower edge as our rules section grows.
    const box=rules || {x:scan.width*.07,width:scan.width*.86,y:scan.height*.625,height:scan.height*.29};
    const drop=(stats.y+stats.height/2-(box.y+box.height))/scan.width*100;
    style['--printed-pt-left']=`${Math.max(80,Math.min(94,(stats.x+stats.width/2-box.x)/box.width*100))}%`;
    style['--printed-pt-drop']=`${Math.max(-2,Math.min(7,drop))}cqw`;
    style['--printed-pt-position']='rules';
    style['--printed-pt-treatment']=printedStatsTreatment(scan,stats);
    style['--printed-pt-font-size']=`calc(${stats.height/scan.width*100}cqw / var(--card-stats-glyph-ratio, .7))`;

  }
  if(!conventional) return style;
  let artBox=matchArtBounds(scan,art);
  if(!artBox) return style;
  if(classifyTitlePanel(scan).kind==='panel') {
    // Crop matching can stop inside the illustration. Sustained straight dark
    // rails are stronger evidence for its actual left/right opening.
    const rail=(edge,right)=>{
      let best=null;
      for(let x=Math.round(edge-10);x<=edge+10;x++) {
        let supported=0;
        for(let i=0;i<32;i++) {
          const y=Math.round(artBox.y+artBox.height*(.08+i*.84/31));
          const value=xx=>{const at=(y*scan.width+xx)*4;return (scan.data[at]+scan.data[at+1]+scan.data[at+2])/3;};
          if(value(x)<90 && value(x+(right?-3:3))-value(x)>35)supported++;
        }
        if(supported>=28&&(!best||supported>best.supported))best={x,supported};
      }
      return best?.x;
    };
    const left=rail(artBox.x,false),right=rail(artBox.x+artBox.width,true);
    if(left!=null&&right!=null) {
      const x=left+3,width=right-left-5;
      const height=width*art.height/art.width;
      if(Math.abs(height-artBox.height)<8)
        artBox={...artBox,x,width,y:artBox.y+(artBox.height-height)/2,height};
    }
  }
  const glyphBox=(section)=>{
    const x=Math.floor(scan.width*.07),y=Math.floor(scan.height*(section==='title'?.035:.55));
    const w=Math.floor(scan.width*.72),h=Math.floor(scan.height*.065),data=new Uint8ClampedArray(w*h*4);
    for(let row=0;row<h;row++) data.set(scan.data.subarray(((y+row)*scan.width+x)*4,((y+row)*scan.width+x+w)*4),row*w*4);
    const analysis=analyzeSection({data,width:w,height:h},{minGlyphHeight:Math.floor(scan.height*.015)});
    const bounds=analysis.textBounds || analysis.glyphBounds;
    const padding=bounds?3:0;
    return bounds?{x:x+bounds.x,y:y+bounds.y-padding,width:bounds.right-bounds.x,height:bounds.bottom-bounds.y+padding*2}:null;
  };
  // Integrated bars use the printed glyph block plus breathing room when
  // there is no enclosing stroke to measure.
  const titleEnclosure=classifyTitlePanel(scan).kind==='panel' ? (detectEnclosedPanelBounds(scan,'title') || detectPanelBounds(scan,'title')) : null;
  let title=titleEnclosure || glyphBox('title');
  style['--title-panel-kind']=titleEnclosure?'panel':'integrated';
  if(title && title.y+title.height>artBox.y && title.y+title.height-artBox.y<=8)
    title={...title,height:artBox.y-title.y};
  const type=detectEnclosedPanelBounds(scan,'type') || detectPanelBounds(scan,'type') || glyphBox('type');
  // A shared type/rules edge can be detected on both sides of its bevel.
  // Allocate that overlap to the type bar instead of rejecting both boxes.
  if (rules && type && rules.y < type.y + type.height
    && type.y + type.height - rules.y < scan.height * .04) {
    const top = type.y + type.height;
    rules = {...rules, y: top, height: rules.y + rules.height - top};
  }
  // The visible artwork remains the source scan. Keep its annotation rectangle
  // inside the measured text rows when a bevel is mistaken for the art edge.
  if(title && title.y+title.height>artBox.y && title.y+title.height-artBox.y<scan.height*.03) {
    const top=title.y+title.height; artBox={...artBox,y:top,height:artBox.y+artBox.height-top};
  }
  style['--printed-layout-candidates']=JSON.stringify({title,type,rules,art:artBox});
  // All regions share source coordinates and one uniform scale. Integrated
  // bars have no enclosing sides: use the art's span, not the glyph width.
  if (title && type && rules && title.y + title.height <= artBox.y + 3
    && artBox.y + artBox.height <= type.y + 3 && type.y + type.height <= rules.y + 3) {
    const titleLeft=Math.min(artBox.x,title.x-4);
    const titleBox = titleEnclosure ? title : {...title, x:titleLeft, width:artBox.x+artBox.width-titleLeft};
    const typeLeft=Math.min(rules.x,type.x-4);
    const typeBox = classifyTypePanel(scan).kind === 'panel' ? type : {...type, x:typeLeft, width:rules.x+rules.width-typeLeft};
    const boxes = {title:titleBox, type:typeBox, rules, art:artBox};
    style['--printed-box-sizing'] = 'measured';
    style['--printed-layout'] = JSON.stringify(boxes);
    for (const [name, box] of Object.entries(boxes)) {
      for (const dimension of ['x', 'y', 'width', 'height']) {
        style[`--printed-${name}-${dimension}`] = box[dimension];
      }
    }
  }
  const setGap=(name,value)=>{if(value>=-3&&value<scan.height*.04) style[`--printed-gap-${name}`]=`${Math.max(0,value)/scan.width*100}cqw`;};
  if(title) setGap('title-art',artBox.y-(title.y+title.height));
  if(type) {
    setGap('art-type',type.y-(artBox.y+artBox.height));
    if(rules) setGap('type-rules',rules.y-(type.y+type.height));
  }
  return style;
}

// Curved Future Sight frames need text envelopes, not rectangular panel edges.
// These frame-family search regions deliberately exclude the type medallion,
// left-hand mana column, set symbol, and curved paper rim. Glyph registration
// below measures the actual text before anything is removed or replaced.
function futureFrameGeometry(scan) {
  const {width:w,height:h}=scan;
  const rect=(x,y,width,height)=>({x:Math.round(x*w),y:Math.round(y*h),width:Math.round(width*w),height:Math.round(height*h)});
  const boxes={title:rect(.175,.055,.74,.055),type:rect(.12,.565,.725,.055),
    rules:rect(.09,.635,.825,.25),art:rect(.20,.12,.73,.44)};
  const style={...measureFrameGeometry(scan,null,false),'--printed-box-sizing':'measured','--printed-layout':JSON.stringify(boxes),'--printed-mana-placement':'column'};
  for(const [name,box] of Object.entries(boxes))for(const [dimension,value] of Object.entries(box))style[`--printed-${name}-${dimension}`]=value;
  return style;
}

// Modern basic lands print only their large mana symbol in the text box, so
// the box is kept as printed. Printings that set flavor text or real reminder
// text there (The Hobbit basics, for example) must be masked like any other
// card, or the printed lettering shows through the live text laid over it.
// Scryfall records the bare symbol as a single letter (printed_text "B").
export function basicLandBoxIsTextless(printing) {
  if (!/\bBasic\b.*\bLand\b/.test(printing?.type_line || '') || !['2003', '2015'].includes(printing?.frame)) return false;
  if (String(printing?.flavor_text || '').trim()) return false;
  return !/\p{L}{3,}/u.test(String(printing?.printed_text || ''));
}

async function sample(fullUrl, typography, printing, setSymbolUrl) {
  const layoutGap=sourceMaskLayoutGap(printing);
  if(layoutGap)return {'--source-frame-status':'original','--source-frame-fallback-reason':layoutGap};
  const artUrl = /^https:\/\/cards\.scryfall\.io\/normal\//.test(fullUrl) ? fullUrl.replace('/normal/', '/art_crop/') : '';
  const [image, art] = await Promise.all([loadImage(fullUrl), artUrl ? loadImage(artUrl).catch(() => null) : null]);
  const canvas = document.createElement('canvas');
  canvas.width = 488; canvas.height = Math.round(image.height * 488 / image.width);
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  ctx.drawImage(image, 0, 0, canvas.width, canvas.height);
  const fullScan = ctx.getImageData(0, 0, canvas.width, canvas.height);
  let artScan = null;
  if (art) {
    const crop = document.createElement('canvas');
    crop.width = 160; crop.height = Math.round(art.height * 160 / art.width);
    const cropCtx = crop.getContext('2d', { willReadFrequently: true });
    cropCtx.drawImage(art, 0, 0, crop.width, crop.height);
    artScan = cropCtx.getImageData(0, 0, crop.width, crop.height);
  }
  // Geometry is evidence for placing editable text, never a recipe for a
  // replacement frame. If it cannot be measured, retain the original card.
  const future = printing?.frame === 'future';
  const style = future ? futureFrameGeometry(fullScan) : measureFrameGeometry(fullScan, artScan, true);
  const fallback = reason => ({'--source-frame-status':'original','--source-frame-fallback-reason':reason});
  if (!typography || !printing) return fallback('printing-metadata');
  if (!style['--printed-layout']) return fallback(!matchArtBounds(fullScan,artScan) ? 'art-registration' : 'text-regions');
  const titlePanel = future ? {kind:'integrated'} : {kind:style['--title-panel-kind'] || classifyTitlePanel(fullScan).kind}, typePanel = future ? {kind:'integrated'} : classifyTypePanel(fullScan);
  style['--title-panel-kind'] = titlePanel.kind;
  style['--type-panel-kind'] = typePanel.kind;
  const measuredBoxes = JSON.parse(style['--printed-layout']);
  const statsBox = printing.power != null && printing.toughness != null ? detectPrintedStats(fullScan) : null;
  for (const [name, box] of Object.entries({ ...measuredBoxes, stats: statsBox })) {
    if (name === 'art' || !box) continue;
    const inset = name === 'stats' ? -6 : 6;
    const x = Math.max(0, Math.ceil(box.x + inset)), y = Math.max(0, Math.ceil(box.y + inset));
    const width = Math.min(canvas.width - x, Math.floor(box.width - inset * 2));
    const height = Math.min(canvas.height - y, Math.floor(box.height - inset * 2));
    if (width <= 0 || height <= 0) continue;
    const region = ctx.getImageData(x, y, width, height);
    // A textless box (basic lands, watermark-only panels) has no glyph cluster
    // to sample: its texture would pick an arbitrary ink. Type lettering shares
    // the panel material, so borrow its ink instead.
    if (name === 'rules' && !printedGlyphHeight(region) && style['--sampled-type-ink']) {
      style['--sampled-rules-ink'] = style['--sampled-type-ink'];
      continue;
    }
    style[`--sampled-${name}-ink`] = `rgb(${sectionInk(region).join(',')})`;
  }
  let setSymbol=null;
  if(style['--printed-layout']) {
    const type=JSON.parse(style['--printed-layout']).type;
    if(setSymbolUrl)try {
      const symbol=await loadImage(setSymbolUrl),icon=document.createElement('canvas');
      icon.width=48;icon.height=Math.max(1,Math.round(symbol.height*48/symbol.width));
      const iconCtx=icon.getContext('2d',{willReadFrequently:true});iconCtx.drawImage(symbol,0,0,icon.width,icon.height);
      setSymbol=locateSetSymbol(fullScan,type,iconCtx.getImageData(0,0,icon.width,icon.height));
    }catch { /* Keep a conservative symbol slot if the SVG is unavailable. */ }
    const stop=setSymbol?setSymbol.x-5:fullScan.width*.855-5;
    style['--printed-type-text-width']=`${Math.max(40,stop-type.x-7)/fullScan.width*100}cqw`;
    if(setSymbol)style['--printed-set-symbol-bounds']=JSON.stringify(setSymbol);
  }
  let manaMatch=null,icons=[];
  if(style['--printed-layout']) {
    const box=JSON.parse(style['--printed-layout']).title;
    try {icons=await manaTemplates(printing?.mana_cost);manaMatch=locateManaSymbols(fullScan,box,icons,future ? {vertical:true,bounds:{x:fullScan.width*.09,y:fullScan.height*.125,width:fullScan.width*.13,height:fullScan.height*.40}} : {});}catch { /* Keep font masks if no reliable SVG registration is available. */ }
    // Unregistered mana may live outside the title (for example future frames).
    // Never move it to a conventional title slot or leave a duplicate behind.
    if (printing.mana_cost && !manaMatch) return fallback('mana-registration');
    if(manaMatch) {
      if (future) for (const symbol of manaMatch.symbols) {
        // Preserve unusual ink colors and disc treatments from this printing.
        const sprite=document.createElement('canvas');
        sprite.width=symbol.width; sprite.height=symbol.height;
        const spriteCtx=sprite.getContext('2d');
        spriteCtx.beginPath();spriteCtx.arc(symbol.width/2,symbol.height/2,symbol.width/2,0,Math.PI*2);spriteCtx.clip();
        spriteCtx.drawImage(canvas,symbol.x,symbol.y,symbol.width,symbol.height,0,0,symbol.width,symbol.height);
        symbol.image=sprite.toDataURL();
      }
      style['--printed-mana-symbols']=JSON.stringify(manaMatch);
      const first=manaMatch.symbols[0];
      style['--printed-mana-center-y']=first.y+first.height/2-box.y;
    }
    // Integrated retro titles keep their established alignment. Enclosed bars
    // use a verified text baseline, independently of the symbol row's size.
    if(titlePanel?.kind==='panel') {
      const x=Math.round(fullScan.width*.08),y=Math.max(0,Math.floor(box.y-3)),w=Math.round(fullScan.width*.6),h=Math.ceil(box.height+6);
      const analysis=analyzeSection(ctx.getImageData(x,y,w,h),{minGlyphHeight:6});
      if(analysis.glyphBounds)style['--printed-title-baseline']=y+analysis.glyphBounds.bottom;
    }
  }
  if(style['--printed-layout']) {
    const stats=statsBox;
    if (typography && printing) {
      const boxes = JSON.parse(style['--printed-layout']);
      // Fit the complete printed line, including ascenders and descenders,
      // inside its detected panel. A fixed scan strip can include artwork.
      for (const section of ['title', 'type', 'stats']) {
        const content = section === 'title' ? printing.printed_name || printing.name
          : section === 'type' ? printing.printed_type_line || printing.type_line
          : printing.power != null && printing.toughness != null ? `${printing.power}/${printing.toughness}` : '';
        if (!content || section === 'stats' && !stats) continue;
        let bounds = stats;
        if (section !== 'stats') {
          const box = boxes[section], stop = section === 'title' ? (future ? box.x+box.width : manaMatch?.symbols[0]?.x) : setSymbol?.x;
          const enclosed = (section === 'title' ? titlePanel : typePanel)?.kind === 'panel';
          const insetX = enclosed ? 6 : 0, insetY = enclosed ? 2 : 0;
          // A rounded title's detected rail can start inside the first capital.
          // Include the space just outside that estimate so the connected-component
          // scan sees complete glyphs; clipped components are deliberately rejected.
          const x = Math.max(0, Math.ceil(box.x + (enclosed && section === 'title' ? -6 : insetX))), y = Math.ceil(box.y + insetY);
          const right = Math.floor(Math.min(box.x + box.width - insetX, (stop ?? fullScan.width * (section === 'title' ? .78 : .855)) - 4));
          const measured = printedTextBounds(ctx.getImageData(x, y, right - x, Math.floor(box.height - insetY * 2)));
          if (!measured) { if(future)return fallback('text-registration'); continue; }
          bounds = {x:x+measured.x, y:y+measured.y, width:measured.right-measured.x, height:measured.bottom-measured.y};
        }
        ctx.font = `${section === 'stats' ? typography.style['--card-stats-weight'] : typography.titleWeight} 100px ${typography[section]}`;
        const metrics = ctx.measureText(content);
        const inkWidth = metrics.actualBoundingBoxLeft + metrics.actualBoundingBoxRight;
        const inkHeight = metrics.actualBoundingBoxAscent + metrics.actualBoundingBoxDescent;
        const size = Math.min(bounds.width / inkWidth, bounds.height / inkHeight) * 100;
        if (!Number.isFinite(size) || size < 10 || size > 40) continue;
        style[`--printed-${section}-font-size`] = `${size / fullScan.width * 100}cqw`;
        style[`--printed-${section}-text-bounds`] = JSON.stringify(bounds);
        style[`--printed-${section}-baseline`] = bounds.y + bounds.height - metrics.actualBoundingBoxDescent * size / 100;
      }
      const flavorLine = measureFlavorFirstLine(ctx, boxes.rules, printing.flavor_text, typography.rules);
      if (flavorLine) {
        style['--printed-flavor-font-size'] = `${flavorLine.size / fullScan.width * 100}cqw`;
        style['--printed-flavor-first-line'] = JSON.stringify(flavorLine);
      }
      const firstLine = measureRulesFirstLine(ctx, boxes.rules, printing.printed_text || printing.oracle_text, typography.rules);
      if (firstLine) {
        style['--printed-rules-first-line'] = JSON.stringify(firstLine);
        style['--printed-rules-font-size'] = `${firstLine.size / fullScan.width * 100}cqw`;
        ctx.font = `400 ${firstLine.size}px ${typography.rules}`;
        const metrics = ctx.measureText(firstLine.line);
        const lineHeight = firstLine.lineHeight && firstLine.lineHeight >= firstLine.height
          && firstLine.lineHeight <= firstLine.size * 1.5 ? firstLine.lineHeight : firstLine.size * 1.24;
        style['--printed-rules-line-height'] = lineHeight / firstLine.size;
        const inkTop = (lineHeight - metrics.fontBoundingBoxAscent - metrics.fontBoundingBoxDescent) / 2
          + metrics.fontBoundingBoxAscent - metrics.actualBoundingBoxAscent;
        style['--printed-rules-padding-top'] = `${Math.max(4, firstLine.y - boxes.rules.y - inkTop) / fullScan.width * 100}cqw`;
        // Text the printing sets on the box's centre line (dual-land promos)
        // stands off both edges by the same margin. That inset is not a
        // padding: kept as one it squeezes the live text into a narrow column,
        // where a longer translation wraps and its next line starts back at
        // the box edge. Centre the live text instead and give it the whole box.
        const leftInset = firstLine.x - boxes.rules.x;
        const rightInset = boxes.rules.x + boxes.rules.width - (firstLine.x + firstLine.width);
        // A left-aligned printing starts its rules text at the same frame
        // inset as its type line, and a line that merely fills most of the box
        // leaves similar margins either side whatever its alignment. Centred
        // text clears that inset and is symmetric to within a rounding error.
        const printedType = JSON.parse(style['--printed-type-text-bounds'] || 'null');
        const typeInset = printedType ? printedType.x - boxes.type.x : 0;
        const centred = Math.abs(leftInset - rightInset) < boxes.rules.width * .01
          && leftInset > typeInset + boxes.rules.width * .03;
        if (centred) style['--printed-rules-text-align'] = 'center';
        // A left-aligned printing indents its text by a frame inset, never by
        // a fraction of the box. Cap the padding so a mismeasured line cannot
        // reflow the live text either.
        const padding = centred ? 6
          : Math.min(Math.max(6, leftInset + metrics.actualBoundingBoxLeft), boxes.rules.width * .12);
        style['--printed-rules-padding-left'] = `${padding / fullScan.width * 100}cqw`;
      }
    }
    const masked=maskSourceFrame(fullScan,JSON.parse(style['--printed-layout']),stats,detectStatsPanelBounds(fullScan,stats),typography ? (patch,options)=>fontGuidedPanel(patch,{
      family:typography[options.section==='footer'?'rules':options.section]||typography.rules,
      weight:['title','type'].includes(options.section)?typography.titleWeight:options.section==='stats'?typography.style['--card-stats-weight']:400,
      section:options.section,
      allowItalic:options.section==='rules',symbols:options.section==='rules'||options.section==='title'&&Boolean(printing.mana_cost)&&!manaMatch,
      text:options.section==='rules'?`${printing?.printed_text||printing?.oracle_text||''} ${printing?.flavor_text||''}`:options.section==='title'?(printing?.printed_name||printing?.name):options.section==='type'?(printing?.printed_type_line||printing?.type_line):options.section==='footer'?`${printing?.artist||''} Illus. Ilus. Wizards of the Coast Inc.`:`${printing?.power||''}/${printing?.toughness||''}`,
    }):reconstructPanel,{title:titlePanel?.kind,type:typePanel?.kind,fontGuided:!!typography,setSymbol,manaMatch,icons,preserveRules:basicLandBoxIsTextless(printing),textBounds:Object.fromEntries(['title','type'].map(name=>[name,JSON.parse(style[`--printed-${name}-text-bounds`]||'null')]))});
    if(masked) {
      const original=document.createElement('canvas');original.width=masked.width;original.height=masked.height;
      original.getContext('2d').putImageData(new ImageData(masked.data,masked.width,masked.height),0,0);
      style['--source-frame-mask-method']=typography?'font-template':'contrast';
      style['--source-frame-image']=`url("${original.toDataURL()}")`;
    }
  }
  if (!style['--source-frame-image']) return fallback('glyph-mask');
  style['--source-frame-status'] = 'masked';
  if (style['--printed-scan-width']) {
    // Typography, bevels, and P/T offsets use the same scale as the boxes,
    // including previews constrained by height instead of width.
    for (const [key, value] of Object.entries(style)) {
      if (typeof value === 'string' && !value.includes('url(')) {
        style[key] = value.replace(/(-?\d+(?:\.\d+)?)cqw/g, 'calc($1 * var(--card-frame-width-unit))');
      }
    }
  }
  return style;
}

export function sampleCardFrameColors(fullUrl, { typography, printing, setSymbolUrl } = {}) {
  if (!fullUrl) return Promise.resolve(null);
  const key = `${typography ? "font-template" : "contrast"}:source-mask:${fullUrl}`;
  if (cache.has(key)) return cache.get(key);
  const request = sample(fullUrl, typography, printing, setSymbolUrl).catch(() => { cache.delete(key); return null; });
  cache.set(key, request);
  if (cache.size > 48) cache.delete(cache.keys().next().value);
  return request;
}
