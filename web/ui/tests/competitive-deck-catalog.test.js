import test from "node:test";
import assert from "node:assert/strict";
import {
  competitiveDeckToLobbyText,
  normalizeCompetitiveDeckCatalog,
  searchCompetitiveDecks,
} from "../src/lib/competitive-deck-catalog.js";

const catalog = normalizeCompetitiveDeckCatalog({
  format: "modern",
  decks: [{
    id: "modern-example-1",
    format: "Modern",
    archetype: "Boros Energy",
    source: "mtgtop8",
    event: "Modern Challenge",
    date: "2026-09-17",
    placement: 1,
    mainboard: [{ name: "Guide of Souls", count: 4 }],
    sideboard: [{ name: "Rest in Peace", count: 2 }],
  }],
});

test("normalizes and searches decks by archetype or card", () => {
  assert.equal(catalog.decks.length, 1);
  assert.equal(searchCompetitiveDecks(catalog, { query: "guide of souls" }).length, 1);
  assert.equal(searchCompetitiveDecks(catalog, { format: "modern" })[0].placement, 1);
});

test("converts a competitive deck to the lobby parser format", () => {
  const result = competitiveDeckToLobbyText(catalog.decks[0]);
  assert.match(result.deckText, /Deck\n4 Guide of Souls/);
  assert.match(result.deckText, /Sideboard\n2 Rest in Peace/);
  assert.equal(result.commanderText, "");
});
