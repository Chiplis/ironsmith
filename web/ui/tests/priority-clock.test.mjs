import test from 'node:test';
import assert from 'node:assert/strict';
import { AUTO_RESOLVE_STORAGE_KEY, readAutoResolveMode } from '../src/lib/priority-clock.js';

test('clock defaults to white and restores each saved mode', () => {
  for (const [saved, expected] of [[null, 'safe'], ['off', 'off'], ['safe', 'safe'],
    ['all', 'all'], ['true', 'safe'], ['regular', 'safe'], ['false', 'off'], ['invalid', 'safe']]) {
    assert.equal(readAutoResolveMode({ getItem(key) {
      assert.equal(key, AUTO_RESOLVE_STORAGE_KEY);
      return saved;
    } }), expected);
  }
});

test('clock works with the default white mode when storage is blocked', () => {
  assert.equal(readAutoResolveMode({ getItem() { throw new Error('blocked'); } }), 'safe');
});
