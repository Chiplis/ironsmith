import test from 'node:test';
import assert from 'node:assert/strict';
import {abilityActionState,abilityActionReason,abilityHitRegions} from '../src/lib/ability-action-state.js';
test('availability distinguishes unchecked, unpaid, illegal and selected abilities',()=>{
 assert.equal(abilityActionState({payment_pending:true,mana_payment_available:false}),'pending');
 for(const action of [{mana_payment_available:false},{legal:false},{available:false}])assert.equal(abilityActionState(action),'unavailable');
 assert.equal(abilityActionState({mana_payment_available:true}),'available');
 assert.equal(abilityActionState({mana_payment_available:null}),'pending');
 assert.equal(abilityActionState({},true),'selected');
 assert.match(abilityActionReason({unavailable_reason:'No legal targets.'}),/No legal targets/);
 assert.match(abilityActionReason({mana_payment_available:false}),/Cannot pay/);
});
test('wrapped abilities become one full-width target while mana choices stay distinct',()=>{
 const action={id:1};
 const regions=abilityHitRegions([{lineIndex:1,angle:0,left:40,top:300,width:200,height:20,actions:[action]},{lineIndex:1,angle:0,left:40,top:320,width:80,height:20,actions:[action]},{lineIndex:2,option:'{W}',angle:0,left:100,top:360,width:15,height:20,actions:[{id:2}]},{lineIndex:2,option:'{U}',angle:0,left:150,top:360,width:15,height:20,actions:[{id:3}]}]);
 assert.equal(regions.length,3);assert.equal(regions[0].width,428);assert.equal(regions[0].height,46);
 assert.equal(regions[1].left,100);assert.equal(regions[2].left,150);
});
