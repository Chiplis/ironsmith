import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const definition=()=>validateMechanic({keyword:'Collect support {Number}',costProgram:true,paymentReferences:[{subject:'support',participle:'collected'}],reminder:'Choose {Number} untapped creatures you control or cards from your graveyard, each representing a different color it has. Tap the chosen creatures on the battlefield. Exile the chosen cards from your graveyard.'});
const expand=text=>compileDraftInput({faces:[{name:'Contributor',oracleText:text}]},[definition()]).faces[0];
test('a declared cost program stays one bounded activation price beside other costs',()=>{
 const source='{R}, Collect support 2, Pay 1 life: Draw a card.';const face=expand(source);
 assert.match(face.oracleText,/\{R\}, Pay the cost "Choose 2 .*; Tap .*; Exile .*", Pay 1 life: Draw a card\./);
 assert.equal(face.displayOracleText,source);assert.ok(face.customKeywordIdentities.includes('collect support'));
});
test('optional additional price produces a retained receipt reference',()=>{
 const face=expand('As an additional cost to cast this spell, you may collect support 3.\nIf support was collected, draw a card.');
 assert.match(face.oracleText,/As an additional cost to cast this spell, you may Pay the cost "Choose 3 /);
 assert.match(face.oracleText,/If this spell's additional cost was paid, draw a card\./);
});
test('resolving declarations retain the whole optional payment owner and X stays authored',()=>{
 assert.match(expand('Whenever this creature attacks, you may collect support 2.').oracleText,/you may Pay the cost "Choose 2 /);
 assert.match(expand('Collect support X: Draw a card.').oracleText,/Pay the cost "Choose X /);
 assert.match(expand('You may collect support X. If you do, scry X.').oracleText,/Choose any number of untapped/);
 assert.match(expand('You may collect support X. If you do, scry X.').oracleText,/With X equal to the number of objects chosen this way, do the following: If you do, scry X/);
 assert.match(expand('As an additional cost to cast this spell, collect support X.').oracleText,/Pay the cost "Choose X /);
});
test('cost programs reject unbounded or quoted bodies',()=>{
 assert.throws(()=>validateMechanic({keyword:'Gather {Number}',costProgram:true,reminder:'Create {Number} tokens with "Flying".'}),/unquoted/);
});

test('declared named payment receipt wraps the whole resolving cost without consuming its result',()=>{
 const named=validateMechanic({...definition(),keyword:'Collect support {Number}',forms:{eventVerb:'collect support',eventLabel:'collect support'}});
 const face=compileDraftInput({faces:[{name:'Contributor',oracleText:'You may collect support X. If you do, scry X.'}]},[named]).faces[0];
 assert.match(face.oracleText,/Perform the collect support action with objects chosen this way if the following cost is paid: You may pay the cost/);
 assert.match(face.oracleText,/With X equal to the number of objects chosen this way/);
});

test('declared receipt unit binds the source watcher to completed count',()=>{
 const named=validateMechanic({...definition(),keyword:'Collect support {Number}',forms:{eventVerb:'collect support',eventLabel:'collect support'},paymentReferences:[{subject:'support',participle:'collected',countUnit:'color'}]});
 const text='At the beginning of your end step, you may collect support X.\nWhenever you collect support from one or more colors, draw that number of cards.';
 const face=compileDraftInput({faces:[{name:'Contributor',oracleText:text}]},[named]).faces[0];
 assert.match(face.oracleText,/Whenever you perform the collect support action with an amount of 1 or more, draw that many cards/);
 assert.equal(face.displayOracleText,text);
});
