import test from 'node:test';
import assert from 'node:assert/strict';
import { createLocalRuntimeRecovery, recoverVerifiedRuntime } from '../src/lib/local-runtime-recovery.js';

test('recovery uses current verified runtime and applies only missed actions', async () => {
  const calls = [];
  const result = await recoverVerifiedRuntime({ current: { level: 'current', seq: 8 }, saved: [],
    restore: async candidate => calls.push(['restore', candidate.seq]),
    genesis: async () => calls.push('genesis'),
    replay: async sequence => calls.push(['tail', sequence]), verify: async () => calls.push('verify') });
  assert.equal(result.level, 'current');
  assert.deepEqual(calls, [['restore', 8], ['tail', 8], 'verify']);
});

test('bad current state and bad tail fall back through local savepoints to genesis', async () => {
  let state = 0;
  const visits = [];
  const result = await recoverVerifiedRuntime({ current: { level: 'current', seq: 8 },
    saved: [{ level: 'local-savepoint', seq: 4 }],
    restore: async candidate => { visits.push(candidate.seq); state = candidate.seq; },
    genesis: async () => { visits.push(0); state = 0; },
    replay: async sequence => { state += 100; if (sequence === 8) throw new Error('corrupt native state'); },
    verify: async () => { if (state !== 100) throw new Error('signed head mismatch'); } });
  assert.equal(result.level, 'genesis');
  assert.deepEqual(visits, [8, 4, 0]);
  assert.equal(state, 100);
});

test('failed genesis is rejected rather than accepting an unverified state', async () => {
  await assert.rejects(recoverVerifiedRuntime({ current: null, saved: [], restore: async () => {},
    genesis: async () => {}, replay: async () => {}, verify: async () => { throw new Error('invalid proof'); } }), /invalid proof/);
});

test('local savepoints cannot cross engines, seats, matches, or transcript forks', async () => {
  const cache = createLocalRuntimeRecovery();
  const game = {};
  const snapshot = { auditStateHash: 'signed-state', release: async () => {} };
  await cache.remember({ game, matchId: 'genesis-A', seat: 1, seq: 1, prefixHash: 'prefix-A', snapshot });
  const context = { game, matchId: 'genesis-A', seat: 1,
    actions: [{ prefixHash: 'prefix-A', audit: { nextStateHash: 'signed-state' } }] };
  assert.equal(cache.candidates(context).length, 1);
  for (const mismatch of [{ game: {} }, { seat: 0 }, { matchId: 'genesis-B' },
    { actions: [{ prefixHash: 'fork', audit: { nextStateHash: 'signed-state' } }] },
    { actions: [{ prefixHash: 'prefix-A', audit: { nextStateHash: 'different-state' } }] }]) {
    assert.equal(cache.candidates({ ...context, ...mismatch }).length, 0);
  }
  await cache.clear();
});

test('bounded savepoints release native handles when replaced and on teardown', async () => {
  const cache = createLocalRuntimeRecovery({ limit: 2 });
  const released = [];
  const game = {};
  for (const seq of [1, 2, 3]) await cache.remember({ game, matchId: 'A', seat: 0, seq,
    snapshot: { release: async () => released.push(seq) } });
  assert.deepEqual(released, [1]);
  await cache.clear();
  assert.deepEqual(released, [1, 3, 2]);
  await cache.clear();
  assert.equal(released.length, 3);
});
