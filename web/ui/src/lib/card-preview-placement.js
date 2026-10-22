// Keep the entire card in the permitted region, preferring its original size
// and then the nearest free position. Only shrink when no full-size space fits.
// `frames` stacks that many card frames vertically (a face-down permanent and
// its known front face); `height` stays the height of one frame.
export function placeCardPreview({ preferred, height, bounds, obstacles = [], gap = 8, frames = 1, frameGap = 8 }) {
  let spaces = [bounds];
  for (const obstacle of obstacles) {
    const o = { left: obstacle.left - gap, right: obstacle.right + gap,
      top: obstacle.top - gap, bottom: obstacle.bottom + gap };
    spaces = spaces.flatMap(r => {
      if (o.right <= r.left || o.left >= r.right || o.bottom <= r.top || o.top >= r.bottom) return [r];
      // Keep maximal rectangles: a full-height strip can fit a card even
      // when the top and bottom regions also overlap that strip.
      return [
        { ...r, right: Math.min(r.right, o.left) },
        { ...r, left: Math.max(r.left, o.right) },
        { ...r, bottom: Math.min(r.bottom, o.top) },
        { ...r, top: Math.max(r.top, o.bottom) },
      ].filter(r => r.right > r.left && r.bottom > r.top);
    });
  }
  const candidates = spaces.map(r => {
    const frameHeight = Math.floor(Math.max(0, Math.min(height,
      (r.bottom - r.top - frameGap * (frames - 1)) / frames, (r.right - r.left) * 88 / 63)));
    const h = frameHeight * frames + (frameHeight > 0 ? frameGap * (frames - 1) : 0);
    const width = frameHeight * 63 / 88;
    const left = Math.max(r.left, Math.min(r.right - width, preferred.left));
    const top = Math.max(r.top, Math.min(r.bottom - h, preferred.top));
    return { left, top, height: h, width, distance: Math.hypot(left - preferred.left, top - preferred.top) };
  }).filter(r => r.height > 0);
  candidates.sort((a, b) => b.height - a.height || a.distance - b.distance);
  const best = candidates[0];
  return best ? { left: best.left, top: best.top, right: 'auto', height: `${best.height}px`, width: `${best.width}px`, maxHeight: `${best.height}px` }
    : { visibility: 'hidden' };
}
