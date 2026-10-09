import test from 'node:test';
import assert from 'node:assert/strict';
import { stackFramePreview } from '../src/lib/stack-frame-preview.js';
import { resolveStackInspectObjectId } from '../src/lib/inspector-selection.js';
const entry = { id: 20, inspect_object_id: 10, source_stable_id: 7, name: 'Source', ability_kind: 'Triggered' };
const original = { id: 10, stable_id: 7, name: 'Source', type_line: 'Creature', oracle_text: 'Draw a card.' };
const wrong = { id: 20, stable_id: 99, name: 'Wrong card' };
for (const zone of ['battlefield', 'graveyard_cards', 'exile_cards', 'hand_cards', 'command_cards']) {
  test(`resolution follows source into ${zone}`, () => {
    const moved = { ...original, id: 30 };
    const state = { decision: { kind: 'select_objects' }, resolving_stack_object: entry, players: [{ battlefield: [wrong], [zone]: [wrong, moved] }] };
    assert.equal(stackFramePreview(state, [original]).card, moved);
    assert.equal(resolveStackInspectObjectId(state, entry), '30');
  });
}
test('vanished token source retains its frame and ignores presentation-id collision', () => {
  const state = { stack_objects: [entry], players: [{ battlefield: [wrong] }] };
  assert.equal(stackFramePreview(state, [original]).card, original);
  assert.equal(stackFramePreview(state).card, entry);
  assert.equal(resolveStackInspectObjectId(state, { id: 20 }), null);
});
test('active resolution beats remaining stack; priority discards stale resolving snapshot', () => {
  const next = { id: 40, name: 'Next spell' };
  const state = { stack_objects: [next], resolving_stack_object: entry, decision: { kind: 'select_options' } };
  assert.equal(stackFramePreview(state).entry, entry);
  assert.equal(stackFramePreview({ ...state, decision: { kind: 'priority' } }).entry, next);
  assert.equal(stackFramePreview({ ...state, stack_objects: [], decision: { kind: 'priority' } }), null);
});
test('same-name objects never substitute for a missing source', () => {
  const other = { ...original, id: 80, stable_id: 81 };
  assert.equal(stackFramePreview({ stack_objects: [entry], players: [{ battlefield: [other] }] }, [other]).card, entry);
});
