import test from "node:test";
import assert from "node:assert/strict";
import { sampleCustomCards, replaceHandWithCustomCards } from "../src/lib/custom-card-pool.js";
import { initWasmGame, getHand } from "../../../scripts/wasm-test-harness.mjs";

function draft(name, oracleText = "Draw a card.") {
  return { layout: "single", hasFuse: false, faces: [{
    name, manaCost: "{U}", colorIndicator: [], supertypes: [],
    cardTypes: ["Instant"], subtypes: [], oracleText,
    power: null, toughness: null, loyalty: null, defense: null,
  }] };
}

test("sampling fills seven, avoids duplicates when possible, and rejects an empty pool", () => {
  assert.throws(() => sampleCustomCards([]), /no verified cards/);
  assert.equal(new Set(sampleCustomCards(Array.from({length: 20}, (_, i) => i))).size, 7);
  const small = sampleCustomCards(["a", "b"], 7);
  assert.equal(small.length, 7);
  for (let i = 0; i < 6; i += 2) assert.equal(new Set(small.slice(i, i + 2)).size, 2);
});

test("real engine replaces only the selected hand and restores it on compile or insertion failure", async () => {
  const {game} = await initWasmGame({pkg: "demo"});
  try {
    game.resetEmpty(["Alice", "Bob"], 20);
    game.createCustomCard({draft: draft("Old hand"), playerIndex: 0, zoneName: "hand", skipTriggers: true});
    game.createCustomCard({draft: draft("Other hand"), playerIndex: 1, zoneName: "hand", skipTriggers: true});
    const drafts = Array.from({length: 7}, (_, i) => draft(`Pool test ${i}`));
    await replaceHandWithCustomCards(game, 0, drafts);
    const state = game.snapshot();
    assert.equal(state.players[0].hand_size, 7);
    assert.equal(state.players[1].hand_size, 1);
    const names = getHand(game, 0).map(card => card.name);
    assert.deepEqual(names, drafts.map(card => card.faces[0].name));
    const invalid = [...drafts.slice(0, 6), draft("Invalid test", "Flibbertigibbet the moon.")];
    await assert.rejects(() => replaceHandWithCustomCards(game, 0, invalid));
    assert.deepEqual(getHand(game, 0).map(card => card.name), names);
    // Exercise rollback after clearing and inserting a card, not just preview.
    let added = 0;
    const failing = new Proxy(game, {get(target, key) {
      if (key === "createCustomCard") return (...args) => {
        if (++added === 2) throw new Error("insertion failed");
        return target.createCustomCard(...args);
      };
      const value = target[key];
      return typeof value === "function" ? value.bind(target) : value;
    }});
    await assert.rejects(() => replaceHandWithCustomCards(failing, 0, drafts), /insertion failed/);
    assert.deepEqual(getHand(game, 0).map(card => card.name), names);
    assert.equal(game.snapshot().players[1].hand_size, 1);
  } finally { game.free(); }
});

test("persistent draw bag exhausts each cycle and avoids repeats across hand boundaries", async () => {
  const {createCustomCardSampler} = await import("../src/lib/custom-card-pool.js");
  const values = new Map();
  const storage = {getItem: key => values.get(key), setItem: (key, value) => values.set(key, value)};
  const pool = Array.from({length: 23}, (_, id) => ({id}));
  let sample = createCustomCardSampler({storage, random: () => 0.37});
  const draws = [];
  let previous = [];
  for (let click = 0; click < 30; click++) {
    // Simulate reloads and both button placements.
    sample = createCustomCardSampler({storage, random: () => 0.37});
    const hand = sample(pool).map(card => card.id);
    assert.equal(new Set(hand).size, 7);
    assert.equal(hand.filter(id => previous.includes(id)).length, 0);
    draws.push(...hand);
    previous = hand;
  }
  for (let i = 0; i + 23 <= draws.length; i += 23) {
    assert.equal(new Set(draws.slice(i, i + 23)).size, 23);
  }
  values.set("ironsmith-custom-card-draw-bag-v1", "broken");
  assert.equal(sample(pool).length, 7);
  assert.equal(sample(pool.slice(0, 3)).length, 7);
});

test("every shipped custom card carries its source URL as display flavor without changing rules", async () => {
  const {readFile} = await import("node:fs/promises");
  const {registerCustomCardFlavor, customCardFlavorText} = await import("../src/lib/custom-card-flavor.js");
  const pool = JSON.parse(await readFile(new URL("../public/custommagic/pool.json", import.meta.url)));
  assert.ok(pool.cards.length > 0);
  registerCustomCardFlavor(pool.cards);
  for (const card of pool.cards) {
    assert.match(card.source.post, /^https:\/\/www\.reddit\.com\/r\/custommagic\/comments\//);
    for (const face of card.draft.faces) {
      assert.equal(face.flavorText, card.source.post);
      assert.equal(customCardFlavorText(face.name), card.source.post);
      assert.ok(!face.oracleText.includes(card.source.post));
    }
  }
});
