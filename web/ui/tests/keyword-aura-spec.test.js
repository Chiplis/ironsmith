import test from "node:test";
import assert from "node:assert/strict";
import { cardKeywords, keywordAuraSpec, protectionTint } from "../src/lib/effect-palette.js";
import { FX } from "../src/lib/keyword-fx/flags.js";

test("keyword auras come from standalone keyword lines, not summoning sickness", () => {
  const spec = keywordAuraSpec({
    id: 7, stable_id: 70, summoning_sick: true,
    oracle_text: "Flying, vigilance\nIndestructible\nWard {2}\nCreatures you control have hexproof.",
  });
  assert.equal(spec.flags, FX.flying | FX.indestructible | FX.ward);
  assert.ok(spec.seed >= 0 && spec.seed < 1);
});

test("current ability lines win over printed oracle text", () => {
  const spec = keywordAuraSpec({ id: 1, oracle_text: "Flying", abilities: ["Hexproof"] });
  assert.equal(spec.flags, FX.hexproof);
});

test("shroud absorbs hexproof and protection picks a single color", () => {
  const card = { id: 2, oracle_text: "Shroud\nHexproof\nProtection from red" };
  assert.equal(keywordAuraSpec(card).flags, FX.shroud | FX.protection);
  assert.equal(protectionTint(card), "#ff7a5c");
  assert.ok(cardKeywords({ oracle_text: "Protection from black and from red" }).has("protection"));
  assert.notEqual(protectionTint({ oracle_text: "Protection from black and from red" }), "#ff7a5c");
});

test("combat marks attackers and blockers, including grouped members", () => {
  const combat = { attackers: [{ creature: 11, target: { kind: "player" } }], blockers: [{ blocker: 22, blocking: 11 }] };
  assert.equal(keywordAuraSpec({ id: 10, member_ids: [10, 11] }, combat).flags, FX.attacking);
  assert.equal(keywordAuraSpec({ id: 22 }, combat).flags, FX.blocking);
});

test("cards without auras and face-down cards get none", () => {
  assert.equal(keywordAuraSpec({ id: 3, oracle_text: "Trample", summoning_sick: true }), null);
  assert.equal(keywordAuraSpec({ id: 4, face_down: true, summoning_sick: true, oracle_text: "Flying" }), null);
});

test("neighbouring objects get unrelated aura seeds", () => {
  const seeds = [70, 71, 72, 73].map(id => keywordAuraSpec({ id, stable_id: id, oracle_text: "Flying" }).seed);
  for (const seed of seeds) assert.ok(seed >= 0 && seed < 1);
  for (let i = 1; i < seeds.length; i++) assert.ok(Math.abs(seeds[i] - seeds[i - 1]) > 0.01, `seeds ${seeds.join(", ")}`);
});
