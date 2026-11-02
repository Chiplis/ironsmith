import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput} from '../src/lib/custom-mechanics.js';
const reminder="As you cast an instant or sorcery spell, you may exile this card from your graveyard and pay {Cost}. If you do, add this card's effects to that spell. You may exile only one card this way per spell.";
const definition=()=>validateMechanic({keyword:'Reprise {Cost}',reminder,paymentKind:'text-addition',textAdditionVerb:'reprising',forms:{variants:['reprising']}});
test('postfix text-addition relation retains the exact originating source operand',()=>{
 const result=compileDraftInput({faces:[{name:"Roggar's Gambit",oracleText:"Reprise {1}{R}\nDamage dealt by sources reprising Roggar's Gambit can't be prevented."}]},[definition()]);
 assert.match(result.faces[0].oracleText,/sources with text added from Roggar's Gambit can't be prevented/);
 assert.doesNotMatch(result.faces[0].oracleText,/sources using reprise/);
});
test('an unrelated named card or quoted text is not rebound as the original source',()=>{
 const oracleText="Reprise {1}{R}\nDamage dealt by sources reprising Another Card can't be prevented.\nCreate a token with \"Damage dealt by sources reprising Roggar's Gambit can't be prevented.\"";
 const result=compileDraftInput({faces:[{name:"Roggar's Gambit",oracleText}]},[definition()]);
 assert.match(result.faces[0].oracleText,/sources reprising Another Card/);
 assert.match(result.faces[0].oracleText,/"Damage dealt by sources reprising Roggar's Gambit can't be prevented\."/);
});
test('lineage verb configuration requires an explicitly typed custom text-addition payment',()=>{
 assert.throws(()=>validateMechanic({keyword:'Named action {Cost}',reminder,textAdditionVerb:'reprisING'}),/custom text-addition payment/);
});
