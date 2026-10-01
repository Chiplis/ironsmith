import test from 'node:test';
import assert from 'node:assert/strict';
import { createOptimisticMatch } from '../src/lib/optimistic-match.js';

async function harness() {
  let visible = 0, verified = 0;
  const publications = [], cancelled = [];
  const controller = createOptimisticMatch({
    calculate: async candidate => {
      if (candidate.missingCrypto) return null;
      visible += candidate.amount;
      return { state: { count: visible }, publicCheckpointHash: String(visible) };
    },
    restoreVerified: async () => { visible = verified; },
    publish: (state, meta) => publications.push({ state, meta }),
    equivalent: (a, b) => a.amount === b.amount,
  });
  await controller.start('match');
  const stage = (amount, extra = {}) => controller.stage({ matchId: 'match', id: `id-${amount}`,
    amount, cancel: reason => cancelled.push(reason), ...extra });
  const accept = async (seq, amount, count) => {
    verified = count;
    await controller.accept({ seq, amount, audit: { publicCheckpointHash: String(count) } }, { count });
  };
  return { controller, stage, accept, publications, cancelled,
    visible: () => visible, verified: () => verified };
}

test('dependent choices calculate immediately while verification lags, and commit without rewinding the UI', async () => {
  const h = await harness();
  await h.stage(1); await h.stage(2);
  assert.equal(h.visible(), 3); assert.equal(h.verified(), 0);
  assert.equal(h.controller.status().provisionalSequence, 2);
  await h.accept(1, 1, 1);
  assert.equal(h.visible(), 3);
  assert.equal(h.publications.length, 2, 'accepting a prefix never publishes an older board');
  await h.accept(2, 2, 3);
  assert.equal(h.controller.status().pending, 0); assert.equal(h.visible(), 3);
});

test('a failed proof rolls back the provisional suffix and cancels dependent submissions', async () => {
  const h = await harness();
  await h.stage(1); await h.stage(2); await h.accept(1, 1, 1);
  await h.controller.fail(2, 'invalid signature');
  assert.equal(h.visible(), 1); assert.deepEqual(h.cancelled, ['invalid signature']);
  assert.equal(h.controller.status().pending, 0);
});

test('missing RNG, hidden cards, or shuffle material stops calculation at the dependency', async () => {
  const h = await harness(); await h.stage(1);
  assert.equal(await h.stage(2, { missingCrypto: true }), null);
  assert.equal(h.visible(), 1); assert.equal(h.controller.status().pending, 1);
});

test('a peer claimed checkpoint differing from replay discards all dependent choices', async () => {
  const h = await harness(); await h.stage(1); await h.stage(2);
  await h.accept(1, 1, 7);
  assert.equal(h.visible(), 7); assert.equal(h.cancelled.length, 2);
  assert.equal(h.controller.status().pending, 0);
});

test('duplicates coalesce and a conflicting provisional action rolls back', async () => {
  const h = await harness(); await h.stage(1);
  await h.stage(1, { seq: 1 }); assert.equal(h.visible(), 1);
  await h.stage(2, { seq: 1 }); assert.equal(h.visible(), 0);
  assert.equal(h.controller.status().pending, 0);
});

test('gaps, foreign matches, and incorrect parents cannot calculate on the wrong base', async () => {
  const h = await harness();
  assert.equal(await h.stage(1, { seq: 2 }), null);
  assert.equal(await h.stage(1, { matchId: 'foreign' }), null);
  await h.stage(1);
  assert.equal(await h.stage(2, { parentId: 'wrong' }), null);
  assert.equal(h.visible(), 1);
});

test('closing during calculation prevents late state from reappearing', async () => {
  let finish, visible = 0;
  const controller = createOptimisticMatch({
    calculate: () => new Promise(resolve => { finish = () => { visible = 1; resolve({ state: {} }); }; }),
    restoreVerified: async () => { visible = 0; }, publish: () => assert.fail('late publish'), equivalent: () => true,
  });
  await controller.start('match');
  const stage = controller.stage({ matchId: 'match' });
  await new Promise(resolve => setImmediate(resolve));
  const closing = controller.close(); finish();
  assert.equal(await stage, null); await closing;
  assert.equal(visible, 0); assert.equal(controller.status().pending, 0);
});

for (const sequence of [1, 3]) {
  test(`accepting verified sequence ${sequence} preserves the restored visible analysis revision`, async () => {
    const { mergePriorityAnalysis } = await import('../src/lib/priority-analysis-scheduler.js');
    const decision = { kind: 'priority', player: 0, analysis_complete: false, actions: [] };
    const branchState = { decision };
    const visibleState = { ...branchState, __priority_revision: 42 };
    let visible;
    const controller = createOptimisticMatch({
      calculate: async () => ({ state: visibleState }),
      restoreVerified: async () => { visible = visibleState; },
      publish: state => { visible = state; },
      equivalent: () => true,
    });
    await controller.start('match');
    if (sequence === 1) await controller.stage({ matchId: 'match', id: 'land' });
    // A branch snapshot must never overwrite the snapshot published by restore.
    await controller.accept({ seq: sequence }, branchState);
    const actions = [
      { kind: 'play_land', object_id: 121 },
      { kind: 'cast_spell', object_id: 122 },
      { kind: 'activate_ability', object_id: 123 },
    ];
    visible = mergePriorityAnalysis(visible, { revision: 42,
      decision: { ...decision, analysis_complete: true, actions } });
    assert.equal(visible.decision.analysis_complete, true);
    assert.deepEqual(visible.decision.actions, actions);
  });
}
