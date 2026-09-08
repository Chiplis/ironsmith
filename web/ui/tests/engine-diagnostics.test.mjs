import test from 'node:test';
import assert from 'node:assert/strict';
import { readEngineDiagnostics } from '../src/lib/engine-diagnostics.js';

test('exports available counters despite a stuck worker request', async () => {
  let finish;
  const report = await readEngineDiagnostics({
    lastDispatchPerf: () => ({ elapsedMs: 12 }),
    lastWorkCounters: () => new Promise(resolve => { finish = resolve; }),
    lastSnapshotPerf: () => Promise.reject(new Error('worker failed')),
  }, 10);
  assert.equal(report.timedOut, true);
  assert.deepEqual(report.dispatchPerf, { elapsedMs: 12 });
  assert.equal(report.workCounters, null);
  finish({ late: true });
  await Promise.resolve();
  assert.equal(report.workCounters, null);
});

test('supports missing diagnostics and successful reads without timeout', async () => {
  const report = await readEngineDiagnostics({ lastManaPaymentPerf: () => ({ visited_nodes: 3 }) });
  assert.equal(report.timedOut, false);
  assert.deepEqual(report.manaPaymentPerf, { visited_nodes: 3 });
  assert.equal(report.dispatchPerf, null);
});
