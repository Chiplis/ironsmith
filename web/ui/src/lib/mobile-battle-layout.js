const CARD_ASPECT_RATIO = 124 / 96;
const MOBILE_MIN_CARD_HEIGHT = 24;
const MOBILE_MAX_CARD_HEIGHT = 82;
const MOBILE_BATTLEFIELD_SIDE_PADDING_PX = 8;
const MOBILE_ROW_GAP_PX = 6;
const MOBILE_SECTION_GAP_PX = 6;
const MOBILE_BOTTOM_PEEK_HEIGHT_PX = 46;
const MOBILE_BOTTOM_BAR_HEIGHT_PX = 0;
const MOBILE_TOP_BUFFER_PX = 2;
const MOBILE_CONTROL_BAND_MIN_HEIGHT_PX = 28;
const MOBILE_CONTROL_BAND_MAX_HEIGHT_PX = 72;
const MOBILE_TOP_STATUS_FALLBACK_PX = 30;
const MOBILE_BACK_ROW_VISIBLE_RATIO = 0.78;

// MTGA-aligned region defaults (used when callers don't pass a measured value).
export const MOBILE_OPPONENT_HUD_HEIGHT_PX = 38;
export const MOBILE_SELF_HUD_HEIGHT_PX = 38;
export const MOBILE_MANA_POOL_HEIGHT_PX = 18;
export const MOBILE_PHASE_STRIP_HEIGHT_PX = 30;
export const MOBILE_TURN_ACTION_STACK_WIDTH_PX = 96;
export const MOBILE_HAND_PEEK_HEIGHT_PX = 24;
export const MOBILE_HAND_FANNED_HEIGHT_PX = 92;
export const MOBILE_STACK_RAIL_WIDTH_PX = 56;

// Compact-mode (height <= 320) shrinks to keep `fitsViewport` true on smaller landscape phones
// (iPhone SE landscape is 320px; some Android phones in landscape with browser chrome
// dip to 280px). At those sizes the opponent mana pool — read-only and least essential —
// drops entirely.
const COMPACT_SELF_HUD_HEIGHT_PX = 28;
const COMPACT_OPPONENT_MANA_POOL_HEIGHT_PX = 0;
const COMPACT_SELF_MANA_POOL_HEIGHT_PX = 12;
const COMPACT_HAND_PEEK_HEIGHT_PX = 16;
const COMPACT_STACK_RAIL_WIDTH_PX = 44;

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

export function solveMobileBattleLayout({
  viewportWidth = 0,
  viewportHeight = 0,
  safeAreaTop = 0,
  safeAreaBottom = 0,
  topBandHeight = 0,
  controlBandHeight = 0,
  collapsedHandRailHeight = MOBILE_BOTTOM_PEEK_HEIGHT_PX,
  opponentFrontCount = 0,
  opponentBackCount = 0,
  selfFrontCount = 0,
  selfBackCount = 0,
  // MTGA-aligned region inputs. When zero, the corresponding region is treated as not rendered
  // and contributes no fixed height to the layout solver. When > 0, the value is subtracted
  // from `availableBattlefieldHeight` so the battlefield bands shrink to make room.
  opponentManaPoolHeight = 0,
  selfManaPoolHeight = 0,
  selfHudHeight = 0,
  handPeekHeight = 0,
  // Horizontal: when stack is visible, reserve a vertical rail on the right edge.
  stackVisible = false,
  stackRailWidth = MOBILE_STACK_RAIL_WIDTH_PX,
}) {
  const width = Math.max(1, Math.floor(viewportWidth || 0));
  const height = Math.max(1, Math.floor(viewportHeight || 0));
  const compactMode = height <= 320;
  const sidePadding = width <= 360 ? 6 : MOBILE_BATTLEFIELD_SIDE_PADDING_PX;
  const rowGap = MOBILE_ROW_GAP_PX;
  const sectionGap = MOBILE_SECTION_GAP_PX;
  const topStatusHeight = Math.max(
    MOBILE_TOP_STATUS_FALLBACK_PX,
    Math.ceil((topBandHeight || 0) + MOBILE_TOP_BUFFER_PX)
  );
  const normalizedControlBandHeight = controlBandHeight > 0
    ? clamp(
      Math.ceil(controlBandHeight || 0),
      MOBILE_CONTROL_BAND_MIN_HEIGHT_PX,
      MOBILE_CONTROL_BAND_MAX_HEIGHT_PX
    )
    : 0;
  const bottomPeekHeight = Math.max(
    0,
    Math.ceil(collapsedHandRailHeight || 0)
  );
  const bottomBandHeight = bottomPeekHeight > 0
    ? Math.max(
      MOBILE_BOTTOM_BAR_HEIGHT_PX,
      bottomPeekHeight
    )
    : 0;

  // New MTGA-aligned region heights — used additively. Compact-mode actively shrinks each
  // region (and drops the opponent mana pool entirely on the smallest phones) so the
  // battlefield bands keep enough vertical space to render at the 24px min card height.
  const shrinkInCompact = (requested, compactCap) => {
    const rounded = Math.max(0, Math.ceil(requested || 0));
    if (rounded <= 0) return 0;
    if (compactMode) return Math.min(rounded, compactCap);
    return rounded;
  };
  const opponentManaPoolPx = opponentManaPoolHeight > 0
    ? shrinkInCompact(opponentManaPoolHeight, COMPACT_OPPONENT_MANA_POOL_HEIGHT_PX)
    : 0;
  const selfManaPoolPx = selfManaPoolHeight > 0
    ? shrinkInCompact(selfManaPoolHeight, COMPACT_SELF_MANA_POOL_HEIGHT_PX)
    : 0;
  const selfHudPx = selfHudHeight > 0
    ? shrinkInCompact(selfHudHeight, COMPACT_SELF_HUD_HEIGHT_PX)
    : 0;
  const handPeekPx = handPeekHeight > 0
    ? shrinkInCompact(handPeekHeight, COMPACT_HAND_PEEK_HEIGHT_PX)
    : 0;
  const stackRailWidthPx = stackVisible
    ? (compactMode
      ? Math.min(Math.max(0, Math.ceil(stackRailWidth)), COMPACT_STACK_RAIL_WIDTH_PX)
      : Math.max(0, Math.ceil(stackRailWidth)))
    : 0;

  // Number of additional 6px section gaps introduced by the new regions. We charge one gap
  // per region that's actually present so the cards aren't pushed flush against each other.
  const extraSectionGapCount = (opponentManaPoolPx > 0 ? 1 : 0)
    + (selfManaPoolPx > 0 ? 1 : 0)
    + (selfHudPx > 0 ? 1 : 0)
    + (handPeekPx > 0 ? 1 : 0);
  const newRegionFixedHeight = opponentManaPoolPx + selfManaPoolPx + selfHudPx + handPeekPx
    + (sectionGap * extraSectionGapCount);
  const maxColumns = Math.max(
    1,
    opponentFrontCount,
    opponentBackCount,
    selfFrontCount,
    selfBackCount
  );
  const usableWidth = Math.max(1, width - (sidePadding * 2) - stackRailWidthPx);
  const widthLimitedCard = Math.floor(
    (usableWidth - (Math.max(0, maxColumns - 1) * rowGap)) / maxColumns
  );
  const availableBattlefieldHeight = Math.max(
    MOBILE_MIN_CARD_HEIGHT,
    height
      - safeAreaTop
      - safeAreaBottom
      - topStatusHeight
      - normalizedControlBandHeight
      - bottomBandHeight
      - newRegionFixedHeight
      - (sectionGap * 3)
  );
  const battlefieldFixedRowGaps = rowGap * 2;
  const heightLimitedCard = Math.floor(
    (availableBattlefieldHeight - battlefieldFixedRowGaps)
      / (3 + MOBILE_BACK_ROW_VISIBLE_RATIO)
  );
  const cardHeight = clamp(
    Math.min(
      MOBILE_MAX_CARD_HEIGHT,
      heightLimitedCard,
      Math.floor(widthLimitedCard / CARD_ASPECT_RATIO)
    ),
    MOBILE_MIN_CARD_HEIGHT,
    MOBILE_MAX_CARD_HEIGHT
  );
  const cardWidth = Math.max(
    1,
    Math.floor(cardHeight * CARD_ASPECT_RATIO)
  );
  const selfBackVisibleHeight = Math.max(
    1,
    Math.ceil(cardHeight * MOBILE_BACK_ROW_VISIBLE_RATIO)
  );
  const opponentBandHeight = (cardHeight * 2) + rowGap;
  const selfBandHeight = cardHeight + rowGap + selfBackVisibleHeight;
  const totalHeight =
    topStatusHeight
    + sectionGap
    + opponentBandHeight
    + sectionGap
    + normalizedControlBandHeight
    + sectionGap
    + selfBandHeight
    + bottomBandHeight
    + newRegionFixedHeight
    + safeAreaTop
    + safeAreaBottom;

  return {
    viewportWidth: width,
    viewportHeight: height,
    safeAreaTop,
    safeAreaBottom,
    sidePadding,
    rowGap,
    sectionGap,
    topStatusHeight,
    controlBandHeight: normalizedControlBandHeight,
    bottomPeekHeight,
    bottomBandHeight,
    cardWidth,
    cardHeight,
    opponentBandHeight,
    selfFrontHeight: cardHeight,
    selfBackVisibleHeight,
    selfBackVisibleRatio: MOBILE_BACK_ROW_VISIBLE_RATIO,
    compactMode,
    totalHeight,
    fitsViewport: totalHeight <= height,
    // New MTGA-aligned outputs. Consumers use these to set fixed heights / widths on regions.
    opponentManaPoolHeight: opponentManaPoolPx,
    selfManaPoolHeight: selfManaPoolPx,
    selfHudHeight: selfHudPx,
    handPeekHeight: handPeekPx,
    stackRailWidth: stackRailWidthPx,
  };
}
