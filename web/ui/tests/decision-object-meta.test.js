import test from "node:test";
import assert from "node:assert/strict";
import { buildObjectNameById, optionForClickedObject, optionForClickedObjects } from "../src/lib/decision-object-meta.js";

const optionDecision = (options) => ({ kind: "select_options", min: 1, max: 1, options });

test("a permanent shared by two legal modes does not choose the first mode", () => {
  const decision = optionDecision([
    { index: 0, description: "Destroy target artifact", legal: true, related_object_ids: [42] },
    { index: 1, description: "Destroy target enchantment", legal: true, related_object_ids: [42] },
  ]);
  assert.equal(optionForClickedObject(decision, 42), null);
  decision.options[0].legal = false;
  assert.equal(optionForClickedObject(decision, 42), decision.options[1], "only legal matches count");
});

test("a merged permanent cannot choose between different options on its members", () => {
  const decision = optionDecision([
    { index: 3, description: "Sacrifice this creature", object_id: 40 },
    { index: 9, description: "Return this creature to hand", object_id: 41 },
  ]);
  assert.equal(optionForClickedObjects(decision, [40, 41]), null);
  assert.equal(optionForClickedObjects(decision, [41, 40]), null);
  assert.equal(optionForClickedObjects(decision, [null, 40, "40"]), decision.options[0]);
});

test("one mode related to several members remains a unique choice", () => {
  const mode = { index: 7, description: "Destroy this permanent", related_object_ids: [40, 41] };
  const decision = optionDecision([mode, { index: 8, description: "Draw a card" }]);
  assert.equal(optionForClickedObjects(decision, [40, 41]), mode);
});

test("explicit equivalent action groups preserve the clicked member shortcut", () => {
  const first = { index: 3, description: "Tap: Add {G}", object_id: 40, legal: true };
  const second = { index: 9, description: "Tap: Add {G}", object_id: 41, legal: true };
  const decision = optionDecision([{ ...first, grouped_options: [first, second] }]);
  assert.equal(optionForClickedObject(decision, 41), second);
  assert.equal(optionForClickedObjects(decision, [40, 41]).index, first.index);
  first.legal = false;
  decision.options[0].legal = false;
  assert.equal(optionForClickedObjects(decision, [40, 41]), second);
});

test("matching descriptions alone do not make independent options equivalent", () => {
  const first = { index: 3, description: "Sacrifice a creature", object_id: 40 };
  const second = { index: 9, description: "Sacrifice a creature", object_id: 41 };
  assert.equal(optionForClickedObjects(optionDecision([first, second]), [40, 41]), null);
  const distinct = { ...second, description: "Return a creature to hand" };
  assert.equal(optionForClickedObjects(optionDecision([{ ...first, grouped_options: [first, distinct] }]), [40, 41]), null);
});

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

test("inspector-only viewed card previews do not populate decision metadata", () => {
  const names = buildObjectNameById({
    players: [{ id: 0 }],
    viewed_cards: {
      inspector_only: true,
      cards: [{ id: 404, name: "Swamp" }],
    },
  });

  assert.equal(names.has("404"), false);
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
