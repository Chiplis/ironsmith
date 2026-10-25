import test from "node:test";
import assert from "node:assert/strict";
import { deckSearchUrl, parseDeckDetail, parseDeckSearch } from "../sources/archidekt.mjs";
import { catalogFormat } from "../formats.mjs";

const card = (name, categories, quantity = 1, faces = []) => ({ categories, quantity, card: { oracleCard: { name, faces } } });

test("Archidekt decks split commander, sideboard, and maybeboard by primary category", () => {
  const deck = parseDeckDetail({
    id: 42,
    name: "PDH Izzet Tokens",
    updatedAt: "2023-05-17T15:55:01Z",
    owner: { username: "someone" },
    categories: [
      { name: "Commander", isPremier: true, includedInDeck: true },
      { name: "Sideboard", isPremier: false, includedInDeck: true },
      { name: "Maybeboard", isPremier: false, includedInDeck: false },
    ],
    cards: [
      card("Third Path Iconoclast", ["Commander"]),
      card("Island", ["Land"], 30),
      card("Delver of Secrets // Insectile Aberration", ["Creature", "Sideboard"], 1, [{ name: "Delver of Secrets" }, { name: "Insectile Aberration" }]),
      card("Pyroblast", ["Sideboard"]),
      card("Rhystic Study", ["Maybeboard"]),
    ],
  }, { format: "paupercommander" });
  assert.deepEqual(deck.commander, [{ name: "Third Path Iconoclast", count: 1 }]);
  assert.deepEqual(deck.mainboard, [{ name: "Delver of Secrets", count: 1 }, { name: "Island", count: 30 }]);
  assert.deepEqual(deck.sideboard, [{ name: "Pyroblast", count: 1 }]);
  assert.equal(deck.id, "archidekt-42");
  assert.equal(deck.date, "2023-05-17");
  assert.equal(deck.sourceUrl, "https://archidekt.com/decks/42");
});

test("Archidekt search keeps public decks and routes Pauper EDH there", () => {
  const page = parseDeckSearch({
    next: "page-2",
    results: [{ id: 1, name: "A", size: 100 }, { id: 2, name: "B", size: 100, private: true }, { id: 3, name: "C", size: 100, theorycrafted: true }],
  });
  assert.equal(page.hasNext, true);
  assert.deepEqual(page.decks.map((deck) => deck.id), ["1"]);
  assert.match(deckSearchUrl({ format: "paupercommander", page: 3 }), /deckFormat=17.*page=3/);
  assert.deepEqual(catalogFormat("pdh"), { slug: "paupercommander", source: "archidekt" });
  assert.equal(catalogFormat("modern").source, "mtgtop8");
});
