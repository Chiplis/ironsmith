import { castHoverTargetAtPoint } from "./hand-drag-intent.js";

// How far off the source an arrow is lifted before it looks aimed at all, and
// the sideways lanes to try when the space straight ahead is already busy.
const RISE_STEPS = [132, 204, 276];
const SIDE_STEPS = [0, -108, 108, -204, 204];
const VIEWPORT_MARGIN = 24;

function clamp(value, low, high) {
  return high <= low ? low : Math.min(high, Math.max(low, value));
}

/** The battlefield's drop grid stages a slot under the pointer, so it is not dead space. */
export function insideBattlefieldDropGrid(x, y, root = globalThis.document) {
  const grid = root?.querySelector?.('[data-battlefield-drop-grid="true"]');
  const rect = grid?.getBoundingClientRect?.();
  if (!rect) return false;
  return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
}

/** Anything a click here would pick: a card, a zone pile, a player, a battlefield slot. */
export function aimPointIsOccupied(x, y, root = globalThis.document) {
  if (castHoverTargetAtPoint(x, y, root)) return true;
  return insideBattlefieldDropGrid(x, y, root);
}

/**
 * Where to point an arrow that has just appeared and has aimed at nothing yet.
 *
 * Collapsing it onto its own source leaves no visible arrow, and snapping it to
 * wherever the mouse happens to rest reads as a choice the player never made —
 * and for a permanent it really would stage that battlefield slot. So the arrow
 * is lifted off its source, into the board and clear of anything a click would
 * pick, until the player moves the mouse and takes it over.
 */
export function deadZoneAimPoint({ from, viewport = globalThis, root = globalThis.document, occupied = aimPointIsOccupied } = {}) {
  const originX = Number(from?.x);
  const originY = Number(from?.y);
  if (!Number.isFinite(originX) || !Number.isFinite(originY)) return null;
  const width = Number(viewport?.innerWidth) || 0;
  const height = Number(viewport?.innerHeight) || 0;
  const maxX = width > VIEWPORT_MARGIN * 2 ? width - VIEWPORT_MARGIN : originX;
  const maxY = height > VIEWPORT_MARGIN * 2 ? height - VIEWPORT_MARGIN : originY;
  // Lift the arrow towards the middle of the board, away from the edge the
  // source sits against: a hand card aims up, an opponent's card aims down.
  const direction = height > 0 && originY > height / 2 ? -1 : 1;
  let first = null;
  for (const rise of RISE_STEPS) {
    for (const side of SIDE_STEPS) {
      const point = {
        x: clamp(originX + side, VIEWPORT_MARGIN, maxX),
        y: clamp(originY + (rise * direction), VIEWPORT_MARGIN, maxY),
      };
      first = first || point;
      if (!occupied(point.x, point.y, root)) return point;
    }
  }
  // A board with no dead space left still deserves a visible arrow.
  return first;
}
