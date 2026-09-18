import test from "node:test";
import assert from "node:assert/strict";
import {
  DeckCatalogImportError,
  deckCatalogEntryToLobbyTexts,
  importDeckCatalogEntry,
  validateDeckCatalogEntry,
} from "../src/lib/deck-catalog-import.js";

const entry = {
  id: "modern-demo",
  format: "modern",
  sourceUrl: "https://mtgtop8.com/event?e=1&d=2&f=MO",
  mainboard: [{ name: "Mountain", count: 60 }],
  sideboard: [{ name: "Relic of Progenitus", count: 2 }],
  commander: [],
};

test("converts a catalog deck to the existing lobby text format", () => {
  const result = deckCatalogEntryToLobbyTexts(entry);
  assert.match(result.deckText, /^Deck\n60 Mountain/);
  assert.match(result.deckText, /Sideboard\n2 Relic of Progenitus/);
  assert.equal(result.commanderText, "");
});

test("validates counts before import", () => {
  const result = validateDeckCatalogEntry(entry);
  assert.equal(result.valid, true);
  assert.deepEqual(result.counts, { mainboard: 60, sideboard: 2, commander: 0 });
  assert.throws(
    () => importDeckCatalogEntry({ ...entry, mainboard: [{ name: "Mountain", count: 10 }] }),
    (error) => error instanceof DeckCatalogImportError && error.errors[0].includes("at least 60"),
  );
});

test("enforces Commander structure separately", () => {
  const commander = {
    format: "commander",
    mainboard: [{ name: "Island", count: 99 }],
    commander: [{ name: "Kraum, Ludevic's Opus", count: 1 }],
  };
  assert.equal(validateDeckCatalogEntry(commander).valid, true);
  assert.equal(validateDeckCatalogEntry({ ...commander, commander: [] }).valid, false);
});
