import test from "node:test";
import assert from "node:assert/strict";
import { getPlayerAccent } from "../src/lib/player-colors.js";

const players = [
  { id: 0, name: "Alice" },
  { id: 1, name: "Bob" },
  { id: 2, name: "Chandra" },
];

test("maps the perspective player to sapphire blue", () => {
  assert.equal(getPlayerAccent(players, 0, 0).hex, "#4484d7");
  assert.equal(getPlayerAccent(players, 1, 1).hex, "#4484d7");
  assert.equal(getPlayerAccent(players, 2, 2).hex, "#4484d7");
});

test("keeps other players colored relative to perspective order", () => {
  assert.equal(getPlayerAccent(players, 2, 1).hex, "#ff3b30");
  assert.equal(getPlayerAccent(players, 0, 1).hex, "#22c55e");
});
