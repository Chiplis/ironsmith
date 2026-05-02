import test from "node:test";
import assert from "node:assert/strict";
import { getPlayerAccent } from "../src/lib/player-colors.js";

const players = [
  { id: 0, name: "Alice" },
  { id: 1, name: "Bob" },
  { id: 2, name: "Chandra" },
];

test("maps the perspective player to red", () => {
  assert.equal(getPlayerAccent(players, 0, 0).hex, "#ff3b30");
  assert.equal(getPlayerAccent(players, 1, 1).hex, "#ff3b30");
  assert.equal(getPlayerAccent(players, 2, 2).hex, "#ff3b30");
});

test("keeps other players colored relative to perspective order", () => {
  assert.equal(getPlayerAccent(players, 2, 1).hex, "#3b82f6");
  assert.equal(getPlayerAccent(players, 0, 1).hex, "#f4c430");
});
