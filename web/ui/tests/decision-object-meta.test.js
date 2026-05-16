import test from "node:test";
import assert from "node:assert/strict";
import { buildObjectNameById } from "../src/lib/decision-object-meta.js";

test("viewed hidden placeholders do not overwrite live object names", () => {
  const names = buildObjectNameById({
    players: [
      {
        id: 0,
        hand_cards: [{ id: 101, name: "Black Lotus" }],
      },
    ],
    viewed_cards: {
      cards: [{ id: 101, name: "Hidden Card" }],
    },
  });

  assert.equal(names.get("101"), "Black Lotus");
});

test("viewed card names still populate objects that are not otherwise visible", () => {
  const names = buildObjectNameById({
    players: [{ id: 0 }],
    viewed_cards: {
      cards: [{ id: 202, name: "Selvala, Explorer Returned" }],
    },
  });

  assert.equal(names.get("202"), "Selvala, Explorer Returned");
});

test("a real viewed card name can replace an earlier hidden zone placeholder", () => {
  const names = buildObjectNameById({
    players: [
      {
        id: 0,
        exile_cards: [{ id: 303, name: "Hidden Card" }],
      },
    ],
    viewed_cards: {
      cards: [{ id: 303, name: "Black Lotus" }],
    },
  });

  assert.equal(names.get("303"), "Black Lotus");
});
