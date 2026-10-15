import test from 'node:test';
import assert from 'node:assert/strict';
import { loadRandomStartingBoard } from '../src/lib/starting-board.js';

const names = ['Alice', 'Bob', 'Charlie', 'Diana'];
const match = commander => ({ match: {
  commanders: names.map(() => [commander]),
  decks: names.map(() => ['Forest']),
} });

test('unsupported prefetched commander is excluded before another random match is loaded', async () => {
  const rejected = match('Unsupported Leader');
  const replacement = match('Working Leader');
  const applied = [];
  const result = await loadRandomStartingBoard(names, 40, 0, {
    initialPayload: rejected,
    build: async (players, life, threshold, { excludedCards }) => {
      assert.deepEqual(players, names);
      assert.equal(life, 40);
      assert.equal(threshold, 0);
      assert.deepEqual([...excludedCards], ['unsupported leader']);
      return replacement;
    },
    apply: async payload => {
      applied.push(payload);
      if (payload === rejected) throw new Error('unknown card name: Unsupported Leader');
    },
  });
  assert.equal(result, replacement);
  assert.deepEqual(applied, [rejected, replacement]);
});

test('unsupported main-deck cards are excluded as well', async () => {
  let builds = 0;
  await loadRandomStartingBoard(names, 40, 0, {
    initialPayload: match('Working Leader'),
    build: async (_names, _life, _threshold, { excludedCards }) => {
      assert.deepEqual([...excludedCards], ['forest']);
      builds++;
      return match('Working Leader');
    },
    apply: async () => { if (!builds) throw new Error('unsupported card: Forest'); },
  });
  assert.equal(builds, 1);
});

test('unrelated engine errors and non-Commander errors are not retried', async () => {
  for (const [payload, error] of [
    [match('Leader'), new Error('WASM memory access out of bounds')],
    [{ players: [] }, new Error('unknown card name: Forest')],
  ]) {
    await assert.rejects(loadRandomStartingBoard(names, 40, 0, {
      initialPayload: payload,
      build: async () => assert.fail('must not retry'),
      apply: async () => { throw error; },
    }), error);
  }
});

test('catalog exhaustion stops retries', async () => {
  await assert.rejects(loadRandomStartingBoard(names, 40, 0, {
    initialPayload: match('Unsupported Leader'),
    build: async () => { throw new Error('No complete supported commander deck'); },
    apply: async () => { throw new Error('unsupported commander'); },
  }), /No complete supported commander deck/);
});
