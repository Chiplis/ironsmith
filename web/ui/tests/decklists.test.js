import assert from "node:assert/strict";
import test from "node:test";

import {
  MATCH_FORMAT_PLANECHASE,
  evaluateLobbyDeckSubmission,
  parseDeckList,
  parseSideboardList,
  readDefaultLobbyDeck,
  saveDefaultLobbyDeck,
} from "../src/lib/decklists.js";

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
