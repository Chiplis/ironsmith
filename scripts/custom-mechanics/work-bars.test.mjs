import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {compileDraftInput,validateMechanic,mergeMechanics} from '../../web/ui/src/lib/custom-mechanics.js';
import {readWorkBarProgram,expandWorkBarProgram} from '../../web/ui/src/lib/reminder-work-bars.js';
const fixture=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/msem-gallery.json',import.meta.url),'utf8'));
const reminder=fixture.mechanics[0].executable_definitions[0].reminder;
test('subtype-scoped work sections retain top abilities, punctuation and targetless unlock instructions',()=>{
 const def=validateMechanic({keyword:'Exhibition',appliesToSubtype:'Museum',reminder});
 const original='{0}: You gain 3 life.\n{1}: Scene I: Dawn\n{0}: Target creature gets +2/+2 until end of turn.\n{2}: Study, Dusk\nCreatures you control get +1/+1.';
 const face=compileDraftInput({faces:[{name:'Witness',typeLine:'Enchantment — Museum',oracleText:original}]},[def]).faces[0];
 assert.equal(face.displayOracleText,original);
 assert.ok(face.oracleText.startsWith('{0}: You gain 3 life.\n{1}: Give this permanent'));
 assert.match(face.oracleText,/the "unlocked work named Scene I: Dawn" status/);
 assert.match(face.oracleText,/this permanent has "\{0\}: Target creature/);
 assert.equal(face.customKeywordIdentities,undefined,'category identity is its subtype, not a fabricated keyword');
 const foreign=compileDraftInput({faces:[{name:'Witness',typeLine:'Enchantment — Nonmuseum',oracleText:original}]},[def]).faces[0];
 assert.equal(foreign.oracleText,original);
});
test('independent granted keyword readers bind inside work bodies without giving a locked parent that keyword',()=>{
 const def=validateMechanic({keyword:'Exhibition',appliesToSubtype:'Museum',reminder});
 const harvest=validateMechanic({keyword:'Harvest witness',reminder:'At the beginning of your upkeep, you gain 1 life.'});
 const text='{0}: Secret Growth\nHarvest witness\n{0}: Quiet Study\nCreatures you control get +1/+1.';
 const face=compileDraftInput({customMechanics:[def,harvest],faces:[{name:'Witness',subtypes:['Museum'],oracleText:text}]},[def,harvest]).faces[0];
 assert.match(face.oracleText,/custom keyword "harvest witness"/);
 assert.match(face.oracleText,/"At the beginning of your upkeep, you gain 1 life\."/);
 assert.equal(face.customKeywordIdentities,undefined);
});
test('scope changes conflict, official keywords stay reserved, and incomplete layouts fail closed',()=>{
 const def=validateMechanic({keyword:'Exhibition',appliesToSubtype:'Museum',reminder});
 assert.throws(()=>mergeMechanics([def],[{...def,appliesToSubtype:'Othermuseum'}]));
 assert.throws(()=>validateMechanic({keyword:'Flying',appliesToSubtype:'Museum',reminder}));
 assert.throws(()=>expandWorkBarProgram('{0}: Lone Work\nFlying',def));
 assert.throws(()=>readWorkBarProgram(reminder+' Draw a card.'));
});
test('all sixteen source variants preserve complete originals and both work sections',()=>{
 assert.equal(fixture.mechanics[0].representative_cards.length,16);
 for(const card of fixture.mechanics[0].representative_cards){
  assert.equal(card.source_faces.length,card.executable_faces.length);
  assert.equal(card.original_text,card.source_faces[0].original_text);
  assert.equal(card.rewrite_status,'frontend_materialized_unverified');
  assert.equal((card.executable_text.match(/Give this permanent the unlocked work status/g)||[]).length,2,card.name);
  assert.equal((card.executable_text.match(/As long as this permanent has the "unlocked work named/g)||[]).length,2,card.name);
 }
});
