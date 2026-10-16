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
export function anchorManaPaymentDock({ viewportWidth, viewportHeight, protectedZones = [], opponentCardBottom = 48, handRight = viewportWidth - handSideReserve(viewportWidth), handTop = viewportHeight, commandZoneMidpoint = null }) {
  const opponentTopLimit = Math.max(DOCK_MIN_TOP, opponentCardBottom + ZONE_CLEARANCE);
  const zoneLeft = protectedZones.reduce((left, zone) => Math.min(left, zone.left), viewportWidth - DOCK_RIGHT_INSET);
  const right = zoneLeft - ZONE_CLEARANCE;
  const laneWidth = right - handRight - HAND_DOCK_GAP;
  const aboveHand = laneWidth < 180;
  const bottom = Math.min(viewportHeight - DECISION_DOCK_BOTTOM_INSET, aboveHand ? Math.max(handTop - HAND_DOCK_GAP, commandZoneMidpoint ?? 0) : viewportHeight);
  const topLimit = aboveHand ? Math.max(DOCK_MIN_TOP, Math.min(opponentTopLimit, bottom - 200)) : opponentTopLimit;
  const maxWidth = Math.max(1, Math.min(320, aboveHand ? right - DOCK_EDGE_PADDING : laneWidth));
  const maxHeight = Math.max(1, Math.floor(bottom - topLimit));
  return { left: Math.round(right - maxWidth), top: bottom - maxHeight, maxWidth: Math.floor(maxWidth), maxHeight: Math.floor(maxHeight) };
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

/**
 * Anchor the floating action dock to the bottom-right corner, just above the
 * hand, so it is always in the same predictable place. It never wanders
 * around the board: it steps left of protected zones (Graveyard/Exile), then
 * moves upward if that placement would intersect the hand.
 */
export function anchorFloatingDock({
  viewportWidth,
  viewportHeight,
  dockWidth,
  dockHeight,
  bottomLimit = viewportHeight - DOCK_EDGE_PADDING,
  protectedZones = [],
  handBounds = null,
}) {
  if (![viewportWidth, viewportHeight, dockWidth, dockHeight].every(Number.isFinite)) return null;
  const maxTop = Math.max(DOCK_MIN_TOP, viewportHeight - dockHeight - DOCK_EDGE_PADDING);
  let top = clamp(bottomLimit - dockHeight, DOCK_MIN_TOP, maxTop);
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
  left = Math.round(clamp(left, DOCK_EDGE_PADDING, maxLeft));
  // A pile can push the dock into the hand's reserved lane. Move upward
  // there instead of letting the hand cover the decision controls. Once
  // lifted, a pile in the way still sends the dock left of it: climbing over
  // a pile column (3-4 players stack the local piles above the hand's corner)
  // would fling the dock to the top of the board.
  const collides = (zone) => left < zone.right + ZONE_CLEARANCE
    && left + dockWidth > zone.left - ZONE_CLEARANCE
    && top < zone.bottom + ZONE_CLEARANCE
    && top + dockHeight > zone.top - ZONE_CLEARANCE;
  const zones = protectedZones.filter(zone => zone && [zone.left, zone.top, zone.right, zone.bottom].every(Number.isFinite));
  for (let pass = 0; handBounds && pass <= zones.length; pass += 1) {
    let moved = false;
    if (collides(handBounds)) {
      top = Math.max(DOCK_MIN_TOP, handBounds.top - ZONE_CLEARANCE - dockHeight);
      moved = true;
    }
    for (const zone of zones) {
      if (collides(zone)) {
        left = Math.round(Math.max(DOCK_EDGE_PADDING, zone.left - ZONE_CLEARANCE - dockWidth));
        moved = true;
      }
    }
    if (!moved) break;
  }
  return { left, top };
}

/** Keep the decision in the horizontal lane beside the actual hand cards.
 * Reserve the entire zone column, even before the panel grows into it, so
 * expanding content cannot alternate between two different placements.
 */
export function anchorDecisionDock({ viewportWidth, viewportHeight, dockWidth, dockHeight, protectedZones = [], handRight = 0, handTop = viewportHeight, commandZoneMidpoint = null }) {
  const zones = protectedZones.filter(zone => Number.isFinite(zone?.left));
  const right = zones.reduce((edge, zone) => Math.min(edge, zone.left - ZONE_CLEARANCE), viewportWidth - DOCK_RIGHT_INSET);
  const leftLimit = Math.max(DOCK_EDGE_PADDING, handRight + HAND_DOCK_GAP);
  const laneWidth = right - leftLimit;
  // If a wide hand leaves too little room, use the space above its cards.
  const aboveHand = laneWidth < 180;
  const maxWidth = Math.max(1, Math.floor(aboveHand ? Math.min(340, right - DOCK_EDGE_PADDING) : laneWidth));
  const bottom = Math.min(viewportHeight - DECISION_DOCK_BOTTOM_INSET, aboveHand ? Math.max(handTop - HAND_DOCK_GAP, commandZoneMidpoint ?? 0) : viewportHeight);
  const zoneTop = zones.reduce((top, zone) => Number.isFinite(zone.top) ? Math.min(top, zone.top) : top, Infinity);
  const deckTop = Math.max(DOCK_MIN_TOP, Number.isFinite(zoneTop) ? zoneTop : DOCK_MIN_TOP);
  // The command midpoint caps hover-driven lifting of the bottom controls;
  // expanded decision content can still grow upward toward the deck.
  const topLimit = aboveHand ? Math.max(DOCK_MIN_TOP, Math.min(deckTop, bottom - 200)) : deckTop;
  const maxHeight = Math.max(1, bottom - topLimit);
  return {
    left: Math.round(right - Math.min(dockWidth, maxWidth)),
    top: Math.round(bottom - Math.min(dockHeight, maxHeight)),
    maxWidth,
    maxHeight,
  };
}
