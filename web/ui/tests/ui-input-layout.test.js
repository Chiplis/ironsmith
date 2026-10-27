import test from "node:test";
import assert from "node:assert/strict";
import { classifyViewport } from "../src/lib/viewport-layout.js";
import { parseNumberChoice } from "../src/lib/number-choice.js";

test("rotating a modern phone keeps the dedicated phone table", () => {
  assert.equal(classifyViewport(390, 844).portraitCompactViewport, true);
  for (const [width, height] of [[667, 375], [844, 390], [932, 430], [1023, 540]]) {
    const layout = classifyViewport(width, height);
    assert.equal(layout.landscapeMobileViewport, true, `${width}×${height}`);
    assert.equal(layout.nonDesktopViewport, true);
    assert.equal(layout.tabletCompactViewport, false);
  }
});

test("tablets get the desktop table and desktop tiers keep their controls", () => {
  for (const [width, height] of [[844, 768], [1023, 541], [768, 1024], [994, 927]]) {
    const layout = classifyViewport(width, height);
    assert.equal(layout.tabletCompactViewport, false, `${width}×${height}`);
    assert.equal(layout.nonDesktopViewport, false, `${width}×${height}`);
    assert.equal(layout.smallDesktopViewport, true, `${width}×${height}`);
  }
  assert.equal(classifyViewport(1280, 720).smallDesktopViewport, true);
  assert.equal(classifyViewport(1440, 900).nonDesktopViewport, false);
  assert.equal(classifyViewport(1920, 1080).largeDesktopViewport, true);
});

test("number decisions accept inclusive bounds, including zero and negative choices", () => {
  assert.equal(parseNumberChoice("0", 0, 10), 0);
  assert.equal(parseNumberChoice("10", 0, 10), 10);
  assert.equal(parseNumberChoice("-2", -2, 10), -2);
});

test("editing a number cannot accidentally submit an empty or illegal choice", () => {
  for (const value of ["", " ", "-1", "11", "1.5", "NaN", "Infinity", "1e100"]) {
    assert.equal(parseNumberChoice(value, 0, 10), null, value);
  }
});


test("unbounded X choices accept large values within the engine number range", () => {
  assert.equal(parseNumberChoice("1000000", 0, 0xffffffff), 1000000);
  assert.equal(parseNumberChoice("4294967295", 0, 0xffffffff), 0xffffffff);
  assert.equal(parseNumberChoice("4294967296", 0, 0xffffffff), null);
});
