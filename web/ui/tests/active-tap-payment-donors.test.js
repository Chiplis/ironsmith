import test from 'node:test';
import assert from 'node:assert/strict';
import {expandTapPaymentReferences} from '../src/lib/reminder-tap-payment-references.js';
const reminder="As an additional cost to cast this spell, you may tap an untapped creature you control. This spell costs {X} less to cast, where X is the power of the creature tapped this way.";
for(const keyword of ['Draw strength','Power price']) {
 const definition={keyword,sourceKeyword:'Harness',reminder};
 test(`${keyword}: active source donor and negative donor are receipt operands`,()=>{
  const positive=expandTapPaymentReferences("If this spell was harnessed, also put a +1/+1 counter on the creature that harnessed it.",[definition]);
  assert.equal(positive,`if this spell paid the additional cost named ${keyword}, also put a +1/+1 counter on the creature tapped to pay its ${keyword} additional cost.`);
  assert.equal(expandTapPaymentReferences("Each creature without flying that didn't harness it.",[definition]),`Each creature without flying that was not tapped to pay its ${keyword} additional cost.`);
  const literal='Create a token named "the creature that harnessed it".';
  assert.equal(expandTapPaymentReferences(literal,[definition]),literal);
 });
}
test('a matching word without typed paid-tap definition is not an operation receipt',()=>{
 assert.equal(expandTapPaymentReferences("the creature that harnessed it",[{keyword:'Draw strength',sourceKeyword:'Harness',reminder:'Draw a card.'}]),'the creature that harnessed it');
});
