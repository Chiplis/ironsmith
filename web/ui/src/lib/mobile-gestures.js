export const MOBILE_TAP_DISTANCE_PX = 16;
export const MOBILE_TAP_DISTANCE_SQ = MOBILE_TAP_DISTANCE_PX * MOBILE_TAP_DISTANCE_PX;
export const MOBILE_LONG_PRESS_MS = 380;
export const MOBILE_OPPONENT_CARD_HIT_SLOP_X = 14;
export const MOBILE_OPPONENT_CARD_HIT_SLOP_Y = 18;

export function pointerDistanceSq(a, b) {
  const dx = (a?.clientX ?? 0) - (b?.clientX ?? 0);
  const dy = (a?.clientY ?? 0) - (b?.clientY ?? 0);
  return (dx * dx) + (dy * dy);
}

export function withinExpandedRect(rect, x, y, slopX = MOBILE_OPPONENT_CARD_HIT_SLOP_X, slopY = MOBILE_OPPONENT_CARD_HIT_SLOP_Y) {
  return (
    x >= rect.left - slopX
    && x <= rect.right + slopX
    && y >= rect.top - slopY
    && y <= rect.bottom + slopY
  );
}

export function findCardElementAtPoint(event, cardSelector, cardLookup) {
  if (typeof document === "undefined") return null;
  if (!Number.isFinite(event?.clientX) || !Number.isFinite(event?.clientY)) return null;

  const fromComposedPath = () => {
    const path = typeof event.composedPath === "function" ? event.composedPath() : (event.path || null);
    if (!Array.isArray(path)) return null;
    for (const node of path) {
      if (!(node instanceof Element)) continue;
      const cardEl = node.closest?.(cardSelector) || (node.matches?.(cardSelector) ? node : null);
      if (cardEl) return cardEl;
    }
    return null;
  };

  const pathEl = fromComposedPath();
  if (pathEl) return pathEl;

  const targetEl = event.target instanceof Element
    ? event.target.closest(cardSelector)
    : null;
  if (targetEl) return targetEl;

  const sampleOffsets = [
    [0, 0], [-12, 0], [12, 0], [0, -12], [0, 12],
    [-10, -10], [10, -10], [-10, 10], [10, 10],
  ];
  for (const [ox, oy] of sampleOffsets) {
    const hit = document.elementFromPoint(event.clientX + ox, event.clientY + oy);
    const cardEl = hit?.closest?.(cardSelector);
    if (cardEl) return cardEl;
  }

  if (cardLookup && typeof cardLookup === "function") {
    return cardLookup({ x: event.clientX, y: event.clientY });
  }

  return null;
}
