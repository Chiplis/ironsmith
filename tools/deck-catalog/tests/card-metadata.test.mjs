import test from "node:test";
import assert from "node:assert/strict";
import { buildManaProfile, cardMetadataKey, deckArtCard, enrichDeckWithManaProfile, manaValue, resolveCardMetadata } from "../card-metadata.mjs";

test("builds mana and land profile from resolved card metadata", () => {
  const deck = {
    mainboard: [
      { name: "Island", count: 8 },
      { name: "Swamp", count: 4 },
      { name: "Counterspell", count: 4 },
    ],
  };
  const metadata = {
    [cardMetadataKey("Island")]: { status: "ok", colors: [], colorIdentity: [], producedMana: ["U"], manaCost: "", typeLine: "Basic Land — Island" },
    [cardMetadataKey("Swamp")]: { status: "ok", colors: [], colorIdentity: [], producedMana: ["B"], manaCost: "", typeLine: "Basic Land — Swamp" },
    [cardMetadataKey("Counterspell")]: { status: "ok", colors: ["U"], colorIdentity: ["U"], producedMana: [], manaCost: "{U}{U}", typeLine: "Instant" },
  };
  const profile = buildManaProfile(deck, metadata);
  assert.deepEqual(profile.colors, ["B", "U"]);
  assert.equal(profile.landCount, 12);
  assert.deepEqual(profile.sourceCounts, { U: 8, B: 4 });
  assert.deepEqual(profile.predominantColors, ["U"]);
  assert.deepEqual(profile.predominantLands[0], { name: "Island", count: 8 });
  assert.equal(profile.metadataCoverage.complete, true);
});

test("caches successful and unresolved card metadata", async () => {
  const calls = [];
  const fetchImpl = async (url) => {
    calls.push(url);
    if (url.includes("Missing")) return { status: 404, ok: false };
    return { status: 200, ok: true, json: async () => ({ name: "Island", colors: [], color_identity: [], produced_mana: ["U"], type_line: "Basic Land — Island" }) };
  };
  const result = await resolveCardMetadata(["Island", "Missing"], { fetchImpl, minDelayMs: 0 });
  assert.equal(calls.length, 2);
  assert.equal(result[cardMetadataKey("Island")].status, "ok");
  assert.equal(result[cardMetadataKey("Missing")].status, "not_found");
});

test("reads a mana value from hybrid, Phyrexian and split costs", () => {
  assert.equal(manaValue("{2}{R}{R}"), 4);
  assert.equal(manaValue("{2/W}{2/W}"), 4);
  assert.equal(manaValue("{W/U}{W/P}"), 2);
  assert.equal(manaValue("{X}{X}{R}"), 1);
  assert.equal(manaValue("{2}{R} // {1}{R}"), 3);
  assert.equal(manaValue(""), 0);
});

test("takes a deck's art from its most expensive nonland", () => {
  const metadata = {
    [cardMetadataKey("Mountain")]: { status: "ok", typeLine: "Basic Land — Mountain", manaCost: "" },
    [cardMetadataKey("Urza's Saga")]: { status: "ok", typeLine: "Enchantment Land — Urza's Saga", manaCost: "" },
    [cardMetadataKey("Lightning Bolt")]: { status: "ok", typeLine: "Instant", manaCost: "{R}" },
    [cardMetadataKey("Goblin Rabblemaster")]: { status: "ok", typeLine: "Creature — Goblin", manaCost: "{2}{R}" },
    [cardMetadataKey("Atraxa")]: { status: "ok", typeLine: "Creature — Phyrexian Angel", manaCost: "{4}{W}{U}{B}{G}" },
  };
  const deck = {
    mainboard: [
      { name: "Mountain", count: 20 },
      { name: "Urza's Saga", count: 4 },
      { name: "Lightning Bolt", count: 4 },
      { name: "Goblin Rabblemaster", count: 4 },
      { name: "Atraxa", count: 1 },
    ],
  };
  assert.equal(deckArtCard(deck, metadata), "Atraxa");
  assert.equal(enrichDeckWithManaProfile(deck, metadata).artCard, "Atraxa");
  // A deck whose cards never resolved leaves the choice to the caller.
  assert.equal(deckArtCard(deck, {}), "");
});

test("breaks an equal mana value on copies, then name", () => {
  const metadata = {
    [cardMetadataKey("Bloodbraid Elf")]: { status: "ok", typeLine: "Creature — Elf", manaCost: "{2}{R}{G}" },
    [cardMetadataKey("Anger of the Gods")]: { status: "ok", typeLine: "Sorcery", manaCost: "{1}{R}{R}{R}" },
    [cardMetadataKey("Alpha Card")]: { status: "ok", typeLine: "Sorcery", manaCost: "{1}{R}{R}{R}" },
  };
  assert.equal(deckArtCard({ mainboard: [
    { name: "Bloodbraid Elf", count: 2 },
    { name: "Anger of the Gods", count: 4 },
  ] }, metadata), "Anger of the Gods");
  assert.equal(deckArtCard({ mainboard: [
    { name: "Anger of the Gods", count: 3 },
    { name: "Alpha Card", count: 3 },
  ] }, metadata), "Alpha Card");
});
