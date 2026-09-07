// Border geometry is separate from texture reconstruction. A border is a
// coherent band around an enclosure, not whichever scan line is strongest.
const SIDES = ['top', 'right', 'bottom', 'left'];
const median = values => {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted.length ? sorted[Math.floor(sorted.length / 2)] : 0;
};
const colorDistance = (a, b) => Math.hypot(...a.map((v, c) => v - b[c]));
const colorMedian = colors => [0, 1, 2].map(c => median(colors.map(rgb => rgb[c])));
const clamp = (value, low, high) => Math.max(low, Math.min(high, value));

function reader({data, width, height}) {
  return (x, y) => {
    const p = (clamp(Math.round(y), 0, height - 1) * width + clamp(Math.round(x), 0, width - 1)) * 4;
    return {rgb: [data[p], data[p + 1], data[p + 2]], alpha: data[p + 3]};
  };
}

function pointOnRay(side, along, depth, width, height) {
  if (side === 'top') return [along * (width - 1), depth];
  if (side === 'bottom') return [along * (width - 1), height - 1 - depth];
  if (side === 'left') return [depth, along * (height - 1)];
  return [width - 1 - depth, along * (height - 1)];
}

// Sample many inward rays away from the corners. A glyph, set symbol, or
// individual highlight cannot establish a side, much less an entire border.
function measureSide(scan, side, pixel) {
  const {width, height} = scan;
  const rays = Array.from({length: 36}, (_, i) => .14 + i * .72 / 35);
  const edgeDepth = Math.max(1, Math.round(width * .004));
  const edgePixels = rays.map(t => pixel(...pointOnRay(side, t, edgeDepth, width, height)));
  const opaque = edgePixels.filter(p => p.alpha >= 240).map(p => p.rgb);
  const color = colorMedian(opaque);
  const uniformity = opaque.filter(rgb => colorDistance(rgb, color) <= 24).length / rays.length;
  const depths = [];
  if (uniformity >= .75) for (const t of rays) {
    const start = pixel(...pointOnRay(side, t, edgeDepth, width, height));
    if (start.alpha < 240 || colorDistance(start.rgb, color) > 24) continue;
    let run = 0;
    for (let d = edgeDepth + 1; d <= width * .10; d++) {
      const next = pixel(...pointOnRay(side, t, d, width, height));
      if (next.alpha >= 240 && colorDistance(next.rgb, color) > 32) run++;
      else run = 0;
      if (run === 3) {depths.push(d - 2); break;}
    }
  }
  let thickness = median(depths);
  const agreeing = depths.filter(d => Math.abs(d - thickness) <= Math.max(2, width * .008));
  let support = agreeing.length / rays.length;
  // A dark frame can nearly match its black border. Look for a coherent
  // transition across the rays, even when the absolute color change is small.
  if (uniformity >= .85 && support < .6) {
    let best = null;
    for (let d = Math.round(width * .015); d <= width * .08; d++) {
      const changes = [], starts = [];
      for (const t of rays) {
        const before = pixel(...pointOnRay(side, t, d - 2, width, height)).rgb;
        const after = pixel(...pointOnRay(side, t, d + 2, width, height)).rgb;
        changes.push(colorDistance(before, after));
        starts.push(colorDistance(before, color));
      }
      const edgeSupport = changes.filter(v => v > 14).length / rays.length;
      const score = median(changes);
      const flatOutside = starts.filter(v => v < 24).length / rays.length;
      if (edgeSupport >= .7 && flatOutside >= .75 && score >= 16 && (!best || score > best.score)) best = {depth: d, score, support: edgeSupport};
    }
    if (best) {thickness = best.depth; support = best.support;}
  }
  // The first opaque material pixel can lie after a dark bevel. Refine back
  // to its onset using the median cross-section, before the textured interior.
  if (support >= .6 && thickness) {
    const profile = d => colorMedian(rays.map(t => pixel(...pointOnRay(side, t, d, width, height)).rgb));
    for (let d = Math.max(edgeDepth + 1, thickness - 4); d <= thickness; d++) {
      const before = profile(d - 1), after = profile(d);
      if (colorDistance(before, color) <= 28 && colorDistance(after, color) > 12 && colorDistance(before, after) > 9) {thickness = d; break;}
    }
  }
  return {color, uniformity, thickness, support, samples: rays.length};
}

// Fit several rays in each corner to a circular contour. A solid corner
// matching the border is ambiguous, not evidence for a particular radius.
function cornerRadius(scan, bounds, outside, requireVisibleCorner) {
  const pixel = reader(scan), limit = Math.max(4, Math.round(scan.width * .065));
  const radii = [];
  for (const [right, bottom] of [[false,false],[true,false],[false,true],[true,true]]) {
    const corner = pixel(bounds.x+(right?bounds.width-1:0), bounds.y+(bottom?bounds.height-1:0));
    if (requireVisibleCorner && !outside(corner)) continue;
    const samples = [];
    for (let y=0; y<limit; y+=2) for (let x=0; x<limit; x+=2) {
      const p = pixel(bounds.x + (right ? bounds.width-1-x : x), bounds.y + (bottom ? bounds.height-1-y : y));
      samples.push({x:x+.5,y:y+.5,out:outside(p)});
    }
    const count = samples.filter(p=>p.out).length;
    if (requireVisibleCorner && (count < 3 || count > samples.length*.9)) continue;
    let best = {radius:0, error:Infinity};
    for (let radius=0; radius<=limit; radius++) {
      const error = samples.filter(p=>{
        const dx=Math.max(0,radius-p.x),dy=Math.max(0,radius-p.y);
        return (dx*dx+dy*dy > radius*radius) !== p.out;
      }).length / samples.length;
      if (error < best.error) best={radius,error};
    }
    if (best.error <= .12) radii.push(best.radius);
  }
  if (radii.length < 2) return null;
  const radius=median(radii);
  return radii.filter(r=>Math.abs(r-radius)<=3).length>=2 ? radius : null;
}

// Locate the physical border's interior, retaining its bounds and color.
// Locate the material immediately INSIDE the physical card border. The
// outside band also supplies the rendered margin and corner evidence.
export function detectInnerFrameBorder(scan) {
  if (!scan || scan.width < 64 || scan.height / scan.width < 1.2) return null;
  const pixel = reader(scan);
  const sides = Object.fromEntries(SIDES.map(side => [side, measureSide(scan, side, pixel)]));
  const candidates = Object.values(sides).filter(side => side.uniformity >= .75 && side.support >= .6 && side.thickness >= 2 && side.thickness <= scan.width * .08);
  let group = [];
  for (const candidate of candidates) {
    const matching = candidates.filter(side => colorDistance(candidate.color, side.color) <= 28 && Math.abs(candidate.thickness - side.thickness) <= scan.width * .025);
    if (matching.length > group.length) group = matching;
  }
  if (group.length < 3) return null;
  const common = median(group.map(side => side.thickness));
  const depths = Object.fromEntries(SIDES.map(name => {
    const side = sides[name];
    return [name, group.includes(side) ? side.thickness : common];
  }));
  const edges = Object.fromEntries(SIDES.map(name => [name, group.includes(sides[name])]));
  const bounds = {
    x: depths.left, y: depths.top,
    width: scan.width - depths.left - depths.right,
    height: scan.height - depths.top - depths.bottom,
  };
  return {
    kind: Object.values(edges).every(Boolean) ? 'closed' : 'partial',
    confidence: group.reduce((sum, side) => sum + side.support, 0) / 4,
    bounds, edges,
    colors: Object.fromEntries(SIDES.map(side=>[side, group.includes(sides[side]) ? sides[side].color : colorMedian(group.map(s=>s.color))])),
    outerRadius: cornerRadius(scan, {x:0,y:0,width:scan.width,height:scan.height},
      p=>p.alpha<128 || colorDistance(p.rgb,colorMedian(group.map(s=>s.color)))>40, true),
    innerRadius: cornerRadius(scan, bounds,
      p=>p.alpha<128 || colorDistance(p.rgb,colorMedian(group.map(s=>s.color)))<12, false) ?? 0,
  };
}

export const FRAME_BEVEL_STEPS = 8;
const lightness = rgb => rgb[0] * .2126 + rgb[1] * .7152 + rgb[2] * .0722;

// Measure each side relative to its own material. Raw scan strips carry black
// exterior pixels, pinlines and rectangular corners into the new frame. A
// signed contrast profile lets one rounded contour shade the existing grain.
export function sampleInnerFrameBevel(scan, border = detectInnerFrameBorder(scan)) {
  if (!border) return null;
  const pixel = reader(scan), {bounds, edges} = border;
  const step = scan.width / 488;
  const profiles = Object.fromEntries(SIDES.map(side => {
    if (!edges[side]) return [side, Array(FRAME_BEVEL_STEPS).fill(0)];
    const samples = Array.from({length: FRAME_BEVEL_STEPS + 3}, (_, depth) => {
      const colors = Array.from({length: 64}, (_, i) => {
        const [x, y] = pointOnRay(side, .15 + i * .7 / 63, depth * step, bounds.width, bounds.height);
        return pixel(bounds.x + x, bounds.y + y).rgb;
      });
      return lightness(colorMedian(colors));
    });
    const material = median(samples.slice(FRAME_BEVEL_STEPS - 2));
    const contrast = samples.map(value => clamp((value - material) / Math.max(16, value < material ? material : 255 - material), -.94, .8));
    // Reconstruct the bevel's broad light/shadow slope, not JPEG ringing or
    // multiple hairlines. Both sides keep their own direction and strength.
    return [side, Array.from({length: FRAME_BEVEL_STEPS}, (_, d) => {
      const value = (contrast[Math.max(0, d - 1)] + 2 * contrast[d] + contrast[d + 1]) / 4;
      return value * Math.min(1, (FRAME_BEVEL_STEPS - d) / 2);
    })];
  }));
  return {...border, profiles, step};
}

// Rasterize one continuous rounded contour. Interpolating the depth profile
// avoids seams between thin CSS rings; the corner normal blends adjacent
// sides without carrying a rectangular scan corner into a rounded card.
export function rasterizeFrameBevel({width, height, radius, step, profiles, pixelRatio = 1}) {
  const w = Math.max(1, Math.round(width * pixelRatio)), h = Math.max(1, Math.round(height * pixelRatio));
  const data = new Uint8ClampedArray(w * h * 4);
  const r = clamp(radius, 0, Math.min(width, height) / 2);
  const extent = step * FRAME_BEVEL_STEPS;
  const band = Math.ceil((r + extent + 1) * pixelRatio);
  const profileAt = (side, depth) => {
    const values = profiles[side], d = Math.max(0, depth / step), i = Math.floor(d), t = d - i;
    return (values[i] || 0) * (1 - t) + (values[i + 1] || 0) * t;
  };
  for (let py = 0; py < h; py++) for (let px = 0; px < w; px++) {
    // The center is transparent. Skip it without evaluating the distance field.
    if (py > band && py < h - band && px === band && w > band * 2) px = w - band;
    const x = (px + .5) / pixelRatio, y = (py + .5) / pixelRatio;
    const qx = Math.abs(x - width / 2) - (width / 2 - r), qy = Math.abs(y - height / 2) - (height / 2 - r);
    const nx = Math.max(qx, 0), ny = Math.max(qy, 0);
    const depth = r - Math.hypot(nx, ny) - Math.min(Math.max(qx, qy), 0);
    if (depth < -.5 / pixelRatio || depth >= extent) continue;
    const horizontal = nx + ny > 0 ? nx / (nx + ny) : qx > qy ? 1 : 0;
    const contrast = profileAt(x < width / 2 ? 'left' : 'right', depth) * horizontal
      + profileAt(y < height / 2 ? 'top' : 'bottom', depth) * (1 - horizontal);
    const p = (py * w + px) * 4, color = contrast < 0 ? 0 : 255;
    data[p] = data[p + 1] = data[p + 2] = color;
    data[p + 3] = Math.round(Math.abs(contrast) * 255 * clamp(depth * pixelRatio + .5, 0, 1));
  }
  return {data, width: w, height: h};
}

// An art crop can include frame material outside its real beveled enclosure.
// Locate paired transitions on both sides and a sustained lower edge. The
// first transition owns the outside of the bevel, rather than the strongest
// (often innermost) pinline. This preserves the bevel when trimming material.
export function detectEmbeddedArtFrame(scan) {
  if (!scan || scan.width < 64 || scan.height < 64) return null;
  const pixel = reader(scan), {width, height} = scan;
  const profile = side => {
    const limit = Math.floor((side === 'bottom' ? height : width) * .06);
    return Array.from({length: limit - 1}, (_, i) => {
      const depth = i + 2;
      const changes = Array.from({length: 48}, (_, row) => {
        const t = .12 + row * .72 / 47;
        return colorDistance(pixel(...pointOnRay(side, t, depth - 1, width, height)).rgb,
          pixel(...pointOnRay(side, t, depth + 1, width, height)).rgb);
      });
      return {depth, score: median(changes), support: changes.filter(v => v > 18).length / changes.length};
    });
  };
  const rows = Object.fromEntries(['left','right','bottom'].map(side => [side, profile(side)]));
  const strong = values => {
    const peaks = values.filter(v => v.support >= .85 && v.score >= 30);
    const first = peaks.find(v => values.some(next => Math.abs(next.depth - v.depth) <= 2 && next.score >= 50 && next.support >= .9));
    if (!first) return null;
    const last = peaks.filter(v => v.depth <= first.depth + width * .025).at(-1);
    return {outer: first.depth - 1, inner: last.depth + 1, confidence: first.support};
  };
  const bottom = strong(rows.bottom);
  if (!bottom) return null;
  let left = strong(rows.left), right = strong(rows.right);
  // A dark side can have a weak but real matching edge. Require evidence at
  // BOTH transitions, anchored by the opposite side and the bottom border.
  const weak = (values, other) => {
    if (!other) return null;
    const near = depth => values.filter(v => Math.abs(v.depth - depth) <= 2).sort((a,b) => b.score - a.score)[0];
    const outer = near(other.outer + 1), inner = near(other.inner - 1);
    if (!outer || !inner || inner.depth - outer.depth < 3 || outer.score < 12 || inner.score < 12 || outer.support < .3 || inner.support < .3) return null;
    return {outer: outer.depth - 1, inner: inner.depth + 1, confidence: Math.min(outer.support, inner.support)};
  };
  if (!left) left = weak(rows.left, right);
  if (!right) right = weak(rows.right, left);
  if (!left || !right || Math.abs(left.outer - right.outer) > width * .02) return null;
  return {left, right, bottom};
}
