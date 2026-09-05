import test from 'node:test';
import assert from 'node:assert/strict';
import { retainBattlefieldSlots } from '../src/lib/battlefield-layout.js';
const card = (id, lane = 'creatures') => ({ id, stable_id: id, lane });
const layout = (cards, previous) => retainBattlefieldSlots(cards, previous, { columns: 6 });

test('departures leave holes and arrivals fill them without moving survivors', () => {
  const initial = layout([card(1), card(2), card(3), card(4, 'lands')]);
  const removed = layout([card(3), card(1), card(4, 'lands')], initial);
  const added = layout([card(5), ...removed.orderedCards], removed);
  for (const id of [1, 3, 4]) {
    assert.deepEqual(removed.gridPositionById.get(String(id)), initial.gridPositionById.get(String(id)));
    assert.deepEqual(added.gridPositionById.get(String(id)), initial.gridPositionById.get(String(id)));
  }
  assert.deepEqual(added.gridPositionById.get('5'), initial.gridPositionById.get('2'));
});

test('overflow and dense counts append slots without changing grid width or existing positions', () => {
  const initial = layout([card(1), card(2, 'lands')]);
  const crowded = layout([...initial.orderedCards, ...Array.from({length: 60}, (_, i) => card(i + 3))], initial);
  assert.equal(crowded.maxCols, initial.maxCols);
  assert.equal(new Set([...crowded.gridPositionById.values()].map(p => `${p.row}:${p.column}`)).size, 62);
  const removed = layout(initial.orderedCards, crowded);
  for (const id of ['1', '2']) {
    assert.deepEqual(crowded.gridPositionById.get(id), initial.gridPositionById.get(id));
    assert.deepEqual(removed.gridPositionById.get(id), initial.gridPositionById.get(id));
  }
});

test('stable and grouped identities preserve slots when runtime representatives change', () => {
  const initial = layout([card(1), { ...card(2), member_stable_ids: [2, 3] }]);
  const updated = layout([{ ...card(11), stable_id: 1 }, { ...card(33), member_stable_ids: [3] }], initial);
  assert.deepEqual(updated.gridPositionById.get('11'), initial.gridPositionById.get('1'));
  assert.deepEqual(updated.gridPositionById.get('33'), initial.gridPositionById.get('2'));
});

test('automatic placement starts at the center and expands outward in each row', () => {
  const result = layout([card(1), card(2), card(3), card(4), card(5, 'lands')]);
  assert.deepEqual([1, 2, 3, 4].map(id => result.gridPositionById.get(String(id)).column), [3, 4, 2, 5]);
  assert.deepEqual(result.gridPositionById.get('5'), { row: 2, column: 3, groupId: 'back' });
});
