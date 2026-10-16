import test from 'node:test';
import assert from 'node:assert/strict';
import { battlefieldKeywords, keywordGainEvents } from '../src/lib/effect-palette.js';
const snapshot = (abilities) => ({ players: [{ battlefield: [{ stable_id: 1, abilities }] }] });
test('keyword gains emit once; ability references and initial cards do not', () => {
  const before = battlefieldKeywords(snapshot(['Flying', 'Other creatures have hexproof.']));
  const after = battlefieldKeywords(snapshot(['Flying, Hexproof', 'Ward {2}']));
  assert.deepEqual([...before.get('1')], ['flying']);
  assert.deepEqual(keywordGainEvents(before, after).map(event => event.label), ['hexproof', 'ward']);
  assert.equal(keywordGainEvents(after, after).length, 0);
  assert.equal(keywordGainEvents(new Map(), after).length, 0);
});
test('unchanged printed keywords use the snapshot oracle fallback', () => {
  const keywords = battlefieldKeywords({ players: [{ battlefield: [{ stable_id: 2, abilities: [], oracle_text: 'Flying\nVigilance' }] }] });
  assert.deepEqual([...keywords.get('2')], ['flying', 'vigilance']);
});
