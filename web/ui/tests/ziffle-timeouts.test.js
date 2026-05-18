import test from "node:test";
import assert from "node:assert/strict";
import {
  MAX_ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS,
  normalizeZiffleCardPositions,
  pendingActionIntentHardTimeoutMs,
  ziffleRevealTokenTimeoutMs,
} from "../src/lib/ziffle-timeouts.js";

test("ziffle reveal positions are normalized before timeout budgeting", () => {
  assert.deepEqual(
    normalizeZiffleCardPositions([2, "2", 3, 0], { deckCount: 5 }),
    [2, 3, 0],
  );
  assert.equal(
    ziffleRevealTokenTimeoutMs(1000, { deckCount: 60 }),
    60 * 2000,
  );
});

test("ziffle reveal positions reject invalid or out-of-range entries", () => {
  assert.throws(
    () => normalizeZiffleCardPositions([0, -1], { deckCount: 5 }),
    /invalid card position/,
  );
  assert.throws(
    () => normalizeZiffleCardPositions([0, ""], { deckCount: 5 }),
    /invalid card position/,
  );
  assert.throws(
    () => normalizeZiffleCardPositions([0, null], { deckCount: 5 }),
    /invalid card position/,
  );
  assert.throws(
    () => normalizeZiffleCardPositions([5], { deckCount: 5 }),
    /outside deck count 5/,
  );
  assert.throws(
    () => normalizeZiffleCardPositions([], { deckCount: 5 }),
    /missing a card position/,
  );
  assert.deepEqual(
    normalizeZiffleCardPositions([], { deckCount: 5 }, { allowEmpty: true }),
    [],
  );
});

test("pending action intent hard timeout is capped by maximum supported reveal work", () => {
  assert.equal(MAX_ZIFFLE_REVEAL_TOKEN_TIMEOUT_MS, 100 * 2000);
  assert.equal(
    pendingActionIntentHardTimeoutMs(60_000),
    (100 * 2000) + 60_000,
  );
});
