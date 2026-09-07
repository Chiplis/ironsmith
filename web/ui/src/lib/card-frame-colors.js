import { reconstructFrameMaterial } from './card-frame-material.js';
import { sampleInnerFrameBevel, detectEmbeddedArtFrame } from './card-border-analysis.js';

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

  return {
    ink: glyphs >= 2 && ink.length >= 24 ? materialColor(ink) : light > .35 ? [23, 24, 25] : [245, 241, 230],
    glyphBounds,
    glyphHeight: cluster.length >= 4 ? cluster[Math.floor((cluster.length - 1) * .8)] : null,
  };
}

export function sectionInk(region) {
  const ink = analyzeSection(region).ink;
  return luminance(ink) > luminance(materialColor(region.data))
    ? [255, 255, 255] : [0, 0, 0];
}
export function printedGlyphHeight(region) { return analyzeSection(region).glyphHeight; }

function loadImage(url) {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.crossOrigin = 'anonymous'; image.referrerPolicy = 'no-referrer';
    const timer = setTimeout(() => { image.src = ''; reject(new Error('Card colors timed out')); }, 12000);
    image.onload = () => { clearTimeout(timer); resolve(image); };
    image.onerror = () => { clearTimeout(timer); reject(new Error('Card colors unavailable')); };
    image.src = url;
  });
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

function railStyle(image, integrated) {
  if (!image || !integrated) return {};
  const canvas = document.createElement('canvas');
  canvas.width = 488; canvas.height = Math.round(image.height * 488 / image.width);
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  ctx.drawImage(image, 0, 0, canvas.width, canvas.height);
  const pixels = ctx.getImageData(0, 0, canvas.width, canvas.height);
  const bottomHeight = artBottomRail(pixels);
  const joinStyle = {};
  if (bottomHeight) {
    const join = document.createElement('canvas');
    join.width = canvas.width; join.height = bottomHeight;
    const joinCtx = join.getContext('2d');
    joinCtx.translate(0, join.height); joinCtx.scale(1, -1);
    joinCtx.drawImage(canvas, 0, canvas.height - bottomHeight, canvas.width, bottomHeight, 0, 0, join.width, join.height);
    joinStyle['--art-top-rail'] = `url("${join.toDataURL()}")`;
    joinStyle['--art-top-rail-height'] = `${bottomHeight / canvas.height * 100}%`;
  }
  const enclosure = detectEmbeddedArtFrame(pixels);
  if (enclosure) Object.assign(joinStyle, {
    '--art-frame-enclosure': 'detected',
    '--art-frame-left': `${enclosure.left.outer / canvas.width * 100}%`,
    '--art-frame-right': `${enclosure.right.outer / canvas.width * 100}%`,
    '--art-frame-bottom': `${enclosure.bottom.outer / canvas.height * 100}%`,
    '--art-frame-title-left': `${enclosure.left.inner / canvas.width * 100}%`,
    '--art-frame-title-right': `${enclosure.right.inner / canvas.width * 100}%`,
  });
  return joinStyle;
}

// Remove high-contrast print from a whole panel before sizing it to our layout.
// Estimate paper in vertical bands so hybrid materials retain their direction.
export function reconstructPanel({ data, width, height }, { removeSeparators = false } = {}) {
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
  if (clean.length < count * .2) return null;
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

function textureStyle(region, name) {
  const panel = reconstructPanel(region);
  if (!panel) return {};
  const canvas = document.createElement('canvas');
  canvas.width = panel.width; canvas.height = panel.height * (name === 'type' ? 2 : 1);
  // Suppress scan-wide bevels and lines while keeping local grain and
  // left-to-right material changes, including hybrid frame colors.
  const means = Array.from({ length: panel.height }, (_, y) => {
    const rgb = [0, 0, 0];
    for (let x = 0; x < panel.width; x++) for (let c = 0; c < 3; c++) rgb[c] += panel.data[(y * panel.width + x) * 4 + c] / panel.width;
    return rgb;
  });
  const average = [0, 1, 2].map(c => means.reduce((sum, rgb) => sum + rgb[c], 0) / panel.height);
  const normalized = panel.data.slice();
  for (let y = 0; y < panel.height; y++) for (let x = 0; x < panel.width; x++) for (let c = 0; c < 3; c++) {
    const p = (y * panel.width + x) * 4 + c;
    normalized[p] += average[c] - means[y][c];
  }
  canvas.getContext('2d').putImageData(new ImageData(normalized, panel.width, panel.height), 0, 0);
  if (name === 'type') {
    const ctx = canvas.getContext('2d');
    ctx.translate(0, canvas.height); ctx.scale(1, -1);
    ctx.drawImage(canvas, 0, 0, panel.width, panel.height, 0, 0, panel.width, panel.height);
  }
  const material = materialColor(panel.data);
  return {
    [`--sampled-${name}-veil`]: `linear-gradient(rgba(${material.join(',')}, .22), rgba(${material.join(',')}, .22))`,
    [`--sampled-${name}-texture`]: `url("${canvas.toDataURL()}")`,
    [`--sampled-${name}-texture-size`]: name === 'type' ? '100% auto' : '100% 100%',
    ...(name === 'type' ? { '--sampled-bar-ink': `rgb(${sectionInk(region).join(',')})` } : {}),
  };
}

// Classify the printing, not the card: an enclosed title has contrasting
// strokes on BOTH sides of its text-free margins. Shared textured frames do not.
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

function rulesBottomStyle(canvas) {
  const ctx = canvas.getContext('2d');
  const edge = rulesBottomEdge(ctx.getImageData(0, 0, canvas.width, canvas.height));
  if (!edge) return {};
  const style = {};
  for (const [name, x, width] of [['left', edge.x, edge.corner], ['middle', edge.x + edge.corner, edge.width - edge.corner * 2], ['right', edge.x + edge.width - edge.corner, edge.corner]]) {
    const strip = document.createElement('canvas');
    strip.width = width; strip.height = edge.height;
    strip.getContext('2d').drawImage(canvas, x, edge.y, width, edge.height, 0, 0, width, edge.height);
    style[`--rules-bottom-${name}`] = `url("${strip.toDataURL()}")`;
  }
  style['--rules-bottom-height'] = `${edge.height / 488 * 100}cqw`;
  style['--rules-bottom-corner'] = `${edge.corner / 488 * 100}cqw`;
  return style;
}

// Locate conventional panels using sustained transitions, not printed glyphs.
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
  const x=left.position-1, y=top.position-1, w=right.position-left.position+3, h=bottom.position-top.position+3;
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
  return best;
}

function wholePanelStyle(canvas, scan, section) {
  const contour = section === 'rules' ? null : detectEnclosedPanelBounds(scan, section);
  const panel = section === 'title' ? classifyTitlePanel(scan) : classifyTypePanel(scan);
  const generatedOutline = section !== 'rules' || panel.kind === 'panel' || classifyTitlePanel(scan).kind === 'panel';
  // A flat rules box may have no detectable lower edge (for example silver
  // artifact frames). A generated rim needs only conventional proportions;
  // never manufacture a crop containing uncertain original edge pixels.
  const bounds = contour || detectPanelBounds(scan, section) || (section === 'rules' && generatedOutline ? {
    x:Math.round(scan.width*.09),y:Math.round(scan.height*.625),
    width:Math.round(scan.width*.82),height:Math.round(scan.height*.29),
  } : null);
  if (!bounds) return {};
  // Integrated rules retain their decorative edge; enclosed panels get a clean
  // vector outline with sampled colors around the reconstructed material.
  const inset = section === 'rules' ? 12 : 10;
  const cleanX = 7, cleanY = 3;
  const source = document.createElement('canvas');
  source.width=bounds.width; source.height=bounds.height;
  const ctx=source.getContext('2d', {willReadFrequently:true});
  ctx.drawImage(canvas,bounds.x,bounds.y,bounds.width,bounds.height,0,0,bounds.width,bounds.height);
  // Generated rules panels use the central paper, excluding the bottom stamp,
  // P/T box, and the original edge bevels from their texture donors.
  const inner=generatedOutline && section === 'rules'
    ? canvas.getContext('2d').getImageData(Math.floor(scan.width*.12),Math.floor(scan.height*.64),Math.floor(scan.width*.76),Math.floor(scan.height*.20))
    : ctx.getImageData(cleanX,cleanY,bounds.width-cleanX*2,bounds.height-cleanY*2);
  let cleaned=reconstructPanel(inner, {removeSeparators: section === 'rules'});
  if (!cleaned) {
    if (!generatedOutline) return {};
    // Dense print may leave too few trustworthy donors. Keep the enclosure
    // and use its dominant paper instead of reintroducing printed fragments.
    cleaned={width:1,height:1,data:new Uint8ClampedArray([...materialColor(inner.data),255])};
  }
  if (generatedOutline) {
    // Never carry unknown edge pixels or stamp fragments into an enclosed
    // panel. Keep its cleaned texture and generate a continuous vector rim.
    const texture = document.createElement('canvas');
    texture.width=cleaned.width; texture.height=cleaned.height;
    texture.getContext('2d').putImageData(new ImageData(cleaned.data,cleaned.width,cleaned.height),0,0);
    const radius=section==='rules'?2:Math.min(12,bounds.height*.25);
    const stroke=Math.max(1,Math.min(2.5,panel.stroke*488/100));
    const color=rgb=>`rgb(${rgb.join(',')})`;
    const svg=`<svg xmlns="http://www.w3.org/2000/svg" width="${bounds.width}" height="${bounds.height}" viewBox="0 0 ${bounds.width} ${bounds.height}"><defs><clipPath id="panel"><rect x="1" y="1" width="${bounds.width-2}" height="${bounds.height-2}" rx="${radius}"/></clipPath></defs><image href="${texture.toDataURL()}" width="${bounds.width}" height="${bounds.height}" preserveAspectRatio="none" clip-path="url(#panel)"/><rect x="${stroke/2}" y="${stroke/2}" width="${bounds.width-stroke}" height="${bounds.height-stroke}" rx="${radius}" fill="none" stroke="${color(panel.border)}" stroke-width="${stroke}"/><rect x="${stroke+1}" y="${stroke+1}" width="${bounds.width-2*stroke-2}" height="${bounds.height-2*stroke-2}" rx="${Math.max(1,radius-stroke)}" fill="none" stroke="${color(panel.highlight)}" stroke-opacity=".65"/></svg>`;
    return {
      [`--whole-${section}-image`]: `url("data:image/svg+xml,${encodeURIComponent(svg)}")`,
      [`--whole-${section}-slice`]: `${inset} 16 ${inset} 16 fill`,
      [`--whole-${section}-width`]: `${inset/488*100}cqw ${16/488*100}cqw`,
    };
  }
  // Restore only actual edge pixels and corner shapes after cleaning text.
  const original = ctx.getImageData(0,0,bounds.width,bounds.height);
  ctx.putImageData(new ImageData(cleaned.data, cleaned.width, cleaned.height),cleanX,cleanY);
  const cornerSize = 12;
  for (const x of [0,bounds.width-cornerSize]) for (const y of [0,bounds.height-cornerSize]) {
    ctx.putImageData(original,0,0,x,y,cornerSize,cornerSize);
  }
  const corner = 16;
  return {
    [`--whole-${section}-image`]: `url("${source.toDataURL()}")`,
    [`--whole-${section}-slice`]: `${inset} ${corner} ${inset} ${corner} fill`,
    [`--whole-${section}-width`]: `${inset/488*100}cqw ${corner/488*100}cqw`,
  };
}

// Locate the exact art crop within its full printing. Comparing interior
// samples avoids mistaking decorative frame edges for the artwork boundary.
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
  for(let w=Math.round(scan.width*.8);w<=scan.width*.95;w+=3) {
    for(let x=Math.round((scan.width-w)/2)-6;x<=(scan.width-w)/2+6;x+=3) {
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

function geometryStyle(scan,art,conventional) {
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
  const artBox=matchArtBounds(scan,art);
  if(!artBox) return style;
  const glyphBox=(section)=>{
    const x=Math.floor(scan.width*.12),y=Math.floor(scan.height*(section==='title'?.035:.55));
    const w=Math.floor(scan.width*.65),h=Math.floor(scan.height*.065),data=new Uint8ClampedArray(w*h*4);
    for(let row=0;row<h;row++) data.set(scan.data.subarray(((y+row)*scan.width+x)*4,((y+row)*scan.width+x+w)*4),row*w*4);
    const bounds=analyzeSection({data,width:w,height:h},{minGlyphHeight:Math.floor(scan.height*.015)}).glyphBounds;
    const padding=bounds?Math.round((bounds.bottom-bounds.y)*.4):0;
    return bounds?{x:x+bounds.x,y:y+bounds.y-padding,width:bounds.right-bounds.x,height:bounds.bottom-bounds.y+padding*2}:null;
  };
  // Integrated bars use the printed glyph block plus breathing room when
  // there is no enclosing stroke to measure.
  const title=classifyTitlePanel(scan).kind==='panel'?(detectEnclosedPanelBounds(scan,'title') || detectPanelBounds(scan,'title')):glyphBox('title');
  const type=detectEnclosedPanelBounds(scan,'type') || detectPanelBounds(scan,'type') || glyphBox('type');
  // A shared type/rules edge can be detected on both sides of its bevel.
  // Allocate that overlap to the type bar instead of rejecting both boxes.
  if (rules && type && rules.y < type.y + type.height
    && type.y + type.height - rules.y < scan.height * .04) {
    const top = type.y + type.height;
    rules = {...rules, y: top, height: rules.y + rules.height - top};
  }
  // All regions share source coordinates and one uniform scale. Integrated
  // bars have no enclosing sides: use the art's span, not the glyph width.
  if (title && type && rules && title.y + title.height <= artBox.y + 3
    && artBox.y + artBox.height <= type.y + 3 && type.y + type.height <= rules.y + 3) {
    const titleBox = classifyTitlePanel(scan).kind === 'panel' ? title : {...title, x:artBox.x, width:artBox.width};
    const typeBox = classifyTypePanel(scan).kind === 'panel' ? type : {...type, x:rules.x, width:rules.width};
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

function innerFrameBorderStyle(scan) {
  const rim = sampleInnerFrameBevel(scan);
  if (!rim) return {};
  return {
    '--inner-frame-border-kind': rim.kind,
    '--frame-border-bounds': JSON.stringify(rim.bounds),
    '--frame-border-x': rim.bounds.x,
    '--frame-border-y': rim.bounds.y,
    '--frame-border-width': rim.bounds.width,
    '--frame-border-height': rim.bounds.height,
    '--frame-border-bottom': scan.height-rim.bounds.y-rim.bounds.height,
    '--frame-border-right': scan.width-rim.bounds.x-rim.bounds.width,
    '--frame-interior-radius': rim.innerRadius,
    ...(rim.outerRadius == null ? {} : {'--frame-outer-radius': `${rim.outerRadius/scan.width*100}cqw`}),
    ...Object.fromEntries(Object.entries(rim.colors).map(([side,rgb])=>[`--frame-border-${side}-color`, `rgb(${rgb.join(',')})`])),
    '--inner-frame-border-confidence': rim.confidence,
    '--inner-frame-bevel-profile': JSON.stringify(rim.profiles),
  };
}

async function sample(fullUrl, textures) {
  const artUrl = /^https:\/\/cards\.scryfall\.io\/normal\//.test(fullUrl) ? fullUrl.replace('/normal/', '/art_crop/') : '';
  const [image, art] = await Promise.all([loadImage(fullUrl), textures && artUrl ? loadImage(artUrl).catch(() => null) : null]);
  const canvas = document.createElement('canvas');
  canvas.width = 488; canvas.height = Math.round(image.height * 488 / image.width);
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  ctx.drawImage(image, 0, 0, canvas.width, canvas.height);
  const patch = (x, y, w, h) => ctx.getImageData(Math.floor(x * canvas.width), Math.floor(y * canvas.height), Math.max(1, Math.floor(w * canvas.width)), Math.max(1, Math.floor(h * canvas.height)));
  // Broad conventional-frame regions, with separate left/right material colors
  // for hybrid frames. The type material also supplies the shared horizontal bar texture.
  const regions = {
    shell: [0.035, 0.12, 0.028, 0.72],
    title: [0.10, 0.045, 0.78, 0.045],
    type: [0.10, 0.557, 0.78, 0.045],
    rules: [0.12, 0.64, 0.76, 0.22],
    stats: [0.77, 0.895, 0.15, 0.045],
  };
  const cssColor = rgb => `rgb(${rgb.join(',')})`;
  const scan = textures ? ctx.getImageData(0, 0, canvas.width, canvas.height) : null;
  const titlePanel = scan ? classifyTitlePanel(scan) : null;
  const typePanel = scan ? classifyTypePanel(scan) : null;
  const style = textures ? { ...rulesBottomStyle(canvas), ...railStyle(art, titlePanel?.kind === 'integrated'), ...textureStyle(patch(.105, .56, .71, .033), 'type') } : {};
  for (const [name, [x, y, w, h]] of Object.entries(regions)) {
    const left = cssColor(materialColor(patch(x, y, w / 2, h).data));
    const right = cssColor(materialColor(patch(x + w / 2, y, w / 2, h).data));
    style[`--sampled-${name}-paper`] = `linear-gradient(90deg, ${left} 25%, ${right} 75%)`;
    if (textures && ['rules', 'stats'].includes(name)) {
      Object.assign(style, textureStyle(patch(x, y, w, h), name));
    }
    if (name !== 'shell') {
      const region=patch(x,y,w,h);
      let ink=sectionInk(region);
      if (!textures) {
        const paper=luminance(materialColor(region.data)), text=luminance(ink);
        if ((Math.max(paper,text)+.05)/(Math.min(paper,text)+.05)<4.5) ink=paper>.179?[0,0,0]:[255,255,255];
      }
      style[`--sampled-${name}-ink`] = cssColor(ink);
    }
    if (['title', 'type', 'rules'].includes(name)) {
      const glyphRegion = name === 'title' ? patch(.085, .035, .72, .072) : name === 'type' ? patch(.09, .55, .72, .075) : patch(x, y, w, h);
      const glyphHeight = printedGlyphHeight(glyphRegion);
      if (glyphHeight) style[`--sampled-${name}-font-size`] = `calc(${glyphHeight / canvas.width * 100}cqw / var(--card-${name}-glyph-ratio, .7))`;
    }
  }
  // Color-only bars share the type material in CSS, so their ink must too.
  if (!textures) style['--sampled-bar-ink'] = style['--sampled-type-ink'];
  // The narrow vertical scan border contains pinlines, not usable panel grain.
  // Extend the reconstructed type material through the surrounding frame.
  if (style['--sampled-type-texture']) {
    style['--sampled-shell-texture'] = style['--sampled-type-texture'];
    style['--sampled-shell-texture-size'] = '100% auto';
    style['--sampled-shell-veil'] = style['--sampled-type-veil'];
  }
  if (titlePanel) {
    style['--title-panel-kind'] = titlePanel.kind;
    style['--title-panel-confidence'] = titlePanel.confidence;
    if (titlePanel.kind === 'panel') {
      Object.assign(style, textureStyle(patch(.105, .054, .71, .042), 'title'));
      style['--title-panel-border'] = cssColor(titlePanel.border);
      style['--title-panel-highlight'] = cssColor(titlePanel.highlight);
      style['--title-panel-stroke'] = `${titlePanel.stroke}cqw`;
      style['--title-panel-radius'] = `${titlePanel.radius}cqw`;
      delete style['--art-top-rail'];
      delete style['--art-top-rail-height'];
    }
  }
  if (typePanel) {
    style['--type-panel-kind'] = typePanel.kind;
    style['--type-panel-confidence'] = typePanel.confidence;
    if (typePanel.kind === 'panel') {
      Object.assign(style, textureStyle(patch(.105, .573, .71, .028), 'type-box'));
      style['--type-panel-border'] = cssColor(typePanel.border);
      style['--type-panel-highlight'] = cssColor(typePanel.highlight);
      style['--type-panel-stroke'] = `${typePanel.stroke}cqw`;
      style['--type-panel-radius'] = `${typePanel.radius}cqw`;
    }
  }
  if (scan) {
    Object.assign(style, innerFrameBorderStyle(scan));
    for (const section of ['title', 'type', 'rules']) {
      if (section === 'title' && titlePanel.kind !== 'panel') continue;
      if (section === 'type' && typePanel.kind !== 'panel') continue;
      Object.assign(style, wholePanelStyle(canvas, scan, section));
    }
    if (style['--whole-rules-image']) {
      for (const key of Object.keys(style)) if (key.startsWith('--rules-bottom-')) delete style[key];
    }
  }
  const fullScan=scan || ctx.getImageData(0,0,canvas.width,canvas.height);
  let artScan=null;
  if(art) {
    const artCanvas=document.createElement('canvas');artCanvas.width=160;artCanvas.height=Math.round(art.height*160/art.width);
    const artCtx=artCanvas.getContext('2d',{willReadFrequently:true});artCtx.drawImage(art,0,0,artCanvas.width,artCanvas.height);
    artScan=artCtx.getImageData(0,0,artCanvas.width,artCanvas.height);
  }
  Object.assign(style, geometryStyle(fullScan,artScan,textures));
  if (style['--printed-layout'] && style['--frame-border-bounds']) {
    const bounds=JSON.parse(style['--frame-border-bounds']);
    const material=reconstructFrameMaterial(fullScan,bounds,JSON.parse(style['--printed-layout']),detectPrintedStats(fullScan));
    if(material) {
      const atlas=document.createElement('canvas');atlas.width=bounds.width;atlas.height=bounds.height;
      atlas.getContext('2d').putImageData(new ImageData(material.data,material.width,material.height),-bounds.x,-bounds.y);
      style['--sampled-shell-texture']=`url("${atlas.toDataURL()}")`;
      style['--sampled-shell-texture-size']='100% 100%';
      style['--sampled-shell-veil']='linear-gradient(transparent, transparent)';
      style['--frame-material-sampling']='regional';
    }
  }
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

export function sampleCardFrameColors(fullUrl, { textures = true } = {}) {
  if (!fullUrl) return Promise.resolve(null);
  const key = `${textures ? "texture" : "color"}:${fullUrl}`;
  if (cache.has(key)) return cache.get(key);
  const request = sample(fullUrl, textures).catch(() => { cache.delete(key); return null; });
  cache.set(key, request);
  if (cache.size > 48) cache.delete(cache.keys().next().value);
  return request;
}
