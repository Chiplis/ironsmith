// Find card-shaped space above/below a zone, excluding live decision surfaces.
export function zonePreviewLayout({ anchor, viewportWidth, viewportHeight, minimumTop = 8, height = 531, obstacles = [] }) {
  const gap = 8;
  let spaces = [
    { left: 8, right: viewportWidth - 8, top: minimumTop, bottom: anchor.top - gap, above: true },
    { left: 8, right: viewportWidth - 8, top: anchor.bottom + gap, bottom: viewportHeight - 8, above: false },
  ];
  for (const obstacle of obstacles) {
    const o = {left: obstacle.left - gap, right: obstacle.right + gap, top: obstacle.top - gap, bottom: obstacle.bottom + gap};
    spaces = spaces.flatMap(r => {
      if (o.right <= r.left || o.left >= r.right || o.bottom <= r.top || o.top >= r.bottom) return [r];
      return [
        {...r, right: Math.min(r.right, o.left)},
        {...r, left: Math.max(r.left, o.right)},
        {...r, bottom: Math.min(r.bottom, o.top)},
        {...r, top: Math.max(r.top, o.bottom)},
      ].filter(r => r.right > r.left && r.bottom > r.top);
    });
  }
  const candidates = spaces.map(r => {
    const h = Math.floor(Math.max(0, Math.min(height, r.bottom - r.top, (r.right - r.left) * 88 / 63)));
    const w = h * 63 / 88;
    const preferredLeft = anchor.right + gap + w <= r.right ? anchor.right + gap : anchor.left - gap - w;
    return {left: Math.max(r.left, Math.min(r.right - w, preferredLeft)), top: r.above ? r.bottom - h : r.top, height: h, width: w, above: r.above};
  }).filter(r => r.height > 0);
  candidates.sort((a, b) => b.height - a.height || Number(b.above) - Number(a.above)
    || Math.abs(a.left - anchor.left) - Math.abs(b.left - anchor.left));
  const best = candidates[0];
  return best ? {left: best.left, top: best.top, right: 'auto', height: `${best.height}px`, width: `${best.width}px`, maxHeight: `${best.height}px`}
    : {visibility: 'hidden'};
}
