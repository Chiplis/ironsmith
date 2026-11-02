import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { compileDraftInput, validateMechanic } from '../src/lib/custom-mechanics.js';
const fixture = JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-sightsee-complete-source.json', import.meta.url)));
const row = fixture.mechanics[0];
const definition = validateMechanic(row.executable_definitions[0]);
test('Sightsee full sources preserve a catalog-derived basic domain including snow colorless', () => {
 assert.equal(row.representative_cards.length, 18);
 for (const card of row.representative_cards) {
  assert.equal(card.original_text, card.source_faces[0].original_text);
  const pool = card.supplied_card_definitions;
  assert.equal(new Set(pool.map(entry => entry.name)).size, 12);
  for (const entry of pool) {
   assert.match(entry.text, /Type: Basic(?: Snow)? Land/);
   assert.ok(definition.reminder.includes(JSON.stringify(entry.name)));
  }
  assert.ok(pool.some(entry => entry.name === 'Snow-Covered Wastes'));
 }
});
test('counted Sightsee repeats the whole choose-and-create program', () => {
 const result = compileDraftInput({ faces: [{ name: 'Independent traveler', oracleText: 'When this creature enters, sightsee three times.' }] }, [definition]);
 const text = result.faces[0].oracleText;
 const head='Perform the sightsee action as follows: ';
 const blocks=text.split(head);
 assert.equal(blocks.shift(),'When this creature enters, ');
 assert.equal(blocks.length,3,'three independently completed Sightsee operations, not one bulk creation');
 const names=row.representative_cards[0].supplied_card_definitions.map(entry=>entry.name);
 for(const block of blocks){
  assert.match(block,/^Choose /,'every operation makes a fresh name choice');
  const pieces=block.trim().split('; ');
  assert.equal(pieces.length,2,'one complete name menu followed by its own creation');
  assert.match(pieces[0],/ as a card name$/);
  for(const name of names) assert.equal(pieces[0].split(JSON.stringify(name)).length-1,1,`each fresh menu retains ${name}`);
  assert.equal(pieces[1],'Create a real card with the chosen name in your hand as part of the named conjure action.');
 }
 assert.equal((text.match(/named conjure action/g)||[]).length,3,'each independent choice has its own nested Conjure completion');
 assert.doesNotMatch(text, /[Cc]reate .*token/);
});
