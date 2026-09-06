// Printing colors and a narrow color continuation for embedded art borders.
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

function analyzeSection({ data, width, height }) {
  const paper = materialColor(data), light = luminance(paper);
  const mask = new Uint8Array(width * height), ink = [];
  for (let p = 0; p < mask.length; p++) {
    const value = luminance([data[p * 4], data[p * 4 + 1], data[p * 4 + 2]]);
    mask[p] = data[p * 4 + 3] >= 128 && (Math.max(light, value) + 0.05) / (Math.min(light, value) + 0.05) >= 2.5 ? 1 : 0;
  }
  let glyphs = 0;
  const heights = [];
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
    if (h >= 5 && w <= h * 2.5 && component.length >= h) heights.push(h);
    for (const at of component) ink.push(data[at * 4], data[at * 4 + 1], data[at * 4 + 2], 255);
  }
  // Use a repeated glyph-height cluster, not punctuation, borders, or symbols.
  let cluster = [];
  for (const h of heights) {
    const similar = heights.filter(value => Math.abs(value - h) <= 2);
    if (similar.length > cluster.length) cluster = similar;
  }
  cluster.sort((a, b) => a - b);
  return {
    ink: glyphs >= 2 && ink.length >= 24 ? materialColor(ink) : light > .35 ? [23, 24, 25] : [245, 241, 230],
    glyphHeight: cluster.length >= 4 ? cluster[Math.floor((cluster.length - 1) * .8)] : null,
  };
}

export function sectionInk(region) { return analyzeSection(region).ink; }
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

// Look for a long vertical boundary close to BOTH art edges. A cloud or a
// single straight object in the illustration is insufficient evidence.
export function artFrameRails({ data, width, height }) {
  const pixel = (x, y) => {
    const at = (Math.floor(y) * width + x) * 4;
    return [data[at], data[at + 1], data[at + 2]];
  };
  const difference = (a, b) => Math.sqrt(a.reduce((n, v, c) => n + (v - b[c]) ** 2, 0));
  const rows = Array.from({ length: 16 }, (_, i) => Math.floor(height * (0.12 + i * 0.045)));
  const edge = right => {
    let best = null;
    for (let offset = Math.max(3, Math.floor(width * 0.015)); offset < width * 0.08; offset++) {
      const x = right ? width - 1 - offset : offset;
      const changes = rows.map(y => difference(pixel(x - 2, y), pixel(x + 2, y)));
      const support = changes.filter(v => v > 32).length / rows.length;
      const score = changes.reduce((n, v) => n + Math.min(100, v), 0) / rows.length;
      if (support >= 0.8 && score >= 40 && (!best || score > best.score)) best = { offset, score };
    }
    return best;
  };
  const left = edge(false), right = edge(true);
  if (!left || !right || Math.abs(left.offset - right.offset) > width * 0.035) return null;
  const leftWidth = left.offset + 2, rightWidth = right.offset + 2;
  const strip = new Uint8ClampedArray(width * 4);
  for (let x = 0; x < width; x++) {
    if (x >= leftWidth && x < width - rightWidth) continue;
    const samples = [];
    for (const y of rows) samples.push(...pixel(x, y), 255);
    strip.set([...materialColor(samples), 255], x * 4);
  }
  return { left: leftWidth / width, right: rightWidth / width, strip };
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

function railStyle(image) {
  if (!image) return {};
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
  const rails = artFrameRails(ctx.getImageData(0, 0, canvas.width, canvas.height));
  if (!rails) return joinStyle;
  const leftWidth = Math.round(rails.left * canvas.width);
  const rightWidth = Math.round(rails.right * canvas.width);
  const stripHeight = Math.min(64, canvas.height);
  const strips = document.createElement('canvas');
  strips.width = canvas.width; strips.height = stripHeight;
  const stripCtx = strips.getContext('2d');
  // Reflect the art's top edge upward: the pixels at the title's lower edge
  // meet the same pixels at the top of the art. The text area stays transparent.
  stripCtx.translate(0, stripHeight); stripCtx.scale(1, -1);
  stripCtx.drawImage(canvas, 0, 0, leftWidth, stripHeight, 0, 0, leftWidth, stripHeight);
  stripCtx.drawImage(canvas, canvas.width - rightWidth, 0, rightWidth, stripHeight, canvas.width - rightWidth, 0, rightWidth, stripHeight);

  return {
    ...joinStyle,
    '--art-title-rails': `url("${strips.toDataURL()}")`,
    '--art-title-left-rail': `${rails.left * 100}%`,
    '--art-title-right-rail': `${rails.right * 100}%`,
  };
}

// Remove high-contrast print from a whole panel before sizing it to our layout.
// Estimate paper in vertical bands so hybrid materials retain their direction.
export function reconstructPanel({ data, width, height }) {
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

function wholePanelStyle(canvas, scan, section) {
  const bounds = detectPanelBounds(scan, section);
  if (!bounds) return {};
  // Retain the original outline and corners; mask only the text-bearing inset.
  const inset = section === 'rules' ? 12 : 10;
  const cleanX = 7, cleanY = 3;
  const source = document.createElement('canvas');
  source.width=bounds.width; source.height=bounds.height;
  const ctx=source.getContext('2d', {willReadFrequently:true});
  ctx.drawImage(canvas,bounds.x,bounds.y,bounds.width,bounds.height,0,0,bounds.width,bounds.height);
  const inner=ctx.getImageData(cleanX,cleanY,bounds.width-cleanX*2,bounds.height-cleanY*2);
  const cleaned=reconstructPanel(inner);
  if (!cleaned) return {};
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
  const style = textures ? { ...rulesBottomStyle(canvas), ...railStyle(art), ...textureStyle(patch(.105, .56, .71, .033), 'type') } : {};
  for (const [name, [x, y, w, h]] of Object.entries(regions)) {
    const left = cssColor(materialColor(patch(x, y, w / 2, h).data));
    const right = cssColor(materialColor(patch(x + w / 2, y, w / 2, h).data));
    style[`--sampled-${name}-paper`] = `linear-gradient(90deg, ${left} 25%, ${right} 75%)`;
    if (textures && ['rules', 'stats'].includes(name)) {
      Object.assign(style, textureStyle(patch(x, y, w, h), name));
    }
    if (name !== 'shell') style[`--sampled-${name}-ink`] = cssColor(sectionInk(patch(x, y, w, h)));
    if (['title', 'type', 'rules'].includes(name)) {
      const glyphRegion = name === 'title' ? patch(.085, .035, .72, .072) : name === 'type' ? patch(.09, .55, .72, .075) : patch(x, y, w, h);
      const glyphHeight = printedGlyphHeight(glyphRegion);
      if (glyphHeight) style[`--sampled-${name}-font-size`] = `calc(${glyphHeight / canvas.width * 100}cqw / var(--card-${name}-glyph-ratio, .7))`;
    }
  }
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
      delete style['--art-title-rails'];
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
    for (const section of ['title', 'type', 'rules']) {
      if (section === 'title' && titlePanel.kind !== 'panel') continue;
      if (section === 'type' && typePanel.kind !== 'panel') continue;
      Object.assign(style, wholePanelStyle(canvas, scan, section));
    }
    if (style['--whole-rules-image']) {
      for (const key of Object.keys(style)) if (key.startsWith('--rules-bottom-')) delete style[key];
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
