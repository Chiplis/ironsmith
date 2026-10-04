import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { createLocalAnalysisReplica } from '../src/lib/local-analysis-replay.js';

const source = readFileSync(new URL('../src/workers/priorityAnalysisWorker.js', import.meta.url), 'utf8')
  .replace(/^import .*;\n/gm, '');
function harness({ rejectedSource = null, checkpointError = null } = {}) {
  const timers = [], messages = [], imports = [], compiled = [];
  let constructors = 0;
  class WasmGame {
    constructor() { constructors++; }
    free() {}
    points = new Map(); nextHandle = 0;
    createRuntimeSavepoint() { const h = ++this.nextHandle; this.points.set(h, this.steps); return h; }
    exchangeRuntimeSavepoint(h) { const before = this.steps; this.steps = this.points.get(h); this.points.set(h, before); }
    releaseRuntimeSavepoint(h) { return this.points.delete(h); }
    registerSource(source) { compiled.push(source); return { failed: source === rejectedSource ? ['unsupported'] : [] }; }
    loadSnapshot(id) { if (checkpointError) throw new Error(checkpointError); imports.push(id); this.steps = 0; }

    setDeferredPriorityAnalysis() {}
    initializeRuntimeIdentityOrigin() { this.steps = 0; }
    beginPriorityAnalysis() { return true; }
    stepPriorityAnalysis() { return { analysis_complete: ++this.steps === 2, actions: [] }; }
    beginInspectorAnalysis() { this.inspectorSteps = 0; }
    stepInspectorAnalysis() { return ++this.inspectorSteps === 2 ? ['result'] : null; }
  }
  const self = { postMessage: message => messages.push(message) };
  vm.runInNewContext(source, { self, WasmGame, createLocalAnalysisReplica, initWasm: async () => {},
    compileAndRegisterCardSources: (_, sources) => { compiled.push(...sources); return { failed: sources.includes(rejectedSource) ? [{ error: "unsupported mechanics" }] : [] }; },
    setTimeout: fn => timers.push(fn) });
  const flush = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };
  return { messages, imports, compiled, constructors: () => constructors,
    async send(data) { self.onmessage({ data }); await flush(); },
    async tick() { assert.ok(timers.length); timers.shift()(); await flush(); },
    async drain() { for (let i = 0; timers.length && i < 100; i++) { timers.shift()(); await flush(); } assert.equal(timers.length, 0); },
  };
}
const analysis = (token, sources = []) => ({ type: 'analyze', token, localReplay: {
  epoch: 1, identityOrigin: { object: 1 }, operations: [
    ...sources.map(([, source]) => ({ method: 'registerSource', args: [source], failed: false })),
    ...Array.from({ length: token }, (_, i) => ({ method: 'loadSnapshot', args: [i + 1], failed: false })),
  ],
} });

test('cancelled searches retain replay progress and only the newest queued snapshot completes', async () => {
  const h = harness(), sources = [['a', 'A'], ['b', 'B']];
  await h.send(analysis(1, sources));
  await h.send({ type: 'cancel', token: 1, serial: 1 });
  await h.send(analysis(2, sources));
  await h.send({ type: 'cancel', token: 2, serial: 2 });
  await h.send(analysis(3, sources));
  await h.drain();
  assert.equal(h.constructors(), 1);
  assert.deepEqual(h.compiled, ['A', 'B']);
  assert.deepEqual(h.imports, [1, 2, 3]);
  assert.ok(h.messages.some(m => m.type === 'available' && m.cancelSerial === 2));
  assert.ok(h.messages.filter(m => m.type === 'priority' && m.decision.analysis_complete).every(m => m.token === 3));
  assert.ok(h.messages.some(m => m.type === 'priority' && m.decision.analysis_complete));
});

test('superseding a yielded search retains its runtime and rejects old inspectors', async () => {
  const h = harness();
  await h.send(analysis(1));
  assert.equal(h.messages.filter(m => m.type === 'priority').length, 1);
  await h.send({ type: 'cancel', token: 1, serial: 1 });
  await h.send(analysis(2));
  await h.send({ type: 'inspector', token: 1, id: 10, args: [] });
  await h.send({ type: 'inspector', token: 2, id: 20, args: [] });
  await h.drain();
  assert.equal(h.constructors(), 1);
  assert.deepEqual(h.imports, [1, 2]);
  assert.equal(h.messages.filter(m => m.type === 'priority' && m.token === 1).length, 1);
  assert.deepEqual(h.messages.filter(m => m.type === 'inspector').map(m => m.id), [20]);
});

test('inspector cancellation yields to the next snapshot without publishing its stale result', async () => {
  const h = harness();
  await h.send(analysis(1)); await h.drain();
  await h.send({ type: 'inspector', token: 1, id: 10, args: [] });
  await h.send({ type: 'cancel', token: 1, serial: 1 });
  await h.send(analysis(2));
  await h.drain();
  assert.equal(h.messages.filter(m => m.type === 'inspector').length, 0);
  assert.deepEqual(h.imports, [1, 2]);
  assert.equal(h.constructors(), 1);
});


test('a rejected fetched source does not strand the guest priority menu', async () => {
  const h = harness({ rejectedSource: 'unsupported' });
  await h.send(analysis(1, [['rejected', 'unsupported'], ['land', 'Mountain']]));
  await h.drain();
  assert.deepEqual(h.compiled, ['unsupported', 'Mountain']);
  assert.deepEqual(h.imports, [1]);
  assert.equal(h.messages.some(m => m.type === 'error'), false);
  assert.ok(h.messages.some(m => m.type === 'priority' && m.decision.analysis_complete));
  await h.send(analysis(2, [['rejected', 'unsupported'], ['land', 'Mountain']]));
  await h.drain();
  assert.equal(h.constructors(), 1, 'the initialized registry survives the diagnostic failure');
  assert.deepEqual(h.imports, [1, 2]);
});


test('priority analysis reports replay divergence instead of publishing a partial reconstructed state', async () => {
  const h = harness({ checkpointError: 'missing required definition' });
  await h.send(analysis(1));
  await h.drain();
  assert.ok(h.messages.some(m => m.type === 'error' && m.error.includes('missing required definition')));
  assert.equal(h.messages.some(m => m.type === 'priority'), false);
});
