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
