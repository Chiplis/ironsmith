import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {compileDraftInput,validateMechanic,draftMechanics} from '../../web/ui/src/lib/custom-mechanics.js';
import {materializeCard} from './cohort-input.mjs';
const fixture=JSON.parse(readFileSync(new URL('../../fixtures/custom_mechanics/msem-module.json',import.meta.url),'utf8'));
const rawDefinition=fixture.mechanics[0].executable_definitions[0];
const definition=validateMechanic(rawDefinition);
const instruction='You may cast an enchantment spell from your hand without paying its mana cost or activate a submodule of a permanent you control without paying its activation cost.';
const compile=(text,definitions=[definition])=>compileDraftInput({faces:[{name:'Declared instruction',typeLine:'Creature — God',oracleText:text}]},definitions).faces[0];
test('a consumer uses the complete supplied submodule declaration without gaining its layout',()=>{
 const face=compile(instruction);
 assert.equal(face.oracleText,'You may cast an enchantment spell from your hand without paying its mana cost or activate an activated ability with designation module of a permanent you control without paying its costs during module actions.');
 assert.deepEqual(face.customKeywordIdentities||[],[]);
 assert.doesNotMatch(face.oracleText,/When this permanent enters|Once group|This activated ability has designation/);
 assert.equal(draftMechanics({faces:[{name:'Consumer',oracleText:instruction}]},[definition]).length,1);
});
test('a different declared family stays data while names and absent declarations stay opaque',()=>{
 const consoleDefinition=validateMechanic({...rawDefinition,keyword:'Console',appliesToSubtype:'Console'});
 assert.match(compile(instruction,[consoleDefinition]).oracleText,/designation console of a permanent you control without paying its costs during console actions/);
 assert.equal(compile(instruction,[]).oracleText,instruction);
 const literal='Search your library for a card named "activate a submodule of a permanent you control without paying its activation cost".';
 assert.equal(compile(literal).oracleText,literal);
 assert.throws(()=>compile(instruction,[validateMechanic({...rawDefinition,reminder:'Flavor-word bars name submodules. This layout has three submodule bars.'})]),/preserve every source cost/);
});
test('the complete actual Pyp source retains its independent draw replacement and raw evidence',()=>{
 const card=fixture.mechanics[0].representative_cards.find(card=>card.source_card_key==='Pyp, the Indexer_BTB');assert.ok(card,'exact missing consumer source must be preserved');
 const original=JSON.stringify(card);const [face]=materializeCard(card,[definition],'msem');
 const replacement=card.source_record.rulesText.split('\n')[0];assert.ok(face.executable_text.includes(replacement),'draw replacement sibling must remain verbatim');
 assert.match(face.executable_text,/\{3}, \{T}: You may cast an enchantment spell from your hand without paying its mana cost or activate an activated ability with designation module of a permanent you control without paying its costs during module actions\./);
 assert.doesNotMatch(face.executable_text,/When this permanent enters the battlefield|Once group/);
 assert.equal(JSON.stringify(card),original,'no rawsource mutation');
});
