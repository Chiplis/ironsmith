import test from 'node:test';
import assert from 'node:assert/strict';
import { manaActionSourceId, manaColorChoiceCommand } from '../src/lib/mana-color-choice.js';

const colors = ['White', 'Blue', 'Black', 'Red', 'Green'].map((description, index) => ({ index, description, legal: true }));
const prompt = { kind: 'select_options', choice_kind: 'colors', min: 1, max: 1, source_id: 42, description: 'Add one mana of any color', options: colors };

test('a strip click answers the any-color prompt with its color', () => {
  assert.deepEqual(manaColorChoiceCommand(prompt, { color: 'U', sourceId: '42' }), { type: 'select_options', option_indices: [1] });
});

test('another source or a non-color prompt stays with the player', () => {
  assert.equal(manaColorChoiceCommand(prompt, { color: 'U', sourceId: '7' }), null);
  assert.equal(manaColorChoiceCommand({ ...prompt, choice_kind: 'options', description: 'Choose a creature type' }, { color: 'U', sourceId: '42' }), null);
  assert.equal(manaColorChoiceCommand({ ...prompt, max: 2 }, { color: 'U', sourceId: '42' }), null);
  assert.equal(manaColorChoiceCommand({ ...prompt, options: colors.filter(option => option.description !== 'Blue') }, { color: 'U', sourceId: '42' }), null);
});

test('mana actions name their source from either action shape', () => {
  assert.equal(manaActionSourceId({ action_ref: { kind: 'activate_mana_ability', source: 42, ability_index: 0 } }), '42');
  assert.equal(manaActionSourceId({ source_id: '42', ability_index: 0 }), '42');
});
