import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const fixture = name => JSON.parse(fs.readFileSync(new URL(`../../../fixtures/custom_mechanics/${name}`,import.meta.url)));
const menus = fixture('msem-audition-choices.json').mechanics;
const modifier = fixture('msem-audition-original-menu-modifier.json').mechanics[0];
test('every formal color menu retains its exact original Role choices beneath one named modal owner',()=>{
 const expected=[['Cleric','Scout'],['Warlock','Wizard'],['Warrior','Wizard'],['Scout','Warrior'],['Cleric','Warlock'],['Warlock','Warrior','Wizard']];
 for(const [index,row] of menus.entries()){
  const raw=row.executable_definitions[0];
  const definition=validateMechanic(raw);
  const text=compileDraftInput({faces:[{name:'Stage',oracleText:`Audition ${raw.template}.`}]},[definition]).faces[0].oracleText;
  assert.match(text,/Perform the audition action as follows: \[Choose one of the following:/i);
  const roles=[...text.matchAll(/named "([A-Za-z]+) Role"/g)].map(match=>match[1]).sort();
  assert.deepEqual(roles,expected[index].slice().sort());
  assert.equal((text.match(/1\/1 colorless Construct Actor/g)||[]).length,1);
  assert.ok(!/If you chose/i.test(text));
 }
});
test('source-declared passive modifier expands only its complete authored literal line',()=>{
 const card=modifier.representative_cards[0];
 const defs=modifier.executable_definitions.map(validateMechanic);
 const raw=card.original_text.split('\n').find(line=>line.startsWith('Auditioning'));
 const face=compileDraftInput({faces:[{name:card.name,oracleText:raw}]},defs).faces[0];
 assert.equal(face.oracleText,'If you would perform the audition action, perform its mode numbered 1, then choose one of its other modes instead.');
 assert.equal(face.displayOracleText,raw);
 for(const changed of [raw+' Draw a card.',raw.replace('1/1','2/2')])
  assert.equal(compileDraftInput({faces:[{name:card.name,oracleText:changed}]},defs).faces[0].oracleText,changed);
});
