import test from 'node:test';
import assert from 'node:assert/strict';
import { cachedInspectorDetails, requestInspectorDetails } from '../src/lib/inspector-details-cache.js';

test('pending and completed details are shared across mounts on the same snapshot', async () => {
  let resolve, calls = 0;
  const game = { objectDetails: () => { calls++; return new Promise(done => { resolve = done; }); } };
  const state = {};
  const first = requestInspectorDetails(game, state, 1);
  assert.equal(requestInspectorDetails(game, state, '1'), first);
  await Promise.resolve();
  assert.equal(calls, 1);
  const details = { name: 'Cached card', power: 2, counters: [] };
  resolve(details);
  await first;
  assert.equal(cachedInspectorDetails(game, state, 1).value, details);
  assert.equal(cachedInspectorDetails(game, state, 1).ready, true);
  assert.equal(await requestInspectorDetails(game, state, 1), details);
  assert.equal(calls, 1);
});

test('new snapshots and different games never reuse old characteristics', async () => {
  let power = 2;
  const game = { objectDetails: async () => ({ power }) };
  const firstState = {}, nextState = {};
  await requestInspectorDetails(game, firstState, 1);
  power = 5;
  assert.equal(cachedInspectorDetails(game, nextState, 1), undefined);
  assert.equal((await requestInspectorDetails(game, nextState, 1)).power, 5);
  assert.equal(cachedInspectorDetails(game, firstState, 1).value.power, 2);
  const anotherGame = { objectDetails: async () => ({ power: 9 }) };
  assert.equal(cachedInspectorDetails(anotherGame, nextState, 1), undefined);
  assert.equal((await requestInspectorDetails(anotherGame, nextState, 1)).power, 9);
});

test('late responses stay in their original snapshot', async () => {
  let finish;
  const game = { objectDetails: () => new Promise(resolve => { finish = resolve; }) };
  const oldState = {}, newState = {};
  const old = requestInspectorDetails(game, oldState, 1);
  await Promise.resolve();
  const finishOld = finish;
  const current = requestInspectorDetails(game, newState, 1);
  await Promise.resolve();
  finish({ power: 7 }); await current;
  finishOld({ power: 1 }); await old;
  assert.equal(cachedInspectorDetails(game, newState, 1).value.power, 7);
});

test('failed requests retry and missing objects settle without repeated requests', async () => {
  let calls = 0;
  const game = { objectDetails: async () => { if (++calls === 1) throw new Error('Temporary failure'); return null; } };
  const state = {};
  await assert.rejects(requestInspectorDetails(game, state, 1));
  assert.equal(cachedInspectorDetails(game, state, 1), undefined);
  assert.equal(await requestInspectorDetails(game, state, 1), null);
  assert.equal(cachedInspectorDetails(game, state, 1).ready, true);
  assert.equal(await requestInspectorDetails(game, state, 1), null);
  assert.equal(calls, 2);
});
