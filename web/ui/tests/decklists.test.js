import assert from "node:assert/strict";
import test from "node:test";

import {
  MATCH_FORMAT_PLANECHASE,
  deckListText,
  evaluateLobbyDeckSubmission,
  isLobbyDeckReady,
  parseDeckList,
  parseCommanderList,
  parseDeckFoilNames,
  parseDeckPrintPreferences,
  parseSideboardList,
  parseStickerList,
  readDefaultLobbyDeck,
  saveDefaultLobbyDeck,
  splitSavedLobbyDeck,
} from "../src/lib/decklists.js";

test("MTGO sideboard headers separate cards and serialize as Sideboard", () => {
  for (const header of ["SIDEBOARD:", "Sideboard:", " sideboard : ", "SIDEBOARD"]) {
    const text = `2 Island\r\n${header}\r\n1 Negate\r\n`;
    assert.deepEqual(parseDeckList(text), ["Island", "Island"]);
    assert.deepEqual(parseSideboardList(text), ["Negate"]);
    assert.equal(deckListText(parseDeckList(text), parseSideboardList(text)),
      "2 Island\n\nSideboard\n1 Negate");
  }
  const saved = splitSavedLobbyDeck("Commander\n1 Atraxa, Grand Unifier\nSIDEBOARD:\n1 Negate");
  assert.equal(saved.commanderText, "1 Atraxa, Grand Unifier");
  assert.deepEqual(parseSideboardList(saved.deckText), ["Negate"]);
});

test("combined card names import their first half across deck sections and print preferences", () => {
  const text = [
    "2 Unholy Annex / Ritual Chamber (DSK) 118 *F*",
    "1 Fire // Ice [MH2:290]",
    "1 Wear/Tear",
    "SIDEBOARD:",
    "1 Valakut Awakening // Valakut Stoneforge",
  ].join("\n");
  assert.deepEqual(parseDeckList(text), ["Unholy Annex", "Unholy Annex", "Fire", "Wear"]);
  assert.deepEqual(parseSideboardList(text), ["Valakut Awakening"]);
  assert.deepEqual(parseCommanderList("1 Esika, God of the Tree / The Prismatic Bridge"),
    ["Esika, God of the Tree"]);
  assert.deepEqual(parseDeckFoilNames(text), ["Unholy Annex"]);
  assert.deepEqual(parseDeckPrintPreferences(text), [
    { name: "Unholy Annex", setCode: "DSK", collectorNumber: "118" },
    { name: "Fire", setCode: "MH2", collectorNumber: "290" },
  ]);
});

test("non-Commander lobbies accept at least 60 main-deck cards", () => {
  const planes = Array.from({ length: 10 }, (_, index) => `Plane ${index}`);
  for (const count of [0, 59, 60, 61, 80, 250]) {
    const deck = parseDeckList(`${count} Island\nSideboard\n15 Forest`);
    assert.equal(deck.length, count);
    assert.equal(isLobbyDeckReady(deck), count >= 60);
    for (const format of ["normal", "planechase", "standard", "pioneer", "modern", "legacy", "vintage", "pauper"]) {
      assert.equal(
        evaluateLobbyDeckSubmission(format, deck, format === "planechase" ? planes : []).ready,
        count >= 60,
        `${format}: ${count} cards`,
      );
    }
  }
});

test("Commander still requires exactly 100 cards including commanders", () => {
  for (const commanders of [["Commander"], ["Partner A", "Partner B"]]) {
    const required = 100 - commanders.length;
    for (const count of [required - 1, required, required + 1]) {
      assert.equal(
        evaluateLobbyDeckSubmission("commander", Array(count).fill("Island"), commanders).ready,
        count === required,
      );
    }
  }
});

test("Planechase lobby submissions require a normal deck and ten unique planar cards", () => {
  const mainDeck = Array.from({ length: 60 }, (_, index) => `Main ${index}`);
  const planarDeck = Array.from({ length: 10 }, (_, index) => `Plane ${index}`);

  assert.equal(
    evaluateLobbyDeckSubmission(MATCH_FORMAT_PLANECHASE, mainDeck, planarDeck).ready,
    true,
  );
  assert.equal(
    evaluateLobbyDeckSubmission(MATCH_FORMAT_PLANECHASE, mainDeck, planarDeck.slice(0, 9)).ready,
    false,
  );
  assert.equal(
    evaluateLobbyDeckSubmission(
      MATCH_FORMAT_PLANECHASE,
      mainDeck,
      [...planarDeck.slice(0, 9), "PLANE 0"],
    ).ready,
    false,
  );
});

function withMockLocalStorage(fn) {
  const previousWindow = globalThis.window;
  const store = new Map();
  globalThis.window = {
    localStorage: {
      getItem(key) {
        return store.has(key) ? store.get(key) : null;
      },
      setItem(key, value) {
        store.set(key, String(value));
      },
    },
  };

  try {
    return fn();
  } finally {
    if (previousWindow === undefined) {
      delete globalThis.window;
    } else {
      globalThis.window = previousWindow;
    }
  }
}

test("parseDeckList strips common print metadata from card names", () => {
  assert.deepEqual(
    parseDeckList([
      "1 Beast Within (clu) 165",
      "1 Beast Within [NPH] 103",
      "1 Beast Within [nph:103]",
      "1 Beast Within (CMM) 294 *F*",
    ].join("\n")),
    ["Beast Within", "Beast Within", "Beast Within", "Beast Within"],
  );
});

test("parseSideboardList strips common print metadata from card names", () => {
  assert.deepEqual(
    parseSideboardList([
      "1 Forest",
      "Sideboard",
      "1 Beast Within (clu) 165",
      "1 Beast Within [NPH] 103",
    ].join("\n")),
    ["Beast Within", "Beast Within"],
  );
});

test("default lobby deck persists main deck and commanders", () => {
  withMockLocalStorage(() => {
    saveDefaultLobbyDeck({
      deckText: "1 Sol Ring\n1 Island",
      commanderText: "1 Talrand, Sky Summoner",
    });

    const saved = readDefaultLobbyDeck();
    assert.deepEqual(saved, {
      deckText: "1 Sol Ring\n1 Island",
      commanderText: "1 Talrand, Sky Summoner",
      updatedAt: saved.updatedAt,
    });
    assert.ok(saved.updatedAt > 0);
  });
});

test("empty lobby deck submissions do not clear the saved default", () => {
  withMockLocalStorage(() => {
    saveDefaultLobbyDeck({
      deckText: "4 Lightning Bolt",
      commanderText: "",
    });

    saveDefaultLobbyDeck({ deckText: "", commanderText: "" });

    assert.equal(readDefaultLobbyDeck().deckText, "4 Lightning Bolt");
  });
});

test("a Stickers section lists sticker sheets apart from the deck and sideboard", () => {
  const text = [
    "Deck",
    "4 Island",
    "",
    "Sideboard",
    "1 Negate",
    "",
    "Stickers",
    "Ancestral Hot Dog Minotaur",
    "1 Cool Fluffy Loxodon",
    "",
  ].join("\n");
  assert.deepEqual(parseDeckList(text), ["Island", "Island", "Island", "Island"]);
  assert.deepEqual(parseSideboardList(text), ["Negate"]);
  assert.deepEqual(parseStickerList(text), ["Ancestral Hot Dog Minotaur", "Cool Fluffy Loxodon"]);
  assert.equal(
    deckListText(["Island", "Island"], ["Negate"], ["Ancestral Hot Dog Minotaur"]),
    "2 Island\n\nSideboard\n1 Negate\n\nStickers\n1 Ancestral Hot Dog Minotaur",
  );
  assert.deepEqual(parseStickerList(deckListText(["Island"], [], ["Cool Fluffy Loxodon"])), ["Cool Fluffy Loxodon"]);
});
