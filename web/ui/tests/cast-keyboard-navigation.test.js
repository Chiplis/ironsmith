import test from "node:test";
import assert from "node:assert/strict";
import { nextPlacementCell } from "../src/lib/cast-keyboard-navigation.js";

test("placement arrows skip occupied cells, move vertically, and stop at edges", () => {
  const cells = [{ row: 1, column: 1 }, { row: 1, column: 3 }, { row: 2, column: 1 }];
  assert.deepEqual(nextPlacementCell(cells, null, "ArrowRight"), cells[0]);
  assert.deepEqual(nextPlacementCell(cells, cells[0], "ArrowRight"), cells[1]);
  assert.deepEqual(nextPlacementCell(cells, cells[1], "ArrowRight"), cells[1]);
  assert.deepEqual(nextPlacementCell(cells, cells[1], "ArrowLeft"), cells[0]);
  assert.deepEqual(nextPlacementCell(cells, cells[0], "ArrowDown"), cells[2]);
  assert.deepEqual(nextPlacementCell(cells, cells[2], "ArrowUp"), cells[0]);
  assert.equal(nextPlacementCell([], null, "ArrowDown"), null);
});
