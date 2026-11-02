import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {expandActionParameterDeclarations} from '../src/lib/reminder-action-parameters.js';
import {expandNamedActionReferences} from '../src/lib/reminder-action-references.js';
import {customKeywordForms} from '../src/lib/keyword-forms.js';
const fixture=JSON.parse(readFileSync(new URL('../../../fixtures/custom_mechanics/msem-oaths-source-complete.json',import.meta.url)));
const row=fixture.mechanics[0], defs=row.executable_definitions;
const expand=text=>defs.reduce((value,def)=>expandActionParameterDeclarations(value,def,'Example Knight'),text);
test('declared multiword Subject action inflects its verb head and keeps the recipient controller',()=>{
 const text=expand('When this creature enters, target creature seeks secrets.');
 assert.match(text,/Choose target creature\. Its controller creates an Oath of Secrets token attached to that creature/i);
 assert.doesNotMatch(text,/seeks secrets/i);
});
test('choice between declared outcomes belongs to the recipient controller',()=>{
 const text=expand('When Example Knight enters, Example Knight seeks secrets or valor.');
 assert.match(text,/Its controller chooses one of the following:/);
 assert.match(text,/\[Its controller creates an Oath of Secrets/);
 assert.match(text,/or \[Its controller creates an Oath of Valor/);
});
test('coordinated subject continuation retains the chosen creature across Aura creation',()=>{
 const text=expand('Create a 2/2 black Zombie creature token. That token seeks secrets and gains haste until end of turn.');
 assert.match(text,/The creature chosen this way gains haste until end of turn/);
 assert.doesNotMatch(text,/\. and gains/);
});
test('completion vocabulary translates general and typed observers without inventing a minimum amount',()=>{
 const def=defs.find(def=>def.forms);
 assert.equal(customKeywordForms(def).eventObjectPreposition,'of');
 assert.equal(expandNamedActionReferences('Whenever you fulfill an Oath, draw a card.',[def]),'Whenever you perform the fulfill oath action, draw a card.');
 assert.equal(expandNamedActionReferences('Whenever you fulfill an Oath of a Knight or Soldier, draw a card.',[def]),'Whenever you perform the fulfill oath action with a Knight or Soldier, draw a card.');
});
test('all exact operative sources and original supplied token text remain evidence',()=>{
 assert.equal(row.representative_cards.length,21);
 assert.equal(new Set(row.representative_cards.map(card=>card.source_card_key)).size,21);
 for(const card of row.representative_cards){assert.equal(card.original_text,card.source_faces[0].original_text);for(const token of card.custom_tokens){assert.match(token.face.oracleText,/fulfill this Oath/);assert.match(token.input.oracleText,/perform the fulfill oath action with enchanted creature/i);assert.ok(token.input.subtypes.includes('Oath'));assert.ok(!token.input.subtypes.includes('Role'));}}
});

test('a subtype-only target remains a native typed selector',()=>{
 const text=expand('Target Knight or Soldier seeks secrets.');
 assert.match(text,/Choose Target Knight or Soldier\. Its controller creates an Oath of Secrets token/i);
 assert.doesNotMatch(text,/seeks secrets/i);
});
