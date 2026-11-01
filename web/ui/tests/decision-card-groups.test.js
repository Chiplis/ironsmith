import test from 'node:test';
import assert from 'node:assert/strict';
import {groupDecisionCards, nextGroupedCardChoice} from '../src/lib/decision-card-groups.js';
const card=(id,extra={})=>({id,stable_id:id,name:'Forest',oracle_text:'{T}: Add {G}.',...extra});
const candidates=[1,2,3].map(id=>({id,name:'Forest',legal:true}));
const state={players:[{id:0,hand_cards:[card(1),card(2)]},{id:1,hand_cards:[card(3)]}]};
test('groups copies only in the same hand and uses real object IDs',()=>{
 const groups=groupDecisionCards(candidates,state);
 assert.deepEqual(groups.map(g=>g.members.map(c=>c.id)),[[1,2],[3]]);
 assert.equal(nextGroupedCardChoice(groups[0].members,[],2),1);
 assert.equal(nextGroupedCardChoice(groups[0].members,[1],2),2);
 assert.equal(nextGroupedCardChoice(groups[0].members,[1,2],2),2);
});
test('library positions do not split copies, but different libraries and zones do',()=>{
 const s={players:[{id:0,hand_cards:[card(3)]}],viewed_cards:{zone:'Library',subject:0,cards:[card(1,{position:2}),card(2,{position:5})]}};
 assert.deepEqual(groupDecisionCards(candidates,s).map(g=>g.members.length),[2,1]);
 const owned=candidates.map((c,i)=>({...c,zone:'Library',object_owner:i===1?1:0}));
 assert.deepEqual(groupDecisionCards(owned,s).map(g=>g.members.map(c=>c.id)),[[1,3],[2]]);
});
test('modified, hidden, illegal, unknown and non-hand/library cards stay separate',()=>{
 for(const extra of [{oracle_text:'Modified'},{perpetual:{power:1}},{face_down:true}]) {
  assert.equal(groupDecisionCards(candidates,{players:[{id:0,hand_cards:[card(1),card(2,extra),card(3)]}]}).length,2);
 }
 assert.equal(groupDecisionCards(candidates,{}).length,3);
 for(const zone of ['exile','graveyard','battlefield']) {
  const s={players:[{id:0,[zone==='battlefield'?zone:`${zone}_cards`]:[card(1),card(2),card(3)]}]};
  assert.equal(groupDecisionCards(candidates,s).length,3);
 }
 assert.equal(groupDecisionCards(candidates.map(c=>({...c,legal:c.id!==2})),state).length,3);
});
test('alternative selection requirements prevent grouping non-interchangeable candidates',()=>{
 const decision={candidates,selection_alternatives:[{min:1,max:1,candidate_indices:[0]},{min:2,max:2,candidate_indices:[1,2]}]};
 assert.equal(groupDecisionCards(candidates,state,decision).length,3);
});
