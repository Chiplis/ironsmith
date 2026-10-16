// Redistribute the fan inside its existing horizontal envelope. Resting cards
// overlap more when the reading card needs room; the outer edges never move.
export function compressedHandSelection(layout, dims, activeIndex) {
  const reach = (card, scale) => {
    const angle = Math.abs(card.rot) * Math.PI / 180;
    return scale * (dims.cardW * Math.cos(angle) + dims.cardH * Math.sin(angle)) / 2;
  };
  if (!layout[activeIndex]) return null;
  // Small hands can use their reserved desktop lane without shrinking the
  // preview; crowded hands already occupy that lane.
  const left = Math.min(-dims.packWidth / 2, ...layout.map(card => card.x - reach(card, 1)));
  const right = Math.max(dims.packWidth / 2, ...layout.map(card => card.x + reach(card, 1)));
  const scale = Math.min(dims.hoverScale, (right - left) / (2 * reach(layout[activeIndex], 1)));
  const activeReach = reach(layout[activeIndex], scale);
  const activeX = Math.max(left + activeReach, Math.min(right - activeReach, layout[activeIndex].x));
  const gap = Math.max(2, dims.cardW * 0.015);
  return layout.map((card, index) => {
    if (index === activeIndex) return { x: activeX, scale };
    const half = reach(card, 0.94);
    let x;
    if (index < activeIndex) {
      const start = Math.max(left + half, Math.min(layout[0].x, activeX - half));
      const end = Math.max(start, Math.min(layout[activeIndex - 1].x, activeX - activeReach - half - gap));
      x = start + (end - start) * (activeIndex === 1 ? 0 : index / (activeIndex - 1));
    } else {
      const end = Math.min(right - half, Math.max(layout.at(-1).x, activeX + half));
      const start = Math.min(end, Math.max(layout[activeIndex + 1].x, activeX + activeReach + half + gap));
      const count = layout.length - activeIndex - 1;
      x = start + (end - start) * (count === 1 ? 1 : (index - activeIndex - 1) / (count - 1));
    }
    return { x: Math.max(left + half, Math.min(right - half, x)), scale: 0.94 };
  });
}

// Resting cards rotate around bottom-center. Reserve their upper corners
// independently of hover scale, straightening, and transition progress.
export function reservedHandHalfWidth(layout, dims) {
  return Math.max(dims.packWidth / 2, ...layout.map(card => {
    const angle = Math.abs(card.rot) * Math.PI / 180;
    return Math.abs(card.x) + dims.cardW * Math.cos(angle) / 2 + dims.cardH * Math.sin(angle);
  }));
}
