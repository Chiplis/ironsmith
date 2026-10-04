import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { actionRefObjectId } from '../src/lib/sync-object-identity.js';

const source = readFileSync(new URL('../src/hooks/peer-lobby/shared.js', import.meta.url), 'utf8');
function declaration(name) {
  const start = source.indexOf(`export function ${name}(`);
  if (start < 0) return '';
  const end = source.indexOf('\nexport ', start + 1);
  return source.slice(start, end).replace(/^export /, '');
}
const collect = new Function('actionRefObjectId', [
  'isFaceDownCastCommand', 'isForetellCommand', 'collectCommandObjectIds',
].map(declaration).join('\n') + '\nreturn collectCommandObjectIds;')(actionRefObjectId);
const command = action_ref => ({ type: 'priority_action', action_ref, object_id: 13 });

test('foretell keeps its command-referenced hand card private', () => {
  assert.deepEqual([...collect(command({ kind: 'special_action', action: { kind: 'foretell', card_id: 13 } }))], []);
  assert.deepEqual([...collect({ type: 'priority_action', actionRef: { kind: 'special_action', action: { kind: 'foretell', card_id: 13 } } })], []);
});

test('casting the foretold card later still requires a public opening', () => {
  assert.deepEqual([...collect(command({ kind: 'cast_spell', spell_id: 13, from_zone: 'exile', casting_method: { kind: 'alternative', index: 0 } }))], [13]);
  assert.deepEqual([...collect(command({ kind: 'special_action', action: { kind: 'plot', card_id: 13 } }))], [13]);
});

test('face-down casts retain their private command behavior', () => {
  assert.deepEqual([...collect(command({ kind: 'cast_spell', spell_id: 13, casting_method: { kind: 'face_down', face_down_kind: 'morph' } }))], []);
});


test('sourced face-down library casts never request a public identity opening', () => {
  assert.deepEqual([...collect(command({ kind: 'cast_spell', spell_id: 13, from_zone: 'library',
    casting_method: { kind: 'face_down_play_from', source: 24, zone: 'library', face_down_kind: 'disguise' } }))], []);
});
