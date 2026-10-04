import test from 'node:test';
import assert from 'node:assert/strict';
import { createIsolatedPriorityAnalysis } from '../src/lib/isolated-priority-analysis.js';
import { mergePriorityAnalysis } from '../src/lib/priority-analysis-scheduler.js';
import { priorityHoldReason } from '../src/lib/priority-automation.js';

function harness({ capture = async () => ({ checkpoint: { perspective: 0 } }), deliver } = {}) {
  const timers = new Map(), workers = [], events = [], errors = [];
  let id = 0, identity = 'A';
  const scheduler = createIsolatedPriorityAnalysis({ capture, deliver, identity: () => identity,
    pending: () => true, publish: value => events.push(value), fail: value => errors.push(value),
    schedule: (fn, delay = 0) => { timers.set(++id, { fn, delay }); return id; }, cancel: key => timers.delete(key),
    createWorker() {
      const worker = { sent: [], terminated: false,
        postMessage(message) { this.sent.push(message); }, terminate() { this.terminated = true; },
        reply(data) { this.onmessage({ data: { token: this.sent[0].token, ...data } }); } };
      workers.push(worker); return worker;
    } });
  return { scheduler, workers, events, errors, timers, identity: value => { identity = value; },
    async tick() { const [key, { fn }] = [...timers].sort((a, b) => a[1].delay - b[1].delay)[0]; timers.delete(key); await fn(); } };
}
test('analysis executes only in the separate worker, publishes partial menus, and captures once', async () => {
  let captures = 0;
  const h = harness({ capture: async () => { captures++; return {}; } });
  h.scheduler.start(7); await h.tick();
  h.scheduler.start(7); assert.equal(h.timers.size, 0); assert.equal(captures, 1);
  const w = h.workers[0];
  w.reply({ type: 'priority', sequence: 1, decision: { player: 0, analysis_complete: false, actions: ['land'] } });
  assert.deepEqual(h.events[0].decision.actions, ['land']);
  w.reply({ type: 'priority', sequence: 2, decision: { player: 0, analysis_complete: true, actions: ['land', 'warp'] } });
  assert.equal(h.events[1].revision, 7); assert.equal(h.events[1].sequence, 2);
});
test('cancellation rejects late results immediately and reuses a cooperative worker', async () => {
  const h = harness(); h.scheduler.start(); await h.tick(); const old = h.workers[0];
  h.scheduler.invalidate(); assert.equal(old.terminated, false);
  assert.equal(old.sent.at(-1).type, 'cancel');
  old.reply({ type: 'priority', decision: { analysis_complete: true } }); assert.equal(h.events.length, 0);
  h.identity('B'); h.scheduler.start(); await h.tick();
  assert.equal(h.workers.length, 1);
  old.reply({ type: 'available', cancelSerial: old.sent.findLast(m => m.type === 'cancel').serial });
  old.reply({ type: 'priority', token: old.sent.at(-1).token, decision: { analysis_complete: false } });
  assert.equal(h.events[0].revision, 1);
});
test('invalidation during snapshot capture cannot launch an obsolete worker', async () => {
  let resolve;
  const h = harness({ capture: () => new Promise(done => { resolve = done; }) });
  h.scheduler.start(); const tick = h.tick(); h.scheduler.invalidate(); resolve({}); await tick;
  assert.equal(h.workers.length, 0);
});
test('a rejected command can resume with the unchanged visible revision', async () => {
  const h = harness(); h.scheduler.start(0); await h.tick(); h.scheduler.invalidate();
  h.scheduler.start(0); await h.tick();
  h.workers[0].reply({ type: 'priority', token: h.workers[0].sent.at(-1).token, decision: {} }); assert.equal(h.events[0].revision, 0);
});
test('inspector work is isolated, deduplicated and cancelled without holding the command queue', async () => {
  const h = harness(); const a = h.scheduler.inspector(1n, 2);
  assert.equal(h.scheduler.inspector(1n, 2), a); await h.tick();
  const w = h.workers[0], request = w.sent.find(m => m.type === 'inspector');
  w.reply({ type: 'inspector', id: request.id, result: ['ability'] });
  assert.deepEqual(await a, ['ability']); assert.equal(h.scheduler.inspector(1n, 2), a);
  const b = h.scheduler.inspector(2n, 0); h.scheduler.invalidate(); assert.deepEqual(await b, []);
});
test('analysis errors do not mark unchecked cards illegal or strand inspector callers', async () => {
  const h = harness(); const request = h.scheduler.inspector(1n, 0); await h.tick();
  h.workers[0].reply({ type: 'error', error: 'bad checkpoint' });
  assert.equal(h.events.length, 0); assert.equal(h.errors.length, 1); await assert.rejects(request, /bad checkpoint/);
});
test('partial actions merge monotonically and stale revisions and players are rejected', () => {
  const state = { __priority_revision: 3, decision: { kind: 'priority', player: 0, analysis_complete: false } };
  const first = { revision: 3, sequence: 1, decision: { kind: 'priority', player: 0, analysis_complete: false, actions: ['land'] } };
  const partial = mergePriorityAnalysis(state, first); assert.deepEqual(partial.decision.actions, ['land']);
  assert.equal(mergePriorityAnalysis(partial, first), partial);
  assert.equal(mergePriorityAnalysis(partial, { ...first, revision: 2 }), partial);
  assert.equal(mergePriorityAnalysis(partial, { ...first, decision: { player: 1 } }), partial);
  const final = mergePriorityAnalysis(partial, { ...first, sequence: 2, decision: { ...first.decision, analysis_complete: true, actions: ['land', 'warp'] } });
  assert.equal(final.decision.analysis_complete, true); assert.equal(mergePriorityAnalysis(final, first), final);
});
test('pending menus cannot prove that auto-pass if no actions is safe', () => {
  const args = { autoPassEnabled: true, decision: { kind: 'priority', player: 0, analysis_complete: false, actions: [{ kind: 'pass_priority' }] }, currentState: { perspective: 0 } };
  assert.equal(priorityHoldReason({ ...args, holdRule: 'if_actions' }), 'checking playable actions');
  assert.equal(priorityHoldReason({ ...args, holdRule: 'never' }), null);
});

test('incremental publication preserves undo controls owned by the live runtime', () => {
  const undo = { action_ref: { kind: 'untap_land', stable_id: 22 }, index: 1 };
  const state = { __priority_revision: 4, decision: { kind: 'priority', player: 0, analysis_complete: false, actions: [undo] } };
  const analysis = { revision: 4, sequence: 1, decision: { kind: 'priority', player: 0, analysis_complete: false, actions: [{ action_ref: { kind: 'play_land', land_id: 3 }, index: 0 }] } };
  const next = mergePriorityAnalysis(state, analysis);
  assert.deepEqual(next.decision.actions[1].action_ref, undo.action_ref);
  assert.equal(mergePriorityAnalysis(next, analysis), next);
});

test('restarting an unchanged visible revision never resets publication ordering', async () => {
  const h = harness(); h.scheduler.start(0); await h.tick();
  h.workers[0].reply({ type: 'priority', sequence: 50, decision: {} });
  h.scheduler.invalidate(); h.scheduler.start(0); await h.tick();
  h.workers[0].reply({ type: 'priority', token: h.workers[0].sent.at(-1).token, sequence: 1, decision: {} });
  assert.ok(h.events[1].sequence > h.events[0].sequence);
  assert.equal(h.events[1].revision, h.events[0].revision);
});

test('publication waits until the command queue leaves a temporary branch, then rechecks staleness', async () => {
  const deliveries = [];
  const h = harness({ deliver: fn => deliveries.push(fn) });
  h.scheduler.start(); await h.tick();
  h.identity('temporary verification branch');
  h.workers[0].reply({ type: 'priority', decision: { analysis_complete: false } });
  assert.equal(h.events.length, 0);
  h.identity('A'); deliveries.shift()(); assert.equal(h.events.length, 1);
  h.workers[0].reply({ type: 'priority', decision: { analysis_complete: true } });
  h.scheduler.invalidate(); deliveries.shift()(); assert.equal(h.events.length, 1);
});

test('a fresh base snapshot of the same revision can restore the latest partial menu', () => {
  const base = { __priority_revision: 3, decision: { kind: 'priority', player: 0, analysis_complete: false, actions: [] } };
  const analysis = { revision: 3, sequence: 4, decision: { ...base.decision, actions: ['land'] } };
  const enriched = mergePriorityAnalysis(base, analysis);
  const refreshed = { ...enriched, decision: base.decision };
  assert.deepEqual(mergePriorityAnalysis(refreshed, analysis).decision.actions, ['land']);
});


test('idle and cooperative busy workers survive invalidation, while dispose frees them', async () => {
  const h = harness(); h.scheduler.start(); await h.tick();
  const worker = h.workers[0];
  worker.reply({ type: 'idle' });
  h.scheduler.invalidate(); assert.equal(worker.terminated, false);
  h.identity('B'); h.scheduler.start(); await h.tick();
  assert.equal(h.workers.length, 1);
  const token = worker.sent.at(-1).token;
  worker.reply({ type: 'priority', decision: {} }); // old token
  assert.equal(h.events.length, 0);
  worker.reply({ type: 'idle', token });
  const request = h.scheduler.inspector(1n, 0);
  h.scheduler.invalidate(); assert.equal(worker.terminated, false);
  assert.equal(worker.sent.at(-1).type, 'cancel');
  assert.deepEqual(await request, []);
  h.scheduler.start(); await h.tick();
  worker.reply({ type: 'available', cancelSerial: worker.sent.findLast(m => m.type === 'cancel').serial });
  worker.reply({ type: 'idle', token: worker.sent.at(-1).token });
  h.scheduler.dispose(); assert.equal(worker.terminated, true);
  assert.equal(h.timers.size, 0);
});

test('an idle notification queued before an inspector request cannot suppress cancellation', async () => {
  const deliveries = [];
  const h = harness({ deliver: fn => deliveries.push(fn) });
  h.scheduler.start(); await h.tick();
  const worker = h.workers[0];
  worker.reply({ type: 'idle' });
  const request = h.scheduler.inspector(8n, 0);
  deliveries.shift()();
  h.scheduler.invalidate();
  assert.equal(worker.sent.at(-1).type, 'cancel');
  assert.deepEqual(await request, []);
});


test('a stuck synchronous search is terminated and restarted after the cancellation deadline', async () => {
  const h = harness(); h.scheduler.start(7); await h.tick();
  const stuck = h.workers[0];
  stuck.reply({ type: 'phase', phase: 'search' });
  h.scheduler.invalidate(); h.identity('B'); h.scheduler.start(8); await h.tick();
  assert.equal(stuck.terminated, false);
  assert.equal([...h.timers.values()][0].delay, 100);
  await h.tick(); // Cancellation deadline, no safe-boundary acknowledgement.
  assert.equal(stuck.terminated, true);
  await h.tick();
  h.workers[1].reply({ type: 'priority', decision: {} });
  assert.equal(h.events[0].revision, 8);
  stuck.reply({ type: 'priority', decision: {} });
  assert.equal(h.events.length, 1);
});

test('initializing workers get time to retain their registry and acknowledge obsolete work', async () => {
  const h = harness(); h.scheduler.start(); await h.tick();
  const worker = h.workers[0];
  h.scheduler.invalidate(); h.identity('B'); h.scheduler.start(); await h.tick();
  assert.equal([...h.timers.values()][0].delay, 5000);
  worker.reply({ type: 'available', cancelSerial: worker.sent.findLast(m => m.type === 'cancel').serial }); // Old token is deliberately accepted.
  assert.equal(h.timers.size, 0);
  assert.equal(worker.terminated, false);
  worker.reply({ type: 'priority', token: worker.sent.at(-1).token, decision: {} });
  assert.equal(h.events.length, 1);
});


test('an acknowledgement sent before a newer cancellation cannot disarm its deadline', async () => {
  const h = harness(); h.scheduler.start(); await h.tick();
  const worker = h.workers[0];
  worker.reply({ type: 'phase', phase: 'search' });
  h.scheduler.invalidate();
  const firstSerial = worker.sent.at(-1).serial;
  h.scheduler.start(); await h.tick();
  h.scheduler.invalidate();
  worker.reply({ type: 'available', cancelSerial: firstSerial });
  assert.equal(h.timers.size, 1);
  await h.tick();
  assert.equal(worker.terminated, true);
});

test('snapshot publication reconciles completed analysis before React renders', async () => {
  const { subscribePriorityAnalysisSnapshots } = await import('../src/lib/priority-analysis-scheduler.js');
  const jobs = [], stateListeners = new Set(), analysisListeners = new Set();
  let state = { __priority_revision: 3, decision: { kind: 'priority', player: 1, analysis_complete: false, actions: [] } };
  let latest = null, writes = 0;
  const publish = next => { state = next; writes++; for (const callback of stateListeners) callback(); };
  const game = { latestPriorityAnalysis: () => latest,
    subscribePriorityAnalysis: callback => { analysisListeners.add(callback); return () => analysisListeners.delete(callback); } };
  const dispose = subscribePriorityAnalysisSnapshots({ game, getState: () => state, setState: publish,
    subscribeState: callback => { stateListeners.add(callback); return () => stateListeners.delete(callback); },
    schedule: callback => jobs.push(callback) });
  const drain = () => { let count = 0; while (jobs.length) { assert.ok(++count < 10, 'reconciliation must converge'); jobs.shift()(); } };
  drain();
  latest = { revision: 4, sequence: 10, decision: { kind: 'priority', player: 1, analysis_complete: true, actions: ['pass', 'cast'] } };
  for (const callback of analysisListeners) callback(latest);
  drain();
  assert.equal(state.__priority_revision, 3, 'future analysis cannot replace an older visible state');
  const base = { __priority_revision: 4, decision: { kind: 'priority', player: 1, analysis_complete: false, actions: ['pass'] } };
  publish(base);
  // Existing publishers can finish assigning their base reference after setState.
  state = base;
  drain();
  assert.equal(state.decision.analysis_complete, true);
  assert.deepEqual(state.decision.actions, ['pass', 'cast']);
  publish(base); drain();
  assert.deepEqual(state.decision.actions, ['pass', 'cast'], 'a delayed base publication must not strand the completed menu');
  const beforeDispose = writes;
  publish(base); dispose(); drain();
  assert.equal(writes, beforeDispose + 1, 'disposed subscriptions cannot publish queued work');
  assert.equal(stateListeners.size, 0); assert.equal(analysisListeners.size, 0);
});

test('snapshot reconciliation rejects obsolete analysis after a priority change', async () => {
  const { subscribePriorityAnalysisSnapshots } = await import('../src/lib/priority-analysis-scheduler.js');
  const jobs = [];
  let state = { __priority_revision: 7, decision: { kind: 'priority', player: 0, analysis_complete: false } };
  const latest = { revision: 7, sequence: 2, decision: { kind: 'priority', player: 0, analysis_complete: true, actions: ['old'] } };
  const dispose = subscribePriorityAnalysisSnapshots({
    game: { latestPriorityAnalysis: () => latest, subscribePriorityAnalysis: () => () => {} },
    getState: () => state, setState: () => assert.fail('stale analysis published'),
    subscribeState: () => () => {}, schedule: callback => jobs.push(callback),
  });
  state = { __priority_revision: 8, decision: { kind: 'priority', player: 1, analysis_complete: false } };
  jobs.shift()(); dispose();
});
