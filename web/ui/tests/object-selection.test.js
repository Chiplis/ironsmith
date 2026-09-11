import assert from "node:assert/strict";
import test from "node:test";

import {
  isObjectChosen,
  selectionAfterChoice,
} from "../src/lib/object-selection.js";

test("a card surface only ever adds its object to the choice", () => {
  const chosen = selectionAfterChoice([], { objectId: 12, mode: "add", max: 3 });
  assert.deepEqual(chosen, [12]);
  // Clicking a chosen card again keeps it: a search must not lose a pick to a
  // stray click.
  assert.equal(selectionAfterChoice(chosen, { objectId: 12, mode: "add", max: 3 }), chosen);
  assert.equal(selectionAfterChoice(chosen, { objectId: "12", mode: "add", max: 3 }), chosen);
});

test("the check badge is what gives a chosen object back", () => {
  assert.deepEqual(
    selectionAfterChoice([12, 15], { objectId: 12, mode: "remove", max: 3 }),
    [15],
  );
  assert.deepEqual(
    selectionAfterChoice([12, 15], { objectId: "15", mode: "remove", max: 3 }),
    [12],
  );
  const chosen = [12];
  assert.equal(selectionAfterChoice(chosen, { objectId: 99, mode: "remove" }), chosen);
});

test("the decision list keeps toggling, and the maximum still holds", () => {
  assert.deepEqual(selectionAfterChoice([12], { objectId: 12, max: 2 }), []);
  assert.deepEqual(selectionAfterChoice([12], { objectId: 15, max: 2 }), [12, 15]);
  const full = [12, 15];
  assert.equal(selectionAfterChoice(full, { objectId: 18, mode: "add", max: 2 }), full);
  assert.deepEqual(
    selectionAfterChoice(full, { objectId: 18, mode: "add", max: Infinity }),
    [12, 15, 18],
  );
  // Order survives so the engine reads the picks as they were made.
  assert.deepEqual(
    selectionAfterChoice([15, 12], { objectId: 18, mode: "add" }),
    [15, 12, 18],
  );
});

test("choices answer to either id shape and ignore missing ids", () => {
  assert.equal(isObjectChosen([12, 15], "12"), true);
  assert.equal(isObjectChosen(["12"], 12), true);
  assert.equal(isObjectChosen([12], 15), false);
  assert.equal(isObjectChosen([12], null), false);
  assert.equal(isObjectChosen(null, 12), false);
  const chosen = [12];
  assert.equal(selectionAfterChoice(chosen, { objectId: null, mode: "add" }), chosen);
});
