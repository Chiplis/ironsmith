import test from "node:test";
import assert from "node:assert/strict";
import {
  DeckValidationError,
  normalizeCompetitiveDeck,
  normalizeCompetitiveDecks,
} from "../normalize.mjs";

const sample = {
  format: "Modern",
  name: "Boros Energy",
  archetype: "Boros Energy",
  colors: ["r", "W", "invalid"],
  mechanics: ["Energy", "Energy"],
  event: "Modern Challenge",
  date: "2026-09-17",
  placement: 1,
  source: "MTGTop8",
  mainboard: [
    { name: "Guide of Souls", count: 4 },
    { name: "Guide of Souls", count: 1 },
  ],
  sideboard: [{ name: "Rest in Peace", count: 2 }],
  tags: ["Top 8", "Top 8"],
};

test("normalizes a deck deterministically", () => {
  const first = normalizeCompetitiveDeck(sample);
  const second = normalizeCompetitiveDeck({ ...sample, source: "mtgtop8" });

  assert.equal(first.format, "modern");
  assert.equal(first.name, "Boros Energy");
  assert.deepEqual(first.colors, ["R", "W"]);
  assert.deepEqual(first.mechanics, ["Energy"]);
  assert.equal(first.mainboard[0].count, 5);
  assert.deepEqual(first.tags, ["Top 8"]);
  assert.equal(first.hash, second.hash);
  assert.match(first.id, /^modern-mtgtop8-modern-challenge-2026-09-17-1-/);
});

test("rejects records without a mainboard", () => {
  assert.throws(
    () => normalizeCompetitiveDeck({ format: "modern", source: "mtgtop8", mainboard: [] }),
    (error) => error instanceof DeckValidationError && error.errors.includes("mainboard must contain at least one card"),
  );
});

test("deduplicates equivalent source records", () => {
  const decks = normalizeCompetitiveDecks([sample, { ...sample, id: "different-id" }]);
  assert.equal(decks.length, 1);
});
