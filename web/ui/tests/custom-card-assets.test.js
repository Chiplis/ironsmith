import test from "node:test";
import assert from "node:assert/strict";
import {cardDisplayTypeLine} from "../src/lib/card-display-type-line.js";

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

test('lightweight live snapshots format engine types and retain current subtypes',()=>{
  assert.equal(cardDisplayTypeLine({card_types:['artifact','creature'],characteristic_signature:'types:artifact,creature\nsupertypes:\nsubtypes:Myr\noracle:Forest is just rules text'}),'Artifact Creature — Myr');
  assert.equal(cardDisplayTypeLine({card_types:['land'],characteristic_signature:'types:land\nsupertypes:legendary\nsubtypes:Mountain'}),'Legendary Land — Mountain');
  assert.equal(cardDisplayTypeLine({card_types:['creature'],subtypes:['Myr'],characteristic_signature:'types:artifact\nsupertypes:\nsubtypes:'}),'Artifact');
  assert.equal(cardDisplayTypeLine({card_types:['artifact','creature']}),'Artifact Creature');
});
