import test from 'node:test';
import assert from 'node:assert/strict';
import { createPriorityPolicy, abilityKey, booleanChoice } from '../src/lib/priority-policy.js';
import { createPriorityStops } from '../src/lib/priority-stops.js';
const pass = { kind: 'pass_priority', index: 0, action_ref: { kind: 'pass_priority' } };
const base = (extra = {}) => ({ perspective: 0, active_player: 0, turn_number: 1, phase: 'FirstMain', step: null,
 stack_size: 0, stack_objects: [], decision: { kind: 'priority', player: 0, actions: [pass], can_act: true }, ...extra });
function setup() { const stops = createPriorityStops({ resolveMode: 'manual' }); return { stops, policy: createPriorityPolicy({ stops }) }; }
const entry = (extra = {}) => ({ id: 10, controller: 1, ability_kind: 'Triggered', source_stable_id: 25,
 inspect_object_id: 6, name: 'Soul Warden', automation_identity: 'trigger:123', ...extra });
const stack = (top = entry()) => base({ stack_size: 1, stack_objects: [top] });
test('normal stops wait without a timer; off steps pass; opponent additions stop', () => {
 const { policy, stops } = setup(); assert.equal(policy.evaluate(base()), null); assert.equal(stops.getSnapshot().window, null);
 assert.ok(policy.evaluate(base({ phase: 'Beginning', step: 'Upkeep' }))); assert.equal(policy.evaluate(stack()), null);
 assert.ok(policy.evaluate(stack(entry({ controller: 0 }))));
});
test('passing releases a stop; casting does not consume main phase', () => {
 const { policy, stops } = setup(); stops.beforeCommand({ type: 'priority_action', action_ref: { kind: 'cast_spell' } }, base());
 assert.equal(policy.evaluate(base()), null); stops.beforeCommand({ type: 'priority_action', action_ref: pass.action_ref }, base()); assert.ok(policy.evaluate(base()));
});
test('response yield pauses for opposition and cancels on a game action', () => {
 const { policy } = setup(); policy.start('responses', base()); assert.ok(policy.evaluate(base())); assert.equal(policy.evaluate(stack()), null);
 policy.beforeCommand({ type: 'priority_action', action_ref: { kind: 'cast_spell' } }, stack()); assert.equal(policy.status(base()).yield, null);
});
for (const mode of ['turn','responses','until','no_play']) test(`${mode} never chooses combat, targets, discards or payment`, () => {
 const { policy } = setup(); policy.start(mode, base(), { key: 'phase:End', player: 0 });
 for (const kind of ['attackers','blockers','select_objects','targets','mana_payment','select_options']) assert.equal(policy.evaluate(base({ decision: { kind, player: 0 } })), null);
});
test('through-turn resumes after required choices and expires next turn', () => {
 const { policy } = setup(); policy.start('turn', base()); assert.ok(policy.evaluate(stack()));
 assert.equal(policy.evaluate(base({ decision: { kind: 'attackers', player: 0 } })), null); assert.ok(policy.evaluate(base())); assert.equal(policy.evaluate(base({ turn_number: 2 })), null);
});
test('no-play requires known legal analysis and persists across turns', () => {
 const { policy } = setup(); policy.start('no_play', base());
 for (const can_act of [true, null]) assert.equal(policy.evaluate(base({ decision: { kind: 'priority', player: 0, actions: [pass], can_act } })), null);
 assert.ok(policy.evaluate(base(), false)); assert.ok(policy.evaluate(base({ turn_number: 2 }), false));
});
test('future stop is seat-specific and overrides an off preference', () => {
 const { policy } = setup(); policy.start('until', base(), { player: 1, key: 'phase:End' });
 assert.ok(policy.evaluate(base({ phase: 'Ending', step: 'End' })));
 assert.equal(policy.evaluate(base({ turn_number: 2, active_player: 1, phase: 'Ending', step: 'End' })), null);
});
test('current-phase target waits for next occurrence rather than stack changes', () => {
 const { policy } = setup(); policy.start('until', base(), { player: 0, key: 'phase:Main' }); assert.ok(policy.evaluate(stack()));
 policy.observe(base({ phase: 'Combat', step: 'BeginCombat' })); assert.equal(policy.evaluate(base({ turn_number: 3 })), null);
});
test('Ctrl hold survives payment and stops when the action reaches the stack', () => {
 const { policy } = setup(); policy.beforeCommand({ type: 'priority_action', action_ref: { kind: 'cast_spell' } }, base(), { maintain: true });
 assert.equal(policy.evaluate(base({ decision: { kind: 'mana_payment', player: 0 } })), null);
 policy.beforeCommand({ type: 'mana_payment' }, base()); const state = stack(entry({ controller: 0 })); assert.equal(policy.evaluate(state), null);
 policy.beforeCommand({ type: 'priority_action', action_ref: pass.action_ref }, state); assert.ok(policy.evaluate(state));
});
test('clear stops immediately, clears preferences, and preserves phase stops', () => {
 const { policy, stops } = setup(); stops.setStop('phase:End','always',base(),1); policy.start('turn',base());
 policy.setRule(entry(),'source',{ yield: true, answer: 'yes' },base()); policy.setGroupTriggers(true,base()); policy.clear(base());
 assert.equal(policy.evaluate(base()),null); assert.equal(policy.status(base()).rules.length,0); assert.equal(policy.status(base()).groupTriggers,false);
 assert.equal(stops.getSnapshot().stops['player:1:phase:End'],'always');
});
test('source and name preferences use structural identities and expire when requested', () => {
 const { policy } = setup(); policy.setRule(entry(),'source',{ yield: true },base()); assert.ok(policy.evaluate(stack()));
 assert.equal(policy.evaluate(stack(entry({ automation_identity:'trigger:999' }))),null);
 assert.equal(policy.evaluate(stack(entry({ source_stable_id:26 }))),null);
 assert.equal(abilityKey(entry({ automation_identity:null })),null);
 policy.setRule(entry(),'name',{ yield:true },base(),true); assert.ok(policy.evaluate(stack(entry({ source_stable_id:26 }))));
 assert.equal(policy.evaluate({ ...stack(entry({ source_stable_id:26 })),turn_number:2 }),null);
});
test('yes/no uses native indices and only answers Boolean choices of the resolving source', () => {
 const { policy } = setup(); const decision = { kind:'select_options',choice_kind:'boolean',player:0,source_id:6,options:[{index:1,legal:true},{index:0,legal:true}] };
 const state=base({decision,resolving_stack_object:entry()}); policy.setRule(entry(),'source',{answer:'yes'},state);
 assert.deepEqual(policy.evaluate(state).option_indices,[1]); assert.equal(policy.evaluate({...state,decision:{...decision,source_id:7}}),null);
 assert.equal(policy.evaluate({...state,decision:{...decision,choice_kind:'modes'}}),null);
 assert.equal(booleanChoice({...decision,options:[{index:1,legal:false}]},true),null); assert.deepEqual(booleanChoice(decision,false).option_indices,[0]);
});
test('identical target-free trigger ordering is opt-in and mixed groups remain manual', () => {
 const {policy}=setup(); const state=base({decision:{kind:'select_options',choice_kind:'trigger_order',player:0,
 options:[{index:2,automation_identity:'trigger:1'},{index:4,automation_identity:'trigger:1'}]}});
 assert.equal(policy.evaluate(state),null); policy.setGroupTriggers(true,state); assert.deepEqual(policy.evaluate(state).option_indices,[2,4]);
 state.decision.options[1].automation_identity=null; assert.equal(policy.evaluate(state),null);
});
test('full control wins; remote seats and loop shortcuts cannot be automated', () => {
 const {policy}=setup(); policy.start('turn',base()); policy.setFullControl(true,base()); assert.equal(policy.evaluate(base()),null);
 policy.setFullControl(false,base()); assert.equal(policy.evaluate(base({decision:{kind:'priority',player:1,actions:[pass]}})),null);
 assert.equal(policy.evaluate(base({loop_shortcut:{}})),null);
});

test('mixed trigger groups stay manually ordered but duplicates move together', async () => {
 const {groupedTriggerOrder}=await import('../src/lib/priority-policy.js');
 const decision={choice_kind:'trigger_order',options:[{index:0,automation_identity:'a'},{index:1,automation_identity:'b'},{index:2,automation_identity:'a'},{index:3}]};
 assert.deepEqual(groupedTriggerOrder(decision,[0,1,2,3]),[0,2,1,3]);
});

test('no-play ignores mana abilities alone but stops for payable spells and non-mana abilities',()=>{
 const {policy}=setup();policy.start('no_play',base());
 const state=base({decision:{kind:'priority',player:0,can_act:false,analysis_complete:true,
 actions:[pass,{kind:'activate_mana_ability',index:1,payment_proven:true}]}});
 assert.ok(policy.evaluate(state));
 for (const kind of ['cast_spell', 'activate_ability']) {
  state.decision.actions=[pass,{kind:'activate_mana_ability',index:1},{kind,index:2,payment_proven:true}];
  state.decision.can_act=true;
  assert.equal(policy.evaluate(state),null);
 }
 state.decision.can_act=false;
 state.decision.actions=[pass];state.decision.analysis_complete=false;
 assert.equal(policy.evaluate(state),null);
});

test('touch-accessible hold-next-action waits for an actual cast rather than holding immediately',()=>{
 const {policy}=setup(); policy.holdNextAction(true,base());
 assert.ok(policy.evaluate(base({phase:'Beginning',step:'Upkeep'})));
 policy.beforeCommand({type:'priority_action',action_ref:{kind:'cast_spell'}},base());
 assert.equal(policy.evaluate(stack(entry({controller:0}))),null);
 assert.equal(policy.status(base()).holdNextAction,false);
});
test('a new game clears match-scoped preferences',()=>{
 const {policy}=setup(); policy.observe(base({turn_number:9}));policy.setRule(entry(),'source',{yield:true},base({turn_number:9}));
 assert.equal(policy.status(base()).rules.length,0);
});
test('manual mixed-group movement preserves the order within each group',async()=>{
 const {moveTriggerGroup}=await import('../src/lib/priority-policy.js');
 const decision={choice_kind:'trigger_order',options:[{index:0,automation_identity:'a'},{index:1,automation_identity:'b'},{index:2,automation_identity:'a'}]};
 assert.deepEqual(moveTriggerGroup(decision,[0,2,1],0,1),[1,0,2]);
});

test('hold-next-action survives mulligans, land plays and mana production',()=>{
 const {policy}=setup();policy.holdNextAction(true,base());
 for(const kind of ['keep_opening_hand','play_land','activate_mana_ability']) {
  policy.beforeCommand({type:'priority_action',action_ref:{kind}},base());
  assert.equal(policy.status(base()).holdNextAction,true);
 }
});

test('next-end-step yield reaches the next player when requested during cleanup',()=>{
 const {policy}=setup();policy.start('until',base({phase:'Ending',step:'Cleanup'}),{key:'phase:End',anyPlayer:true});
 assert.equal(policy.evaluate(base({phase:'Ending',step:'End',active_player:1,turn_number:2})),null);
});
test('temporary ability yields do not shorten a permanent optional-answer preference',()=>{
 const {policy}=setup();policy.setRule(entry(),'source',{answer:'yes'},base());
 policy.setRule(entry(),'source',{yield:true},base(),true);
 const rules=policy.status(base({turn_number:2})).rules;
 assert.equal(rules.length,1);assert.equal(rules[0].answer,'yes');
});
