import test from "node:test";
import assert from "node:assert/strict";
import {withOfficialOracleText} from "../src/lib/hand-oracle-text.js";

test("hand cards take official Oracle text for single cards, the named face, and every split face", () => {
  const compiled = {name: "Lightning Bolt", oracle_text: "compiled"};
  assert.equal(withOfficialOracleText(compiled, {oracle_text: "Lightning Bolt deals 3 damage to any target."}).oracle_text,
    "Lightning Bolt deals 3 damage to any target.");
  const mdfc = {name: "Delver of Secrets", card_faces: [{name: "Delver of Secrets", oracle_text: "upkeep"}, {name: "Insectile Aberration", oracle_text: "Flying"}]};
  assert.equal(withOfficialOracleText({name: "Delver of Secrets", oracle_text: "compiled"}, mdfc).oracle_text, "upkeep");
  const split = withOfficialOracleText({name: "Fire // Ice", card_faces: [{name: "Fire", oracle_text: "a"}, {name: "Ice", oracle_text: "b"}]},
    {card_faces: [{name: "Fire", oracle_text: "Fire text"}, {name: "Ice", oracle_text: "Ice text"}]});
  assert.deepEqual(split.card_faces.map(face => face.oracle_text), ["Fire text", "Ice text"]);
});

test("missing official text keeps the compiled text", () => {
  const card = {name: "Unknown", oracle_text: "compiled"};
  assert.equal(withOfficialOracleText(card, null), card);
  assert.equal(withOfficialOracleText(card, {oracle_text: undefined}).oracle_text, "compiled");
  assert.equal(withOfficialOracleText(card, {card_faces: [{name: "Other", oracle_text: "x"}]}).oracle_text, "compiled");
});
