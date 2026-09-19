export const MOBILE_BATTLE_CARD_ASPECT_RATIO = 1.24;
const MIN_CARD_HEIGHT = 24;
const MAX_CARD_HEIGHT = 124;

export const MOBILE_OPPONENT_HUD_HEIGHT_PX = 36;
export const MOBILE_CONTROL_BAND_HEIGHT_PX = 36;
export const MOBILE_HAND_PEEK_HEIGHT_PX = 52;
export const MOBILE_STACK_RAIL_WIDTH_PX = 60;

/**
 * Lay out two battlefield bands and the hand inside the safe-area-adjusted scene.
 * Player and decision controls float in reserved side areas within the bands.
 * Measured controls never shrink: Safari, translations and accessibility font
 * sizes must be charged at their real height. Crowded lanes scroll horizontally
 * instead of reducing every card to an untappable dot.
 */
export function solveMobileBattleLayout({
  viewportWidth = 852,
  viewportHeight = 320,
  topBandHeight = MOBILE_OPPONENT_HUD_HEIGHT_PX,
  controlBandHeight = MOBILE_CONTROL_BAND_HEIGHT_PX,
  handPeekHeight = MOBILE_HAND_PEEK_HEIGHT_PX,
  stackVisible = false,
  stackRailWidth = MOBILE_STACK_RAIL_WIDTH_PX,
} = {}) {
  const width = Math.max(1, Math.floor(viewportWidth));
  const height = Math.max(1, Math.floor(viewportHeight));
  const sectionGap = 0;
  const rowGap = 0;
  const sidePadding = 0;
  const topStatusHeight = Math.ceil(topBandHeight);
  const controlHeight = Math.ceil(controlBandHeight);
  const handPeek = Math.ceil(handPeekHeight);
  const stackWidth = stackVisible ? Math.ceil(stackRailWidth) : 0;
  const fixedHeight = handPeek;
  const availableBattlefieldHeight = Math.max(0, height - fixedHeight);
  const cardHeight = Math.max(MIN_CARD_HEIGHT, Math.min(MAX_CARD_HEIGHT,
    Math.floor((availableBattlefieldHeight - rowGap * 2) / 3.3)));
  const cardWidth = Math.floor(cardHeight * MOBILE_BATTLE_CARD_ASPECT_RATIO);
  const landHeight = Math.floor(cardHeight * 0.65);
  const bandHeight = Math.max(cardHeight + landHeight + rowGap, topStatusHeight, controlHeight);
  const zonePileWidth = Math.max(28, Math.min(36, Math.floor((handPeek - 11) * 63 / 88)));
  const zonePilesWidth = zonePileWidth * 2;
  const totalHeight = fixedHeight + bandHeight * 2;

  return {
    viewportWidth: width,
    viewportHeight: height,
    sidePadding,
    rowGap,
    sectionGap,
    topStatusHeight,
    controlBandHeight: controlHeight,
    handPeekHeight: handPeek,
    cardWidth,
    cardHeight,
    landHeight,
    opponentBandHeight: bandHeight,
    selfBandHeight: bandHeight,
    selfBackVisibleHeight: landHeight,
    stackRailWidth: stackWidth,
    zonePileWidth,
    zonePilesWidth,
    battlefieldWidth: Math.max(1, width - sidePadding * 2 - zonePilesWidth - rowGap
      - (stackWidth ? stackWidth + sectionGap : 0)),
    totalHeight,
    fitsViewport: totalHeight <= height,
  };
}
