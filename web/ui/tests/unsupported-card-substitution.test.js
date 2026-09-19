import test from "node:test";
import assert from "node:assert/strict";
import {
  basicLandForManaCost,
  describeSubstitutions,
  dominantManaColor,
  applyKnownSubstitutions,
  resetKnownSubstitutions,
  substituteUnsupportedCards,
} from "../src/lib/unsupported-card-substitution.js";

test("takes the colour a card leans on most", () => {
  assert.equal(dominantManaColor("{2}{R}{R}"), "R");
  assert.equal(dominantManaColor("{B}{B}{B}{G}"), "B");
  assert.equal(dominantManaColor("{4}{W}{U}{B}{G}{G}"), "G");
});

test("breaks a tie in WUBRG order", () => {
  assert.equal(dominantManaColor("{G}{W}"), "W");
  assert.equal(dominantManaColor("{R}{U}"), "U");
  assert.equal(dominantManaColor("{G}{B}{R}"), "B");
});

test("reads hybrid, Phyrexian, colourless and split costs", () => {
  // Either half of a hybrid can pay it, so both colours count.
  assert.equal(dominantManaColor("{U/R}{R}"), "R");
  assert.equal(dominantManaColor("{G/W}"), "W");
  assert.equal(dominantManaColor("{B/P}{B}"), "B");
  assert.equal(dominantManaColor("{2}{C}"), "C");
  assert.equal(dominantManaColor("{3}"), "");
  assert.equal(dominantManaColor("{U}{U} // {2}{R}{R}"), "U");
});

test("maps a cost to its basic land, colourless when it has no colour", () => {
  assert.equal(basicLandForManaCost("{W}"), "Plains");
  assert.equal(basicLandForManaCost("{1}{U}"), "Island");
  assert.equal(basicLandForManaCost("{B}{B}"), "Swamp");
  assert.equal(basicLandForManaCost("{R}"), "Mountain");
  assert.equal(basicLandForManaCost("{G}{G}{W}"), "Forest");
  assert.equal(basicLandForManaCost("{2}"), "Wastes");
  assert.equal(basicLandForManaCost(""), "Wastes");
});

function stubFetch(costs) {
  return async (url) => {
    const route = String(url).split("/").pop().replace(/\.json$/, "");
    if (!Object.hasOwn(costs, route)) return { ok: false, status: 404 };
    return { ok: true, json: async () => ({ scryfall: { mana_cost: costs[route] } }) };
  };
}

test("swaps only the cards the engine cannot load, keeping the deck's size", async () => {
  const deck = ["Lightning Bolt", "Unbuilt Firebrand", "Lightning Bolt", "Mountain"];
  const sideboard = ["Unbuilt Firebrand", "Pyroblast"];
  const result = await substituteUnsupportedCards({ deck, sideboard }, {
    game: { filterKnownCardNames: async (names) => names.filter((name) => name !== "Unbuilt Firebrand") },
    fetchImpl: stubFetch({ "unbuilt-firebrand": "{1}{R}{R}" }),
  });

  assert.deepEqual(result.deck, ["Lightning Bolt", "Mountain", "Lightning Bolt", "Mountain"]);
  assert.deepEqual(result.sideboard, ["Mountain", "Pyroblast"]);
  assert.equal(result.deck.length, deck.length);
  assert.deepEqual(result.substitutions, [{ from: "Unbuilt Firebrand", to: "Mountain" }]);
});

test("falls back to the colourless basic when no printing is on hand", async () => {
  const result = await substituteUnsupportedCards({ deck: ["Unprinted Oddity"], sideboard: [] }, {
    game: { filterKnownCardNames: async () => [] },
    fetchImpl: stubFetch({}),
  });
  assert.deepEqual(result.deck, ["Wastes"]);
});

test("leaves a fully supported deck untouched", async () => {
  const deck = ["Llanowar Elves", "Forest"];
  const result = await substituteUnsupportedCards({ deck, sideboard: [] }, {
    game: { filterKnownCardNames: async (names) => names },
    fetchImpl: stubFetch({}),
  });
  assert.equal(result.deck, deck);
  assert.deepEqual(result.substitutions, []);

  // No engine to ask means no claim about what it supports.
  const withoutGame = await substituteUnsupportedCards({ deck, sideboard: [] }, {});
  assert.equal(withoutGame.deck, deck);
  assert.deepEqual(withoutGame.substitutions, []);
});

test("summarizes substitutions for the lobby status line", () => {
  assert.equal(describeSubstitutions([]), "");
  assert.equal(
    describeSubstitutions([{ from: "A", to: "Plains" }, { from: "B", to: "Island" }]),
    "A → Plains, B → Island",
  );
  assert.equal(
    describeSubstitutions([1, 2, 3, 4, 5].map((n) => ({ from: `C${n}`, to: "Swamp" }))),
    "C1 → Swamp, C2 → Swamp, C3 → Swamp +2 more",
  );
});

test("a decision made once is reapplied to a deck rebuilt from its text", async () => {
  resetKnownSubstitutions();
  await substituteUnsupportedCards({ deck: ["Reconnect Oddity"], sideboard: [] }, {
    game: { filterKnownCardNames: async () => [] },
    fetchImpl: stubFetch({ "reconnect-oddity": "{U}{U}{B}" }),
  });

  // The reconnect and host-takeover paths re-parse the player's original text;
  // they must land on the same list that was committed.
  assert.deepEqual(applyKnownSubstitutions(["Reconnect Oddity", "Island"]), ["Island", "Island"]);
  assert.deepEqual(applyKnownSubstitutions(["Island"]), ["Island"]);
  resetKnownSubstitutions();
  assert.deepEqual(applyKnownSubstitutions(["Reconnect Oddity"]), ["Reconnect Oddity"]);
});
