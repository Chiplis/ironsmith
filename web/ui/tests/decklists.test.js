import assert from "node:assert/strict";
import test from "node:test";

import {
  MATCH_FORMAT_PLANECHASE,
  evaluateLobbyDeckSubmission,
  isLobbyDeckReady,
  parseDeckList,
  parseSideboardList,
  readDefaultLobbyDeck,
  listSavedDeckPresets,
  saveSavedDeckPreset,
  saveDefaultLobbyDeck,
  SAVED_DECK_PRESETS_LIMIT,
} from "../src/lib/decklists.js";

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

function withMockSessionStorage(fn) {
  const previousWindow = globalThis.window;
  const store = new Map();
  globalThis.window = {
    sessionStorage: {
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

test("saved deck presets stay in the browser session and stop at five", () => {
  withMockSessionStorage(() => {
    for (let index = 0; index < SAVED_DECK_PRESETS_LIMIT; index += 1) {
      assert.equal(saveSavedDeckPreset(`Deck ${index}`, [`${index} Island`], index === 0 ? ["Alice", "Bob"] : []).saved, true);
    }

    const rejected = saveSavedDeckPreset("Deck extra", ["1 Island"]);
    assert.equal(rejected.saved, false);
    assert.equal(rejected.reason, "limit");
    assert.equal(listSavedDeckPresets().length, SAVED_DECK_PRESETS_LIMIT);
    assert.deepEqual(listSavedDeckPresets().find((entry) => entry.name === "Deck 0")?.playerNames, ["Alice", "Bob"]);
  });
});
