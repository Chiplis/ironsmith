import test from 'node:test';
import assert from 'node:assert/strict';
import {createPriorityPolicy} from '../src/lib/priority-policy.js';
import {resolvableTriggerGroup} from '../src/lib/resolve-group.js';
const trigger=(id,source=id)=>({id,source_stable_id:source,inspect_object_id:source,controller:0,ability_kind:'Triggered',automation_identity:'trigger:surveil'});
const initial=()=>({perspective:0,turn_number:1,stack_size:4,stack_objects:[trigger(4,40),trigger(3,30),trigger(2,20),trigger(1,10)],decision:{kind:'priority',player:0,actions:[{index:0,kind:'pass_priority',action_ref:{kind:'pass_priority'}}]}});
function prompt(id,card=100,extra={}){return {...initial(),stack_objects:initial().stack_objects.filter(x=>x.id<id),resolving_stack_object:trigger(id,id*10),decision:{kind:'select_objects',player:0,source_id:id*10,source_name:`Source ${id}`,description:'Surveil 1',min:0,max:1,candidates:[{id:card,name:'Island',legal:true}],...extra},viewed_cards:{source:id*10,card_ids:[card],cards:[{id:card,name:'Island'}]}};}
function begin(){const p=createPriorityPolicy();assert.equal(p.startGroup(initial()),true);return p;}
test('groups matching triggers from different sources, not unrelated abilities or noncontiguous entries',()=>{
 assert.equal(resolvableTriggerGroup(initial()).entries.length,4);
 const s=initial();s.stack_objects[1].automation_identity='other';assert.equal(resolvableTriggerGroup(s),null);
 s.stack_objects=initial().stack_objects.map(x=>({...x,ability_kind:'Activated'}));assert.equal(resolvableTriggerGroup(s),null);
});
test('surveil keep repeats across sources; mill pauses on the next card then newly repeatable choices cascade',()=>{
 const p=begin();assert.ok(p.evaluate(initial()));assert.equal(p.evaluate(prompt(4)),null);
 p.beforeCommand({type:'select_objects',object_ids:[]},prompt(4));
 assert.deepEqual(p.evaluate(prompt(3)),{type:'select_objects',object_ids:[]});
 // User changes their mind and mills this card. A different Island is still a different choice.
 p.beforeCommand({type:'select_objects',object_ids:[100]},prompt(3));
 assert.equal(p.evaluate(prompt(2,101)),null);
 p.beforeCommand({type:'select_objects',object_ids:[]},prompt(2,101));
 assert.deepEqual(p.evaluate(prompt(1,101)),{type:'select_objects',object_ids:[]});
});
test('exact matching includes legality, limits, viewed identity, option order and decision stage',()=>{
 const p=begin();p.evaluate(prompt(4));p.beforeCommand({type:'select_objects',object_ids:[]},prompt(4));
 for(const extra of [{max:2},{candidates:[{id:100,name:'Island',legal:false}]},{description:'Choose cards to exile'}])assert.equal(p.evaluate(prompt(3,100,extra)),null);
 const changed=prompt(3);changed.viewed_cards.cards[0].id=999;assert.equal(p.evaluate(changed),null);
 const opts=id=>prompt(id,100,{kind:'select_options',options:[{index:0,description:'Keep'},{index:1,description:'Mill'}]});
 const q=begin();q.evaluate(opts(4));q.beforeCommand({type:'select_options',option_indices:[0]},opts(4));
 const reordered=opts(3);reordered.decision.options.reverse();assert.equal(q.evaluate(reordered),null);
 assert.deepEqual(q.evaluate(opts(3)),{type:'select_options',option_indices:[0]});
 q.beforeAutomaticCommand({type:'select_options',option_indices:[0]},opts(3));
 assert.equal(q.evaluate(opts(3)),null,'a second identical-looking step within one trigger needs its own recorded answer');
});
test('clear, changed stack, other players, errors and completed group cannot replay stale choices',()=>{
 const p=begin();p.evaluate(prompt(4));const checkpoint=p.beforeCommand({type:'select_objects',object_ids:[]},prompt(4));p.rollback(checkpoint,prompt(4));
 assert.equal(p.evaluate(prompt(3)),null);
 p.beforeCommand({type:'select_objects',object_ids:[]},prompt(3));
 assert.equal(p.evaluate({...prompt(2),perspective:1}),null);
 p.clear(prompt(2));assert.equal(p.evaluate(prompt(1)),null);
 const q=begin();q.evaluate({...initial(),stack_objects:[{...trigger(9),automation_identity:'different'},...initial().stack_objects]});assert.equal(q.status(initial()).resolveGroup,null);
 const r=begin();r.evaluate({...initial(),stack_objects:[],stack_size:0,decision:null});assert.equal(r.status(initial()).resolveGroup,null);
});

test('an explicitly started group works under full control without changing that preference',()=>{
 const p=createPriorityPolicy();p.setFullControl(true,initial());p.startGroup(initial());assert.ok(p.evaluate(initial()));
 p.clear(initial());assert.equal(p.status(initial()).fullControl,true);assert.equal(p.evaluate(initial()),null);
});
test('payment replay requires the same mana sources and pool; hidden unknown choices never replay',()=>{
 const p=begin();
 const payment=id=>({...prompt(id),players:[{id:0,mana_pool:{blue:1},battlefield:[{id:50,tapped:false}]}],decision:{kind:'mana_payment',player:0,source_id:id*10,needed:'{U}'}});
 p.evaluate(payment(4));p.beforeCommand({type:'mana_payment',response:{kind:'pay'}},payment(4));
 assert.ok(p.evaluate(payment(3)));
 const changed=payment(3);changed.players[0].battlefield[0].tapped=true;assert.equal(p.evaluate(changed),null);
 const q=begin();const hidden=id=>prompt(id,100,{candidates:[{id:100,name:'Hidden card'}]});
 q.evaluate(hidden(4));q.beforeCommand({type:'select_objects',object_ids:[]},hidden(4));assert.equal(q.evaluate(hidden(3)),null);
});

test('multi-step resolutions replay each recorded step in order',()=>{
 const p=begin();const first=id=>prompt(id);
 const second=id=>prompt(id,100,{kind:'number',description:'Choose an amount',min:0,max:3});
 p.evaluate(first(4));p.beforeCommand({type:'select_objects',object_ids:[]},first(4));
 p.beforeCommand({type:'number_choice',value:2},second(4));
 assert.deepEqual(p.evaluate(first(3)),{type:'select_objects',object_ids:[]});
 p.beforeAutomaticCommand({type:'select_objects',object_ids:[]},first(3));
 assert.deepEqual(p.evaluate(second(3)),{type:'number_choice',value:2});
});
