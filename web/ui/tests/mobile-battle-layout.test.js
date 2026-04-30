import test from "node:test";
import assert from "node:assert/strict";
import {
  solveMobileBattleLayout,
  MOBILE_HAND_PEEK_HEIGHT_PX,
  MOBILE_MANA_POOL_HEIGHT_PX,
  MOBILE_SELF_HUD_HEIGHT_PX,
  MOBILE_STACK_RAIL_WIDTH_PX,
} from "../src/lib/mobile-battle-layout.js";

const HEIGHTS = [390, 340, 320, 300, 280];

for (const height of HEIGHTS) {
  test(`mobile battle layout fits viewport at ${height}px height`, () => {
    const layout = solveMobileBattleLayout({
      viewportWidth: 844,
      viewportHeight: height,
      topBandHeight: 42,
      controlBandHeight: 38,
      collapsedHandRailHeight: 58,
      opponentFrontCount: 7,
      opponentBackCount: 7,
      selfFrontCount: 7,
      selfBackCount: 7,
    });

    assert.equal(layout.fitsViewport, true);
    assert.ok(layout.totalHeight <= layout.viewportHeight);
    assert.ok(layout.selfFrontHeight === layout.cardHeight);
    assert.ok(layout.selfBackVisibleHeight >= Math.floor(layout.cardHeight * 0.78));
    assert.equal(layout.selfBackVisibleRatio, 0.78);

    const usableWidth = layout.viewportWidth - (layout.sidePadding * 2);
    const rowWidth = (layout.cardWidth * 7) + (layout.rowGap * 6);
    assert.ok(rowWidth <= usableWidth);
  });
}

test("mobile battle layout preserves mixed battlefield rows without overlap", () => {
  const layout = solveMobileBattleLayout({
    viewportWidth: 844,
    viewportHeight: 320,
    topBandHeight: 44,
    controlBandHeight: 72,
    collapsedHandRailHeight: 58,
    opponentFrontCount: 5,
    opponentBackCount: 7,
    selfFrontCount: 4,
    selfBackCount: 6,
  });

  assert.equal(layout.fitsViewport, true);
  assert.ok(layout.cardWidth > 0);
  assert.ok(layout.cardHeight > 0);
  assert.ok(layout.opponentBandHeight >= (layout.cardHeight * 2));
  assert.ok(layout.bottomBandHeight >= 46);
});

for (const height of HEIGHTS) {
  test(`mtga-aligned mobile layout fits viewport at ${height}px height with new regions`, () => {
    const layout = solveMobileBattleLayout({
      viewportWidth: 844,
      viewportHeight: height,
      topBandHeight: 38,
      controlBandHeight: 30,
      collapsedHandRailHeight: 0,
      opponentManaPoolHeight: MOBILE_MANA_POOL_HEIGHT_PX,
      selfManaPoolHeight: MOBILE_MANA_POOL_HEIGHT_PX,
      selfHudHeight: MOBILE_SELF_HUD_HEIGHT_PX,
      handPeekHeight: MOBILE_HAND_PEEK_HEIGHT_PX,
      stackVisible: false,
      opponentFrontCount: 6,
      opponentBackCount: 7,
      selfFrontCount: 6,
      selfBackCount: 7,
    });

    assert.equal(layout.fitsViewport, true, `expected layout to fit at ${height}px`);
    assert.ok(layout.totalHeight <= layout.viewportHeight);
    assert.ok(layout.cardHeight >= 24);
    if (height > 320) {
      assert.ok(layout.opponentManaPoolHeight > 0, `opponent mana pool should render outside compact mode (${height}px)`);
    } else {
      // Compact mode (≤320px) drops the opponent mana pool to keep core gameplay visible.
      assert.equal(layout.opponentManaPoolHeight, 0, `opponent mana pool should drop in compact mode (${height}px)`);
    }
    assert.ok(layout.selfManaPoolHeight > 0);
    assert.ok(layout.selfHudHeight > 0);
    assert.ok(layout.handPeekHeight > 0);
    assert.equal(layout.stackRailWidth, 0);
  });
}

test("stack rail reserves horizontal space when stackVisible", () => {
  const without = solveMobileBattleLayout({
    viewportWidth: 844,
    viewportHeight: 390,
    topBandHeight: 38,
    controlBandHeight: 30,
    opponentFrontCount: 7,
    opponentBackCount: 7,
    selfFrontCount: 7,
    selfBackCount: 7,
  });
  const withStack = solveMobileBattleLayout({
    viewportWidth: 844,
    viewportHeight: 390,
    topBandHeight: 38,
    controlBandHeight: 30,
    opponentFrontCount: 7,
    opponentBackCount: 7,
    selfFrontCount: 7,
    selfBackCount: 7,
    stackVisible: true,
  });

  assert.equal(without.stackRailWidth, 0);
  assert.equal(withStack.stackRailWidth, MOBILE_STACK_RAIL_WIDTH_PX);
  assert.ok(withStack.cardWidth <= without.cardWidth);
});

test("compact-mode shrinks the new region heights at <=320px", () => {
  const compact = solveMobileBattleLayout({
    viewportWidth: 844,
    viewportHeight: 300,
    topBandHeight: 38,
    controlBandHeight: 30,
    collapsedHandRailHeight: 0,
    opponentManaPoolHeight: MOBILE_MANA_POOL_HEIGHT_PX,
    selfManaPoolHeight: MOBILE_MANA_POOL_HEIGHT_PX,
    selfHudHeight: MOBILE_SELF_HUD_HEIGHT_PX,
    handPeekHeight: MOBILE_HAND_PEEK_HEIGHT_PX,
    opponentFrontCount: 6,
    opponentBackCount: 7,
    selfFrontCount: 6,
    selfBackCount: 7,
  });

  assert.equal(compact.compactMode, true);
  assert.equal(compact.fitsViewport, true);
  // Compact mode drops the opponent mana pool entirely and shrinks the self-side regions.
  assert.equal(compact.opponentManaPoolHeight, 0);
  assert.ok(compact.selfManaPoolHeight < MOBILE_MANA_POOL_HEIGHT_PX);
  assert.ok(compact.selfHudHeight < MOBILE_SELF_HUD_HEIGHT_PX);
  assert.ok(compact.handPeekHeight < MOBILE_HAND_PEEK_HEIGHT_PX);
});
