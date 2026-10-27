import test from 'node:test';
import assert from 'node:assert/strict';
import {splitLoyaltyAbility} from '../src/lib/loyalty-ability.js';
test('recognizes loyalty costs independently of the permanent type',()=>{
 for(const [input,cost,direction] of [['+1: Draw a card.','+1','up'],['0: Draw a card.','0','zero'],['−2: Draw a card.','−2','down'],['-X: Draw a card.','−X','down'],['[+2]: Draw a card.','+2','up']])
  assert.deepEqual(splitLoyaltyAbility(input),{cost,text:'Draw a card.',direction});
 for(const line of ['{1}: Draw a card.','{T}: Add {G}.','Protection from Humans','Whenever a creature dies: draw a card.'])assert.equal(splitLoyaltyAbility(line),null);
});
