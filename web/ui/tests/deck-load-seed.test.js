import test from "node:test";
import assert from "node:assert/strict";
import { withFreshDeckLoadSeed } from "../src/lib/deck-load-seed.js";

test("deck loads get a fresh shuffle seed so the same deck does not deal the same hand", () => {
  const decks = [["Island"], ["Plains"]];
  let next = 41;
  const random = () => (next += 1);
  const first = withFreshDeckLoadSeed({ decks, preserveMissingDecks: true }, random);
  const second = withFreshDeckLoadSeed({ decks, preserveMissingDecks: true }, random);
  assert.equal(first.seed, 42);
  assert.equal(second.seed, 43);
  assert.equal(first.preserveMissingDecks, true);
  assert.deepEqual(withFreshDeckLoadSeed(decks, random), { decks, seed: 44 });
});

test("an explicit seed is kept so journal replays reshuffle identically", () => {
  const payload = { decks: [["Island"]], seed: 7 };
  assert.equal(withFreshDeckLoadSeed(payload, () => 99), payload);
  assert.equal(typeof withFreshDeckLoadSeed({ decks: [] }).seed, "number");
});
