import test from 'node:test';
import assert from 'node:assert/strict';
import { createWorkerTaskDiagnostics } from '../src/lib/worker-task-diagnostics.js';
import { beginEngineRequest, endEngineRequest, recordWorkerTaskDiagnostics,
  recordEngineResultReceipt, getDiagnosticsSnapshot, exportDiagnostics, resetDiagnostics } from '../src/lib/action-diagnostics.js';
import { inRuntimeBranch } from '../src/lib/runtime-branches.js';

test('exports the blocked preparation task and queue without asking the worker', () => {
  resetDiagnostics();
  let clock = 0;
  const tracker = createWorkerTaskDiagnostics({ now: () => clock, wallNow: () => Date.now(),
    publish: recordWorkerTaskDiagnostics });
  const task = tracker.create({ requestId: 10, method: 'dispatch', runtimeBranch: 1,
    commandType: 'priority_action', args: [{ card: 'private identity' }] });
  tracker.enqueue(task); tracker.start(task); tracker.phase(task, 'preparation_wait');
  clock = 200;
  const queued = tracker.create({ requestId: 11, method: 'uiState' }); tracker.enqueue(queued);
  beginEngineRequest(10, 'dispatch', 1); beginEngineRequest(11, 'uiState');
  const report = exportDiagnostics();
  assert.equal(report.workerTasks.active.phase, 'preparation_wait');
  assert.equal(report.workerTasks.active.runtimeBranch, 1);
  assert.ok(report.workerTasks.active.phaseElapsedMs >= 200);
  assert.equal(report.workerTasks.queueDepth, 1);
  assert.equal(report.workerTasks.queued[0].requestId, 11);
  assert.equal(report.engineRequests.count, 2);
  assert.ok(!JSON.stringify(report.workerTasks).includes('private identity'));
  endEngineRequest(10); endEngineRequest(11);
});

test('branch exit and response posting are measured even after an engine error', async () => {
  const messages = []; let clock = 0;
  const tracker = createWorkerTaskDiagnostics({ now: () => clock, wallNow: () => clock, publish: m => messages.push(m) });
  const task = tracker.create({ method: 'dispatch', runtimeBranch: 1 }); tracker.enqueue(task); tracker.start(task);
  let branch = 'visible';
  const game = { exchangeRuntimeSavepoint() { branch = branch === 'visible' ? 'verified' : 'visible'; clock += 5; } };
  await assert.rejects(inRuntimeBranch(game, 1, () => {
    tracker.phase(task, 'engine_call'); clock += 600; throw Error('invalid action');
  }, name => tracker.phase(task, name)), /invalid action/);
  assert.equal(branch, 'visible');
  tracker.leaveQueue(task); tracker.phase(task, 'error_response_post'); clock += 43_000; tracker.finish(task, 'error');
  const completed = messages.at(-1).completed;
  assert.equal(completed.outcome, 'error');
  assert.equal(completed.phases.find(p => p.phase === 'engine_call').ms, 600);
  assert.equal(completed.phases.find(p => p.phase === 'branch_exit').ms, 5);
  assert.equal(completed.phases.at(-1).ms, 43_000);
  assert.equal(messages.at(-1).state.pendingCount, 0);
});

test('background work announces its active stage before execution and never exposes arguments', () => {
  const messages = [];
  const tracker = createWorkerTaskDiagnostics({ publish: m => messages.push(m) });
  tracker.runSync({ kind: 'registry_preload', args: ['secret'] }, () => {
    tracker.phaseActive('registry_preload', { nodeBudget: 16 });
    assert.equal(messages.at(-1).state.active.nodeBudget, 16);
    assert.equal(messages.at(-1).state.active.kind, 'registry_preload');
  });
  assert.equal(messages.at(-1).completed.kind, 'registry_preload');
  assert.ok(!JSON.stringify(messages).includes('secret'));
  const broken = createWorkerTaskDiagnostics({ publish() { throw Error('transport failed'); } });
  assert.equal(broken.runSync({ kind: 'analysis' }, () => 42), 42);
});

test('bounds queue details, phase history and main-thread history and resets sessions', () => {
  resetDiagnostics();
  const tracker = createWorkerTaskDiagnostics({ maxQueued: 3, maxPhases: 2, publish: recordWorkerTaskDiagnostics });
  const tasks = Array.from({ length: 50 }, () => tracker.create({ method: 'uiState' }));
  for (const task of tasks) tracker.enqueue(task);
  assert.equal(getDiagnosticsSnapshot().workerTasks.queued.length, 3);
  assert.equal(getDiagnosticsSnapshot().workerTasks.queueDepth, 50);
  for (const task of tasks) {
    tracker.start(task); tracker.phase(task, 'engine_call'); tracker.phase(task, 'response_post'); tracker.finish(task);
  }
  const report = getDiagnosticsSnapshot().workerTasks;
  assert.equal(report.recent.length, 32);
  assert.ok(report.recent[0].omittedPhases > 0);
  assert.equal(report.recent[0].phases.length, 3);
  tracker.reset();
  assert.equal(getDiagnosticsSnapshot().workerTasks.recent.length, 0);
  assert.equal(getDiagnosticsSnapshot().workerTasks.active, null);
});

test('records response delivery and decode time separately from worker execution', () => {
  resetDiagnostics(); beginEngineRequest(5, 'uiState', 2);
  recordEngineResultReceipt(5, { sentAtWall: 1000, receivedAtWall: 44_000, snapshotDecodeMs: 7 });
  endEngineRequest(5);
  const tracker = createWorkerTaskDiagnostics({ publish: recordWorkerTaskDiagnostics }); tracker.reset();
  const receipt = exportDiagnostics().workerTasks.resultReceipts[0];
  assert.equal(receipt.deliveryDelayMs, 43_000); assert.equal(receipt.snapshotDecodeMs, 7);
  assert.equal(receipt.runtimeBranch, 2);
});
