import test from "node:test";
import assert from "node:assert/strict";
import {
  battlefieldGridSlotAtPoint,
  battlefieldGridContentBounds,
  battlefieldLaneForCard,
  battlefieldPlacementForDrag,
  isPermanentCard,
} from "../src/lib/battlefield-layout.js";

test("future battlefield lanes mirror engine precedence", () => {
  assert.equal(battlefieldLaneForCard({ card_types: ["artifact", "land"] }), "artifacts");
  assert.equal(battlefieldLaneForCard({ card_types: ["artifact", "creature"] }), "creatures");
  assert.equal(battlefieldLaneForCard({ card_types: ["creature", "enchantment"] }), "enchantments");
  assert.equal(battlefieldLaneForCard({ type_line: "Legendary Planeswalker — Teferi" }), "planeswalkers");
});

test("only permanent plays create battlefield placement previews", () => {
  const cast = (card) => battlefieldPlacementForDrag({
    card,
    actions: [{ kind: "cast_spell" }],
  });

  assert.deepEqual(cast({ card_types: ["creature"] }), { lane: "creatures", kind: "cast_spell" });
  assert.equal(cast({ card_types: ["instant"] }), null);
  assert.equal(cast({ card_types: ["sorcery"] }), null);
  assert.equal(isPermanentCard({ type_line: "Artifact Creature — Thopter" }), true);
});

test("land actions get a land placement even when snapshot metadata is sparse", () => {
  assert.deepEqual(
    battlefieldPlacementForDrag({ actions: [{ action_ref: { kind: "play_land" } }] }),
    { lane: "lands", kind: "play_land" }
  );
});

test("battlefield moves reuse the permanent placement grid", () => {
  assert.deepEqual(
    battlefieldPlacementForDrag({
      card: { type_line: "Artifact Creature — Construct" },
      actions: [{ kind: "move_battlefield" }],
    }),
    { lane: "creatures", kind: "move_battlefield" }
  );
});

test("pointer coordinates resolve to the compact centered battlefield grid", () => {
  const layout = {
    left: 100,
    top: 50,
    width: 500,
    rows: 2,
    columns: 4,
    cardWidth: 72,
    cardHeight: 101,
    gap: 4,
    overlap: 0,
  };
  // Four 72px cards and three 4px gaps occupy 300px, centered at x=200.
  assert.deepEqual(
    battlefieldGridSlotAtPoint({ ...layout, x: 236, y: 100 }),
    { row: 1, column: 1 }
  );
  assert.deepEqual(
    battlefieldGridSlotAtPoint({ ...layout, x: 466, y: 160 }),
    { row: 2, column: 4 }
  );
  assert.equal(battlefieldGridSlotAtPoint({ ...layout, x: 120, y: 100 }), null);
});


test("drop slots follow the padded content box with a stack rail and scrollbar", () => {
  const element = {
    getBoundingClientRect: () => ({left: 10, top: 100, width: 1000}),
    clientLeft: 2, clientTop: 2, clientWidth: 981,
  };
  const bounds = battlefieldGridContentBounds(element, {
    paddingLeft: "400px", paddingRight: "21px", paddingTop: "22px", justifyContent: "center",
  });
  assert.deepEqual(bounds, {left: 412, top: 124, width: 560, alignStart: false});
  const layout = {...bounds, rows: 2, columns: 4, cardWidth: 100, cardHeight: 90, gap: 20, rowGap: 30};
  // 460px grid centered in 560px of usable space. Scroll the second row up 40px.
  for (let column = 1; column <= 4; column++) {
    assert.deepEqual(battlefieldGridSlotAtPoint({...layout, x: 512 + (column - 1) * 120, y: 249, scrollTop: 40}), {row: 2, column});
  }
  assert.equal(battlefieldGridSlotAtPoint({...layout, x: 300, y: 169}), null, "stack rail cannot receive cards");
  assert.equal(battlefieldGridSlotAtPoint({...layout, x: 512, y: 110}), null, "top padding cannot receive cards");
  assert.deepEqual(battlefieldGridSlotAtPoint({...layout, alignStart: true, x: 420, y: 169}), {row: 1, column: 1});
});
