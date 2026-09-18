import test from "node:test";
import assert from "node:assert/strict";
import { buildSearchIndex, searchIndex, tokenize } from "../index.mjs";

const entries = [
  {
    id: "deck-boros",
    format: "modern",
    archetype: "Boros Energy",
    event: "Modern Challenge",
    source: "mtgtop8",
    tags: ["Top 8"],
    cards: ["Guide of Souls", "Ocelot Pride"],
  },
  {
    id: "deck-dimir",
    format: "modern",
    archetype: "Dimir Control",
    event: "Modern Challenge",
    source: "mtgtop8",
    tags: [],
    cards: ["Psychic Frog", "Counterspell"],
  },
];

test("tokenizes accents and ignores short/common words", () => {
  assert.deepEqual(tokenize("Ángeles of Control"), ["angeles", "control"]);
});

test("builds an inverted index and intersects query tokens", () => {
  const index = buildSearchIndex(entries, { generatedAt: "now" });
  assert.deepEqual(searchIndex(index, "Boros Energy"), ["deck-boros"]);
  assert.deepEqual(searchIndex(index, "Modern Challenge"), ["deck-boros", "deck-dimir"]);
});
