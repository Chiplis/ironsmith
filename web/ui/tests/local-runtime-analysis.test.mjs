// Source-authored only; these scenarios have not been executed.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createLocalPriorityWorker, createLocalQueryWorker, collectLocalTargetPreviews } from '../src/lib/local-runtime-analysis.js';
import { inRuntimeBranch } from '../src/lib/runtime-branches.js';
import { captureEngineRestorePoint, restoreEngineRestorePoint } from '../src/lib/engine-restore-point.js';

const flush = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };
function fixture({ fail = false } = {}) {
  let active = { shield: 3, nextId: 100, step: 0 }, handle = 0;
  const saved = new Map(), timers = [], messages = [], released = [];
  const visible = active;
  const game = { exchangeRuntimeSavepoint(id) {
    const next = saved.get(id); assert.ok(next); saved.set(id, active); active = next;
  } };
  const worker = createLocalPriorityWorker({
    capture: async () => { const id = ++handle; saved.set(id, structuredClone(active)); return id; },
    call: async (id, method) => inRuntimeBranch(game, id, () => {
      assert.notEqual(active, visible); assert.equal(active.shield, 3);
      if (method === 'beginPriorityAnalysis') return true;
      if (method === 'stepPriorityAnalysis') {
        active.nextId++; if (fail) throw Error('resource limit: incomplete analysis');
        return { actions: ['legal'], analysis_complete: ++active.step === 2 };
      }
      if (method === 'stepInspectorAnalysis') return ['ability'];
      return undefined;
    }),
    release: async id => { released.push(id); saved.delete(id); },
    yieldTask: () => new Promise(resolve => timers.push(resolve)),
  });
  worker.onmessage = ({ data }) => messages.push(data);
  return { worker, messages, visible, released, saved,
    async drain() { await flush(); while (timers.length) { timers.shift()(); await flush(); } },
    async tick() { await flush(); timers.shift()?.(); await flush(); },
  };
}

test('sliced local priority and inspector queries retain shields without changing visible counters', async () => {
  const h = fixture();
  h.worker.postMessage({ type: 'analyze', token: 1 });
  h.worker.postMessage({ type: 'inspector', token: 1, id: 9, args: [77] });
  await h.drain();
  assert.ok(h.messages.some(message => message.type === 'priority' && message.decision.analysis_complete));
  assert.deepEqual(h.messages.find(message => message.type === 'inspector').result, ['ability']);
  assert.deepEqual(h.visible, { shield: 3, nextId: 100, step: 0 });
  h.worker.terminate(); await flush(); assert.equal(h.saved.size, 0); assert.deepEqual(h.released, [1]);
});

test('cancelled local slices release the old branch and publish no old completion', async () => {
  const h = fixture(); h.worker.postMessage({ type: 'analyze', token: 1 }); await flush();
  h.worker.postMessage({ type: 'cancel', token: 1, serial: 1 });
  h.worker.postMessage({ type: 'analyze', token: 2 }); await h.drain();
  assert.ok(!h.messages.some(message => message.type === 'priority' && message.token === 1 && message.decision.analysis_complete));
  assert.ok(h.messages.some(message => message.type === 'priority' && message.token === 2 && message.decision.analysis_complete));
  assert.equal(h.visible.nextId, 100); assert.deepEqual(h.released, [1]);
  h.worker.terminate(); await flush(); assert.deepEqual(h.released, [1, 2]);
});

test('resource failure is reported, never converted to an empty completed menu', async () => {
  const h = fixture({ fail: true }); h.worker.postMessage({ type: 'analyze', token: 1 }); await h.drain();
  assert.ok(h.messages.some(message => message.type === 'error' && /incomplete analysis/.test(message.error)));
  assert.ok(!h.messages.some(message => message.type === 'priority')); assert.equal(h.saved.size, 0);
  assert.equal(h.visible.nextId, 100);
});

test('cancelled local payment inventory never publishes stale results', async () => {
  let finish, result = false;
  const worker = createLocalQueryWorker(() => new Promise(resolve => { finish = resolve; }));
  worker.onmessage = () => { result = true; };
  worker.postMessage({ token: 1 }); await flush(); worker.terminate(); finish(['stale']); await flush();
  assert.equal(result, false);
});

test('optional unsupported checkpoint backup does not invalidate a lossless native restore point', async () => {
  let restored = false;
  const game = { supportsRuntimeSavepoints: true, createRuntimeSavepoint: async () => 7,
    restoreRuntimeSavepoint: async id => { assert.equal(id, 7); restored = true; },
    exportSyncCheckpoint: async () => { throw Error('unencoded active shield'); } };
  const point = await captureEngineRestorePoint(game, { keepCheckpoint: true });
  assert.equal(point.checkpoint, undefined); await restoreEngineRestorePoint(game, point, 0); assert.equal(restored, true);
  await assert.rejects(() => captureEngineRestorePoint({ ...game, supportsRuntimeSavepoints: false }), /does not support native runtime restore points/);
});

test('each target preview starts from the exact shield state and restores IDs even on rejection', async () => {
  let active = { shields: [3], nextId: 21 }, other;
  const visible = active;
  const game = { exchangeRuntimeSavepoint() { [active, other] = [other, active]; } };
  const run = action => {
    other = structuredClone(visible);
    return inRuntimeBranch(game, 1, () => {
      assert.deepEqual(active.shields, [3]); active.shields.pop();
      const id = active.nextId++;
      if (action.reject) throw Error('incomplete preview');
      return [{ source: action.source, targetId: id }];
    });
  };
  const result = await collectLocalTargetPreviews([{ source: 'first' }, { source: 'second' }], 0, run, () => true, async () => {});
  assert.deepEqual(result.requirements.map(entry => entry.targetId), [21, 21]);
  assert.deepEqual(visible, { shields: [3], nextId: 21 });
  await assert.rejects(() => collectLocalTargetPreviews([{ reject: true }], 0, run, () => true), /incomplete preview/);
  assert.equal(active, visible); assert.equal(visible.nextId, 21);
  assert.equal(await collectLocalTargetPreviews([{}], 0, run, () => false), null);
});

test('cancellation during native branch acquisition releases exactly that branch', async () => {
  let captured, releases = 0;
  const worker = createLocalPriorityWorker({ capture: () => new Promise(resolve => { captured = resolve; }),
    call: async () => { throw Error('cancelled branch must never execute'); }, release: async id => { assert.equal(id, 8); releases++; } });
  worker.postMessage({ type: 'analyze', token: 1 }); await flush();
  worker.postMessage({ type: 'cancel', token: 1, serial: 1 }); captured(8); await flush();
  assert.equal(releases, 1); worker.terminate(); await flush(); assert.equal(releases, 1);
});
