// Keep substantial props in free perimeter pockets. These are screen-space
// bounds from the same DOM measurement as the cards, never a second card layout.
export function placeForgeScenery(width, height, obstacles) {
  const maximum = Math.min(165, height * 0.22, width * 0.115);
  const used = [...obstacles];
  return [
    { kind: 'rock', side: 0, y: 0.24 }, { kind: 'rock', side: 1, y: 0.25 },
    { kind: 'pit', side: 0, y: 0.62 }, { kind: 'pit', side: 1, y: 0.64 },
    { kind: 'rock', side: 0, y: 0.90 }, { kind: 'rock', side: 1, y: 0.91 },
  ].map(({ kind, side, y: preferred }) => {
    let best;
    for (let step = 0; step <= 12; step++) {
      const y = height * (0.12 + step * 0.065);
      const x = side ? width - maximum * 0.20 : maximum * 0.20;
      let radius = maximum * 0.5;
      for (const r of used) {
        const dx = Math.max(r.left - x, 0, x - r.right);
        const dy = Math.max(r.top - y, 0, y - r.bottom);
        radius = Math.min(radius, Math.hypot(dx, dy) - 12);
      }
      radius = Math.max(0, radius);
      const score = radius - Math.abs(y / height - preferred) * maximum * 0.35;
      if (!best || score > best.score) best = { x, y, radius, score };
    }
    const size = best.radius * 2;
    used.push({ left: best.x - size / 2, right: best.x + size / 2, top: best.y - size / 2, bottom: best.y + size / 2 });
    return { kind, x: best.x, y: best.y, size };
  });
}
