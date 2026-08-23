import assert from "node:assert/strict";
import test from "node:test";

import {
  buildIronsmithUrl,
  convertEldraziUrl,
  eldraziResponseToPuzzle,
  parseEldraziUrl,
  selectEldraziState,
} from "./eldrazi-to-ironsmith-url.mjs";

const sampleResponse = {
  defaultSaveSlotIndex: 1,
  state: {
    commandCards: [{ name: "Tayam, Luminous Enigma" }],
    deckName: "Tayam Time",
    handCards: [{ name: "Opening Hand Card" }],
    seats: ["You", "Opponent"],
  },
  saveStateSlots: [
    null,
    {
      lifeTotal: 37,
      zones: {
        battlefield: [{ name: "Forest", tapped: true }, { name: "Birds of Paradise" }],
        command: [{ name: "Tayam, Luminous Enigma" }],
        graveyard: [{ name: "Verdant Catacombs" }],
        hand: [{ name: "Carrion Feeder" }],
        library: [{ name: "Plains" }],
      },
    },
  ],
};

test("parseEldraziUrl extracts the hand and one-based save slot", () => {
  const parsed = parseEldraziUrl(
    "https://eldrazi.gg/playtest?moxfieldDeckId=deck&handId=hand-123&saveState=2",
  );
  assert.equal(parsed.handId, "hand-123");
  assert.equal(parsed.saveSlot, 2);
});

test("selectEldraziState uses the Eldrazi default save slot", () => {
  const selected = selectEldraziState(sampleResponse);
  assert.equal(selected.slotIndex, 1);
  assert.equal(selected.savedState.lifeTotal, 37);
});

test("eldraziResponseToPuzzle maps card names, life, and zones", () => {
  const puzzle = eldraziResponseToPuzzle(sampleResponse);
  assert.deepEqual(puzzle, {
    version: 1,
    players: [{
      name: "You",
      life: 37,
      zones: {
        ante: [],
        battlefield: ["Forest", "Birds of Paradise"],
        command: ["Tayam, Luminous Enigma"],
        exile: [],
        graveyard: ["Verdant Catacombs"],
        hand: ["Carrion Feeder"],
        library: ["Plains"],
      },
    }],
  });
});

test("eldraziResponseToPuzzle can omit a large library", () => {
  const puzzle = eldraziResponseToPuzzle(sampleResponse, { omitLibrary: true });
  assert.deepEqual(puzzle.players[0].zones.library, []);
});

test("buildIronsmithUrl produces a URL that decodes to the puzzle", () => {
  const puzzle = eldraziResponseToPuzzle(sampleResponse);
  const output = new URL(buildIronsmithUrl(puzzle, "http://localhost:5173/"));
  assert.equal(output.origin, "http://localhost:5173");
  const decoded = JSON.parse(Buffer.from(output.searchParams.get("puzzle"), "base64url"));
  assert.deepEqual(decoded, puzzle);
});

test("convertEldraziUrl calls the public hand endpoint", async () => {
  let requestedUrl = "";
  const fetchImpl = async (url) => {
    requestedUrl = url.toString();
    return { ok: true, json: async () => sampleResponse };
  };
  const converted = await convertEldraziUrl(
    "https://eldrazi.gg/playtest?handId=hand-123",
    { baseUrl: "https://example.test/ironsmith/", fetchImpl },
  );
  assert.equal(requestedUrl, "https://eldrazi.gg/api/playtest-from-hand?handId=hand-123");
  assert.match(converted.url, /^https:\/\/example\.test\/ironsmith\/\?puzzle=/);
});

test("an explicit empty save slot is rejected", () => {
  assert.throws(
    () => eldraziResponseToPuzzle(sampleResponse, { saveSlot: 1 }),
    /slot 1 is empty/,
  );
});
