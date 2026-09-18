import test from "node:test";
import assert from "node:assert/strict";
import { buildManaProfile, cardMetadataKey, resolveCardMetadata } from "../card-metadata.mjs";

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
