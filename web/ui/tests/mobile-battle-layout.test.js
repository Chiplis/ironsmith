import test from "node:test";
import assert from "node:assert/strict";
import { solveMobileBattleLayout } from "../src/lib/mobile-battle-layout.js";

// Scene sizes exclude Safari's safe areas; 255px is the real iPhone 15 with
// browser controls visible (276px viewport minus the 21px bottom inset).
for (const height of [255, 268, 300, 320, 369, 422]) {
  test(`battlefields use the height previously reserved by controls in the ${height}px scene`, () => {
    const control = height <= 300 ? 32 : 36;
    const layout = solveMobileBattleLayout({
      viewportWidth: 734, viewportHeight: height,
      topBandHeight: control, controlBandHeight: control,
    });
    const actualHeight = layout.opponentBandHeight + layout.selfBandHeight + layout.handPeekHeight;
    assert.equal(actualHeight, layout.totalHeight);
    assert.ok(actualHeight <= height);
    assert.equal(layout.fitsViewport, true);
    assert.equal(layout.selfBackVisibleHeight, layout.landHeight);
    assert.equal(layout.cardHeight, Math.floor((height - layout.handPeekHeight) / 3.3));
    assert.ok(layout.handPeekHeight >= 32);
  });
}

test("measured, multiline actions retain their full height", () => {
  const layout = solveMobileBattleLayout({
    viewportWidth: 734, viewportHeight: 369,
    topBandHeight: 38.2, controlBandHeight: 58.3,
  });
  assert.equal(layout.topStatusHeight, 39);
  assert.equal(layout.controlBandHeight, 59);
  assert.equal(layout.fitsViewport, true);
});

test("the stack has reserved width instead of covering battlefield cards", () => {
  const without = solveMobileBattleLayout({viewportWidth: 734, viewportHeight: 320});
  const withStack = solveMobileBattleLayout({viewportWidth: 734, viewportHeight: 320, stackVisible: true});
  assert.equal(without.stackRailWidth, 0);
  assert.equal(withStack.battlefieldWidth + withStack.stackRailWidth + withStack.sectionGap, without.battlefieldWidth);
  assert.equal(withStack.cardHeight, without.cardHeight);
});

test("both zone piles reserve space beside each player's battlefield", () => {
  const layout = solveMobileBattleLayout({ viewportWidth: 568, viewportHeight: 255, stackVisible: true });
  assert.ok(layout.zonePileWidth >= 16);
  assert.ok(layout.zonePileWidth * 88 / 63 + 11 <= layout.handPeekHeight);
  assert.equal(layout.battlefieldWidth + layout.zonePilesWidth + layout.rowGap
    + layout.stackRailWidth + layout.sectionGap + layout.sidePadding * 2, 568);
});

test("an impossibly short scene reports overflow without shrinking controls or hit targets", () => {
  const layout = solveMobileBattleLayout({viewportWidth: 568, viewportHeight: 100, controlBandHeight: 60});
  assert.equal(layout.fitsViewport, false);
  assert.equal(layout.controlBandHeight, 60);
  assert.equal(layout.cardHeight, 24);
  assert.ok(layout.totalHeight > 100);
});
