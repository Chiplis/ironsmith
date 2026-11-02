import test from 'node:test';
import assert from 'node:assert/strict';
import {validateMechanic,compileDraftInput,definitionPayment} from '../src/lib/custom-mechanics.js';
const reminder="As you cast an instant or sorcery spell, you may exile this card from your graveyard and pay {Cost}. If you do, add this card's effects to that spell. You may exile only one card this way per spell.";
test('explicit text-addition payment metadata survives validation and direct expansion',()=>{
 const def=validateMechanic({keyword:'Reprise {Cost}',reminder,paymentKind:'text-addition'});
 assert.equal(def.paymentKind,'text-addition');
 const result=compileDraftInput({faces:[{name:'Text source',oracleText:'Draw a card.\nReprise {1}{U}'}]},[def]);
 assert.deepEqual(result.faces[0].customKeywordPayments,['reprise']);
 assert.match(result.faces[0].oracleText,/exile this card from your graveyard and pay \{1\}\{U\}/);
});
test('ordinary prose does not implicitly acquire text-addition payment attribution',()=>{
 const def=validateMechanic({keyword:'Reprise {Cost}',reminder});
 assert.equal(definitionPayment(reminder,def),null);
});
test('unknown or operand-free explicit text payment metadata is rejected',()=>{
 assert.throws(()=>validateMechanic({keyword:'Reprise {Cost}',reminder,paymentKind:'invented'}),/Unknown custom payment kind/);
 assert.throws(()=>validateMechanic({keyword:'Memory',reminder:'Draw a card.',paymentKind:'text-addition'}),/explicit \{Cost\} operand/);
});

test('explicit progressive form binds reprising-spell references to payment identity',()=>{
 const def=validateMechanic({keyword:'Reprise {Cost}',reminder,paymentKind:'text-addition',forms:{variants:['reprising']}});
 const result=compileDraftInput({faces:[{name:'Grant observer',oracleText:"Reprising spells you control can't be countered."}]},[def]);
 assert.match(result.faces[0].oracleText,/spells you control using reprise|spells using reprise you control/i);
 assert.equal(result.faces[0].customKeywordPayments,undefined,'an observer does not itself pay the text-addition cost');
});
