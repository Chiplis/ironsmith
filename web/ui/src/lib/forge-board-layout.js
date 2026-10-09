// Screen-space scenery only. Card layout and hit targets remain authoritative.
export const FORGE_SETTLE_MS = 1600;
export const unionRect = (a, b) => ({
  left: Math.min(a.left, b.left), top: Math.min(a.top, b.top),
  right: Math.max(a.right, b.right), bottom: Math.max(a.bottom, b.bottom),
});
export function clipRect(rect, width, height, padding = 0) {
  const result = {
    left: Math.max(0, rect.left - padding), top: Math.max(0, rect.top - padding),
    right: Math.min(width, rect.right + padding), bottom: Math.min(height, rect.bottom + padding),
  };
  return result.right > result.left && result.bottom > result.top ? result : null;
}
const sameRect = (a, b) => a && b && ['left', 'top', 'right', 'bottom'].every(k => Math.abs(a[k] - b[k]) < 2);

// Preserve outgoing space until flights finish; rapid blink/reanimation never
// collapses the platform between snapshots. Resize/reconnect can reset directly.
export function settleForgeZones(previous, zones, now, { reset = false, locked = false } = {}) {
  const next = new Map();
  for (const zone of zones) {
    const old = reset ? null : previous.get(zone.key);
    const changed = !sameRect(old?.requested, zone.rect);
    const releaseAt = changed ? now + FORGE_SETTLE_MS : old.releaseAt;
    const rect = old && (locked || now < releaseAt) ? unionRect(old.rect, zone.rect) : zone.rect;
    next.set(zone.key, { ...zone, rect, requested: zone.rect, releaseAt });
  }
  if (!reset) for (const [key, old] of previous) {
    if (next.has(key)) continue;
    const releaseAt = old.removing ? old.releaseAt : now + FORGE_SETTLE_MS;
    if (locked || now < releaseAt) next.set(key, { ...old, removing: true, releaseAt });
  }
  return next;
}

// Largest corner ornament that does not intersect any occupied UI rectangle.
export function forgeCornerSizes(width, height, obstacles) {
  return [[0, 0], [width, 0], [0, height], [width, height]].map(([x, y]) => {
    let size = Math.min(94, width * 0.12, height * 0.18);
    for (const r of obstacles) {
      const dx = x === 0 ? r.left : width - r.right;
      const dy = y === 0 ? r.top : height - r.bottom;
      size = Math.min(size, Math.max(dx, dy) - 12);
    }
    return Math.max(0, size);
  });
}

export function forgeSignal(state) {
  return {
    turn: `${state?.turn_number ?? ''}:${state?.active_player ?? ''}`,
    combat: /combat|attack|block/i.test(`${state?.phase || ''} ${state?.step || ''}`),
  };
}
