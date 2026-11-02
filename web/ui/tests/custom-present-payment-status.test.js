import test from 'node:test';
import assert from 'node:assert/strict';
import {compileDraftInput,validateMechanic,mergeMechanics} from '../src/lib/custom-mechanics.js';
const raw={keyword:'Echo price',forms:{participle:'echo priced'},template:'{Cost}',reminder:'As an additional cost to cast this spell, you may pay {Cost}. When you cast this spell, if you paid this additional cost, copy this spell and you may choose new targets for the copy.'};
const payment=validateMechanic(raw);
const status=validateMechanic({...raw,statusPredicate:{spellCopy:true,customKeyword:'echo price'}});
const expand=(definition,name,text)=>compileDraftInput({faces:[{name,oracleText:text}]},[definition]).faces[0].oracleText;
test('only an explicit status definition gives present or past participles a copy predicate',()=>{
 for(const verb of ['is','was','is not',"isn't",'was not']) {
  const text=expand(status,'Arbitrary Source',`Echo price {2}\nIf Arbitrary Source ${verb} echo priced, draw a card.`);
  assert.ok(text.includes(`If this spell is ${['is','was'].includes(verb)?'':'not '}a copy with custom keyword "echo price", draw a card.`));
  assert.doesNotMatch(text,/If Its additional cost/);
 }
 assert.match(expand(payment,'A','Echo price {2}\nIf it is echo priced, draw a card.'),/If it is echo priced/,'no present-tense payment inference');
 assert.match(expand(payment,'A','Echo price {2}\nIf it was echo priced, draw a card.'),/If its additional cost was paid/,'legacy explicit payment participle preserved');
});
test('source predicates keep quoted references and longer object names opaque',()=>{
 const text=expand(status,'Experiment','Echo price {2}\nIf Other Experiment is echo priced, draw a card.\nIf Experiment is echo priced, gain 1 life.\nCreate a token with "If it is echo priced, draw a card."');
 assert.match(text,/If Other Experiment is echo priced/);assert.match(text,/If this spell is a copy with custom keyword "echo price", gain 1 life/);
 assert.match(text,/"If it is echo priced, draw a card\."/);assert.doesNotMatch(text,/Other this spell/);
});
test('copy status vocabulary survives normalization and conflicting definitions are rejected',()=>{
 assert.deepEqual(status.statusPredicate,{spellCopy:true,customKeyword:'echo price'});
 assert.throws(()=>validateMechanic({...raw,statusPredicate:{spellCopy:false,customKeyword:'echo price'}}),/status predicate/);
 assert.throws(()=>mergeMechanics([status],[{...raw,statusPredicate:{spellCopy:true,customKeyword:'other identity'}}]),/could not be added/);
});

test('opaque status pronouns never become a predicate on the source spell',()=>{
 for (const verb of ['is','was','is not',"wasn't"]) {
  const text=expand(status,'Experiment',`Echo price {2}\nChoose target spell. If it ${verb} echo priced, draw a card.`);
  assert.ok(text.includes(`If it ${verb} echo priced, draw a card.`));
  assert.doesNotMatch(text,/If this spell is/);
 }
 for (const subject of ['this spell','this card']) {
  assert.match(expand(status,'Experiment',`Echo price {2}\nIf ${subject} is echo priced, draw a card.`),/If this spell is a copy with custom keyword "echo price"/);
 }
});
