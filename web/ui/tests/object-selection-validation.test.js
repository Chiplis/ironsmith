import test from 'node:test';
import assert from 'node:assert/strict';
import { isObjectSelectionValid } from '../src/lib/object-selection-validation.js';
const choice = {
  min: 1, max: 2, candidates: [{ id: 10 }, { id: 20 }, { id: 30 }],
  selection_alternatives: [
    { min: 1, max: 1, candidate_indices: [1] },
    { min: 2, max: 2, candidate_indices: [0, 1, 2] },
  ],
};
test('one qualifying card or any two cards enables Submit', () => {
  for (const ids of [[20], [10,20], [10,30], [20,30]]) assert.equal(isObjectSelectionValid(choice, ids), true);
  for (const ids of [[], [10], [30], [20,20], [10,20,30], [999]]) assert.equal(isObjectSelectionValid(choice, ids), false);
});
test('alternatives use rendered indices, including opaque IDs, and preserve count gaps', () => {
  const decision = { ...choice, max: 3, candidates: [{id:'hidden:a'}, {id:'hidden:b'}, {id:'hidden:c'}],
    selection_alternatives: [{min:1,max:1,candidate_indices:[1]}, {min:3,max:3,candidate_indices:[0,1,2]}] };
  assert.equal(isObjectSelectionValid(decision, ['hidden:b']), true);
  assert.equal(isObjectSelectionValid(decision, ['hidden:a','hidden:b']), false);
  assert.equal(isObjectSelectionValid(decision, ['hidden:a','hidden:b','hidden:c']), true);
});
test('ordinary optional and mandatory selections retain their bounds', () => {
  const ordinary = {...choice, selection_alternatives: []};
  assert.equal(isObjectSelectionValid(ordinary, [10]), true);
  assert.equal(isObjectSelectionValid(ordinary, []), false);
  assert.equal(isObjectSelectionValid({...ordinary,allow_partial_completion:true}, []), true);
  assert.equal(isObjectSelectionValid({...ordinary,candidates:[{id:10,legal:false}]}, [10]), false);
});

test('impossible payment alternatives do not make an incomplete main selection valid', () => {
  const decision={...choice,selection_alternatives:[{min:1,max:0,candidate_indices:[]},choice.selection_alternatives[1]]};
  assert.equal(isObjectSelectionValid(decision,[20]),false);
  assert.equal(isObjectSelectionValid(decision,[10,20]),true);
});

test('a mixed group cannot satisfy two different alternatives piecemeal', () => {
  const decision={min:2,max:2,candidates:[{id:1},{id:2},{id:3},{id:4}],selection_alternatives:[
    {min:2,max:2,candidate_indices:[0,1]}, {min:2,max:2,candidate_indices:[2,3]},
  ]};
  assert.equal(isObjectSelectionValid(decision,[1,2]),true);
  assert.equal(isObjectSelectionValid(decision,[3,4]),true);
  assert.equal(isObjectSelectionValid(decision,[1,3]),false);
});


test('local opponent fallback chooses a complete legal alternative', async () => {
  const { buildOpponentDecisionCommand } = await import('../src/lib/opponent-decision.js');
  const command = await buildOpponentDecisionCommand({ perspective:0, decision:{...choice,kind:'select_objects',player:1} }, {});
  assert.deepEqual(command, {type:'select_objects',object_ids:[20]});
});
