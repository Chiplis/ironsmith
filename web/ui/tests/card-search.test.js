import test from 'node:test';
import assert from 'node:assert/strict';
import { cardSearchKey } from '../src/lib/card-search.js';
import { customCardOptions } from '../src/lib/custom-card-library.js';

test('card search folds accents and combining marks without changing displayed names', () => {
  for (const query of ['Kili', 'KÍLI', 'Ki\u0301li', '  Kíli  ']) {
    assert.equal(cardSearchKey(query), 'kili');
  }
  const cards = [{ id: 'kili', draft: { faces: [{ name: 'Kíli the Resourceful' }] } }];
  assert.equal(customCardOptions(cards, 'kili')[0].name, 'Kíli the Resourceful');
  assert.equal(customCardOptions(cards, 'resourceful')[0].card.id, 'kili');
  assert.deepEqual(customCardOptions(cards, 'unrelated'), []);
});
