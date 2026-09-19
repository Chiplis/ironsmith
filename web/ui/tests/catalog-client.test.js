import test from "node:test";
import assert from "node:assert/strict";
import { searchCatalogEntries } from "../src/lib/catalog-client.js";

const entries = [
  { id: "energy", name: "Boros Energy", archetype: "Boros Energy", cardNames: ["Guide of Souls"], date: "2026-09-18", placement: 2 },
  { id: "control", name: "Dimir Control", archetype: "Dimir Control", cardNames: ["Counterspell"], date: "2026-09-17", placement: 1 },
];

test("searches catalog entries by deck and card terms", () => {
  assert.deepEqual(searchCatalogEntries(entries, "Boros" ).map((entry) => entry.id), ["energy"]);
  assert.deepEqual(searchCatalogEntries(entries, "Guide Souls").map((entry) => entry.id), ["energy"]);
});

test("uses the generated token index to narrow candidates", () => {
  const searchIndex = { tokens: { counterspell: ["control"], dimir: ["control"] } };
  assert.deepEqual(
    searchCatalogEntries(entries, "Dimir Counterspell", { searchIndex }).map((entry) => entry.id),
    ["control"],
  );
  assert.deepEqual(searchCatalogEntries(entries, "missing", { searchIndex }), []);
});

test("falls back to catalog fields when the token index omits common words", () => {
  const searchIndex = { tokens: { song: ["song"], creation: ["song"] } };
  assert.deepEqual(
    searchCatalogEntries([
      { id: "song", name: "Song of Creation", cardNames: [] },
    ], "Song of Creation", { searchIndex }).map((entry) => entry.id),
    ["song"],
  );
});

test("ranks exact deck names ahead of card-only matches", () => {
  const results = searchCatalogEntries([
    { id: "card-match", name: "Control Shell", cardNames: ["Dimir Control"] },
    { id: "name-match", name: "Dimir Control", cardNames: ["Counterspell"] },
  ], "Dimir Control");
  assert.deepEqual(results.map((entry) => entry.id), ["name-match", "card-match"]);
});
