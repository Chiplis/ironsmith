const CARD_ASPECT_RATIO = 124 / 96;
const MIN_CARD_HEIGHT = 28;
const MAX_CARD_HEIGHT = 82;

export const MOBILE_OPPONENT_HUD_HEIGHT_PX = 36;
export const MOBILE_SELF_HUD_HEIGHT_PX = 36;
export const MOBILE_PHASE_STRIP_HEIGHT_PX = 36;
export const MOBILE_HAND_PEEK_HEIGHT_PX = 40;
export const MOBILE_STACK_RAIL_WIDTH_PX = 60;

/**
 * Lay out the six rendered regions inside the safe-area-adjusted scene.
 * Measured controls never shrink: Safari, translations and accessibility font
 * sizes must be charged at their real height. Crowded lanes scroll horizontally
 * instead of reducing every card to an untappable dot.
 */
export function solveMobileBattleLayout({
  viewportWidth = 852,
  viewportHeight = 320,
  topBandHeight = MOBILE_OPPONENT_HUD_HEIGHT_PX,
  controlBandHeight = MOBILE_PHASE_STRIP_HEIGHT_PX,
  selfHudHeight = MOBILE_SELF_HUD_HEIGHT_PX,
  handPeekHeight = MOBILE_HAND_PEEK_HEIGHT_PX,
  stackVisible = false,
  stackRailWidth = MOBILE_STACK_RAIL_WIDTH_PX,
} = {}) {
  const width = Math.max(1, Math.floor(viewportWidth));
  const height = Math.max(1, Math.floor(viewportHeight));
  const sectionGap = 4;
  const rowGap = 4;
  const sidePadding = 8;
  const topStatusHeight = Math.ceil(topBandHeight);
  const controlHeight = Math.ceil(controlBandHeight);
  const selfHud = Math.ceil(selfHudHeight);
  const handPeek = Math.ceil(handPeekHeight);
  const stackWidth = stackVisible ? Math.ceil(stackRailWidth) : 0;
  const fixedHeight = topStatusHeight + controlHeight + selfHud + handPeek + sectionGap * 5;
  const availableBattlefieldHeight = Math.max(0, height - fixedHeight);
  const cardHeight = Math.max(MIN_CARD_HEIGHT, Math.min(MAX_CARD_HEIGHT,
    Math.floor((availableBattlefieldHeight - rowGap * 2) / 4)));
  const cardWidth = Math.floor(cardHeight * CARD_ASPECT_RATIO);
  const bandHeight = cardHeight * 2 + rowGap;
  const totalHeight = fixedHeight + bandHeight * 2;

  return {
    viewportWidth: width,
    viewportHeight: height,
    sidePadding,
    rowGap,
    sectionGap,
    topStatusHeight,
    controlBandHeight: controlHeight,
    selfHudHeight: selfHud,
    handPeekHeight: handPeek,
    cardWidth,
    cardHeight,
    opponentBandHeight: bandHeight,
    selfBandHeight: bandHeight,
    selfBackVisibleHeight: cardHeight,
    stackRailWidth: stackWidth,
    battlefieldWidth: Math.max(1, width - sidePadding * 2 - (stackWidth ? stackWidth + sectionGap : 0)),
    totalHeight,
    fitsViewport: totalHeight <= height,
  };
}
