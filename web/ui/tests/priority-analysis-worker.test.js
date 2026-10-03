import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const source = readFileSync(new URL('../src/workers/priorityAnalysisWorker.js', import.meta.url), 'utf8')
  .replace(/^import .*;\n/, '');
function harness() {
  const timers = [], messages = [], imports = [], compiled = [];
  let constructors = 0;
  class WasmGame {
    constructor() { constructors++; }
    free() {}
    setDeferredPriorityAnalysis() {}
    importSyncCheckpoint(checkpoint) { imports.push(checkpoint.id); this.steps = 0; }
    beginPriorityAnalysis() { return true; }
    stepPriorityAnalysis() { return { analysis_complete: ++this.steps === 2, actions: [] }; }
    beginInspectorAnalysis() { this.inspectorSteps = 0; }
    stepInspectorAnalysis() { return ++this.inspectorSteps === 2 ? ['result'] : null; }
  }
  const self = { postMessage: message => messages.push(message) };
  vm.runInNewContext(source, { self, WasmGame, initWasm: async () => {},
    compileAndRegisterCardSources: (_, sources) => { compiled.push(...sources); return {}; },
    setTimeout: fn => timers.push(fn) });
  const flush = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };
  return { messages, imports, compiled, constructors: () => constructors,
    async send(data) { self.onmessage({ data }); await flush(); },
    async tick() { assert.ok(timers.length); timers.shift()(); await flush(); },
    async drain() { for (let i = 0; timers.length && i < 100; i++) { timers.shift()(); await flush(); } assert.equal(timers.length, 0); },
  };
}
const analysis = (token, sources = []) => ({ type: 'analyze', token, sources, checkpoint: { id: token } });

test('cancelled registry setup finishes once and only the newest queued snapshot is analyzed', async () => {
  const h = harness(), sources = [['a', 'A'], ['b', 'B']];
  await h.send(analysis(1, sources));
  await h.send({ type: 'cancel', token: 1, serial: 1 });
  await h.send(analysis(2, sources));
  await h.send({ type: 'cancel', token: 2, serial: 2 });
  await h.send(analysis(3, sources));
  await h.drain();
  assert.equal(h.constructors(), 1);
  assert.deepEqual(h.compiled, ['A', 'B']);
  assert.deepEqual(h.imports, [3]);
  assert.ok(h.messages.some(m => m.type === 'available' && m.cancelSerial === 2));
  assert.ok(h.messages.filter(m => m.type === 'priority').every(m => m.token === 3));
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
