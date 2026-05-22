import test from "node:test";
import assert from "node:assert/strict";

import { isDisadvantageousActivePlayerClockAdvance } from "../src/lib/match-clock.js";

test("match clock accepts active player self-disadvantageous elapsed drift", () => {
  assert.equal(
    isDisadvantageousActivePlayerClockAdvance({
      actorIndex: 1,
      activePlayerIndex: 1,
      elapsedMs: 5000,
      observedElapsedMs: 1000,
      previousRemainingMsByPlayer: [10000, 10000],
      submittedRemainingMsByPlayer: [10000, 5000],
      skewMs: 200,
    }),
    true,
  );
});

test("match clock self-disadvantage exception is not available for favorable underreports", () => {
  assert.equal(
    isDisadvantageousActivePlayerClockAdvance({
      actorIndex: 1,
      activePlayerIndex: 1,
      elapsedMs: 1000,
      observedElapsedMs: 5000,
      previousRemainingMsByPlayer: [10000, 10000],
      submittedRemainingMsByPlayer: [10000, 9000],
      skewMs: 200,
    }),
    false,
  );
});

test("match clock self-disadvantage exception only applies to the ticking actor", () => {
  assert.equal(
    isDisadvantageousActivePlayerClockAdvance({
      actorIndex: 0,
      activePlayerIndex: 1,
      elapsedMs: 5000,
      observedElapsedMs: 1000,
      previousRemainingMsByPlayer: [10000, 10000],
      submittedRemainingMsByPlayer: [10000, 5000],
      skewMs: 200,
    }),
    false,
  );
});

test("match clock self-disadvantage exception does not bless timeout claims", () => {
  assert.equal(
    isDisadvantageousActivePlayerClockAdvance({
      actorIndex: 1,
      activePlayerIndex: 1,
      elapsedMs: 10000,
      observedElapsedMs: 1000,
      previousRemainingMsByPlayer: [10000, 10000],
      submittedRemainingMsByPlayer: [10000, 0],
      isTimeoutForfeit: true,
      skewMs: 200,
    }),
    false,
  );
});

test("match clock self-disadvantage exception requires internally consistent remaining time", () => {
  assert.equal(
    isDisadvantageousActivePlayerClockAdvance({
      actorIndex: 1,
      activePlayerIndex: 1,
      elapsedMs: 5000,
      observedElapsedMs: 1000,
      previousRemainingMsByPlayer: [10000, 10000],
      submittedRemainingMsByPlayer: [10000, 7000],
      skewMs: 200,
    }),
    false,
  );
});
