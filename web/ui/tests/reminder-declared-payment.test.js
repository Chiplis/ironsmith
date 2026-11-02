import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput,mergeMechanics} from '../src/lib/custom-mechanics.js';
const template='a non{Name} land you control';
const effect='Choose a land you control not named {Name}. If you do, that land becomes a basic land named Wastes with "{T}: Add {C}" and loses all other supertypes, card types, subtypes, and abilities.';
const payment='Have a land you control not named {Name} become a basic land named Wastes with "{T}: Add {C}" and loses all other supertypes, card types, subtypes, and abilities.';
const definition=()=>validateMechanic({keyword:'Waste',template,reminder:effect,paymentReminder:payment});
test('declared payment reminder binds the same operand in an activation cost',()=>{
 const original='{T}, Waste a nonwastes land you control: Add one mana of any color.';
 const face=compileDraftInput({faces:[{name:'Channeler',oracleText:original}]},[definition()]).faces[0];
 assert.match(face.oracleText,/\{T\}, Have a land you control not named wastes become a basic land named Wastes/);
 assert.match(face.oracleText,/abilities: Add one mana of any color\.$/);
 assert.deepEqual(face.customKeywordIdentities,['waste']);
 assert.equal(face.displayOracleText,original);
});
test('resolving actions keep their effect meaning while costs require an explicit payment body',()=>{
 const def=definition();
 const action='When this creature enters, waste a nonwastes land you control.';
 const expanded=compileDraftInput({faces:[{name:'Remnant',oracleText:action}]},[def]).faces[0];
 assert.match(expanded.oracleText,/Choose a land you control not named wastes\./);
 assert.doesNotMatch(expanded.oracleText,/Have a land/);
 const bare=validateMechanic({keyword:'Waste',template,reminder:effect});
 const cost='{T}, Waste a nonwastes land you control: Add {G}.';
 assert.equal(compileDraftInput({faces:[{name:'Unspecified payment',oracleText:cost}]},[bare]).faces[0].oracleText,cost);
});
test('payment readers are independent of the action spelling and result name',()=>{
 const def=validateMechanic({keyword:'Forge',template:'a creature named {Name}',reminder:'Choose a creature named {Name}. It becomes an artifact.',paymentReminder:'Have a creature named {Name} become an artifact.'});
 const original='Forge a creature named Apprentice: Draw a card.';
 assert.equal(compileDraftInput({faces:[{name:'Smith',oracleText:original}]},[def]).faces[0].oracleText,'Have a creature named Apprentice become an artifact: Draw a card.');
});
test('invalid payment operands and contradictory payment definitions are rejected',()=>{
 assert.throws(()=>validateMechanic({keyword:'Waste',template,reminder:effect,paymentReminder:'Have a land become a basic land.'}),/same operands/);
 assert.throws(()=>mergeMechanics([definition()],[{...definition(),paymentReminder:payment.replace('basic land','legendary artifact')}]),/could not be added/);
 const quoted='The name is "Waste a nonwastes land you control: Add {C}."';
 assert.equal(compileDraftInput({faces:[{name:'Label',oracleText:quoted}]},[definition()]).faces[0].oracleText,quoted);
});
