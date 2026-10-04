const DOCK_EDGE_PADDING = 14;
export const DECISION_DOCK_BOTTOM_INSET = 16;
const DOCK_RIGHT_INSET = 18;
const DOCK_MIN_TOP = 48;
const ZONE_CLEARANCE = 12;
const HAND_DOCK_GAP = 16;
const FOCUSED_HUD_MIN_VIEWPORT = 1024;

/**
 * Horizontal room the desktop hand leaves free on each side so the decision
 * dock, anchored bottom-right, never runs into it. Shared by the hand (which
 * caps its width with it) and the dock (which fills what is left).
 */
export function handSideReserve(viewportWidth) {
  if (!(viewportWidth >= FOCUSED_HUD_MIN_VIEWPORT)) return 24;
  return DOCK_RIGHT_INSET + clamp(viewportWidth * 0.25, 260, 340) + HAND_DOCK_GAP;
}

/** Widest the decision dock may get without reaching the hand's reserve. */
export function dockMaxWidth(viewportWidth) {
  return Math.max(220, handSideReserve(viewportWidth) - DOCK_RIGHT_INSET - HAND_DOCK_GAP);
}

/** Payment lives in the lane between the reserved hand edge and zone piles.
 * Its top boundary is the bottom of the opponents' first card row; scrolling
 * consumes excess content, never that protected battlefield space.
 */
export function anchorManaPaymentDock({ viewportWidth, viewportHeight, protectedZones = [], opponentCardBottom = 48 }) {
  const bottom = viewportHeight - DECISION_DOCK_BOTTOM_INSET;
  const topLimit = Math.max(DOCK_MIN_TOP, opponentCardBottom + ZONE_CLEARANCE);
  const handRight = viewportWidth - handSideReserve(viewportWidth);
  const zoneLeft = protectedZones.reduce((left, zone) => Math.min(left, zone.left), viewportWidth - DOCK_RIGHT_INSET);
  const right = zoneLeft - ZONE_CLEARANCE;
  const maxWidth = Math.max(1, Math.min(320, right - handRight - HAND_DOCK_GAP));
  const maxHeight = Math.max(1, Math.floor(bottom - topLimit));
  return { left: Math.round(right - maxWidth), top: bottom - maxHeight, maxWidth: Math.floor(maxWidth), maxHeight: Math.floor(maxHeight) };
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

/**
 * Anchor the floating action dock to the bottom-right corner, just above the
 * hand, so it is always in the same predictable place. It never wanders
 * around the board: the only thing it steps aside for is a protected zone
 * (Graveyard/Exile), and then it slides left of that column while staying
 * anchored to the bottom.
 */
export function anchorFloatingDock({
  viewportWidth,
  viewportHeight,
  dockWidth,
  dockHeight,
  bottomLimit = viewportHeight - DOCK_EDGE_PADDING,
  protectedZones = [],
}) {
  if (![viewportWidth, viewportHeight, dockWidth, dockHeight].every(Number.isFinite)) return null;
  const maxTop = Math.max(DOCK_MIN_TOP, viewportHeight - dockHeight - DOCK_EDGE_PADDING);
  const top = clamp(bottomLimit - dockHeight, DOCK_MIN_TOP, maxTop);
  let left = viewportWidth - dockWidth - DOCK_RIGHT_INSET;

  for (const zone of protectedZones) {
    if (!zone || ![zone.left, zone.top, zone.right, zone.bottom].every(Number.isFinite)) continue;
    const overlapsVertically = top < zone.bottom + ZONE_CLEARANCE
      && top + dockHeight > zone.top - ZONE_CLEARANCE;
    const overlapsHorizontally = left < zone.right + ZONE_CLEARANCE
      && left + dockWidth > zone.left - ZONE_CLEARANCE;
    if (overlapsVertically && overlapsHorizontally) {
      left = Math.min(left, zone.left - ZONE_CLEARANCE - dockWidth);
    }
  }

  const maxLeft = Math.max(DOCK_EDGE_PADDING, viewportWidth - dockWidth - DOCK_EDGE_PADDING);
  return { left: Math.round(clamp(left, DOCK_EDGE_PADDING, maxLeft)), top };
}
