import test from "node:test";
import assert from "node:assert/strict";
import {readFile, access} from "node:fs/promises";
import {cardDisplayTypeLine} from "../src/lib/card-display-type-line.js";
import {customCardAssets, customCardImageUrl} from "../src/lib/custom-card-assets.js";
import {cardArtCropUrl} from "../src/lib/card-image-variants.js";
import {fullCardImageUrl} from "../src/lib/card-frame-colors.js";

test("hand snapshots display actual types and preserve inspector type changes", () => {
  assert.equal(cardDisplayTypeLine({card_types: ["Land"]}), "Land");
  assert.equal(cardDisplayTypeLine({card_types: ["Artifact", "Creature"]}), "Artifact Creature");
  assert.equal(cardDisplayTypeLine({card_types: ["Creature"]}, "Legendary Creature — Cat"), "Legendary Creature — Cat");
  assert.equal(cardDisplayTypeLine({type_line: "Artifact"}, "Creature — Cat"), "Artifact");
  assert.equal(cardDisplayTypeLine({type_line: "Artifact"}, "Artifact — Software", "Artifact"), "Artifact — Software");
  assert.equal(cardDisplayTypeLine({type_line: "Creature"}, "Artifact — Software", "Artifact"), "Creature");
  assert.equal(cardDisplayTypeLine({face_down: true, type_line: "Artifact"}, "Artifact — Software", "Artifact"), "");
  assert.equal(cardDisplayTypeLine({name: "Hidden card", card_types: []}, "Creature — Cat"), "");
});

test("every custom face has distinct local full and art assets and a real type", async () => {
  const pool = JSON.parse(await readFile(new URL("../public/custommagic/pool.json", import.meta.url)));
  for (const card of pool.cards) {
    for (const face of card.draft.faces) {
      const assets = customCardAssets(face.name);
      assert.ok(assets, face.name);
      assert.ok(assets.typeLine && assets.typeLine !== "Card", face.name);
      assert.equal(assets.flavorText, card.source.post);
      for (const key of ["manaCost", "colorIndicator", "power", "toughness", "loyalty", "defense"]) {
        assert.deepEqual(assets[key], face[key], face.name + " " + key);
      }
      for (const path of [assets.full, assets.art]) await access(new URL("../public/" + path, import.meta.url));
      const full = customCardImageUrl(face.name);
      const art = customCardImageUrl(face.name, "art_crop");
      assert.notEqual(full, art);
      assert.equal(cardArtCropUrl(full), art);
      assert.equal(fullCardImageUrl(art), full);
    }
  }
  assert.equal(customCardAssets("Cleveland").typeLine, "Land");
  assert.equal(customCardAssets("10 Fucking Sol Rings").typeLine, "Artifact");
});


test("every admitted card retains printed creature stats in the real engine", async () => {
  const {initWasmGame} = await import("../../../scripts/wasm-test-harness.mjs");
  const pool = JSON.parse(await readFile(new URL("../public/custommagic/pool.json", import.meta.url)));
  const {game} = await initWasmGame({pkg: "demo"});
  try {
    for (const card of pool.cards) {
      game.resetEmpty(["Alice", "Bob"], 20);
      const id = game.createCustomCard({draft: card.draft, playerIndex: 0, zoneName: "hand", skipTriggers: true});
      const details = game.objectDetails(id);
      const face = card.draft.faces[0];
      for (const stat of ["power", "toughness", "loyalty"]) {
        if (face[stat] != null) assert.equal(String(details[stat]), String(face[stat]), face.name + " " + stat);
      }
      assert.ok(details.type_line && details.type_line !== "Card", face.name);
    }
  } finally { game.free(); }
});
