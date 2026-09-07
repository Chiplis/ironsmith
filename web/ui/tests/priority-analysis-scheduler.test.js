import test from "node:test";
import assert from "node:assert/strict";
import { createPriorityAnalysisScheduler, mergePriorityAnalysis } from "../src/lib/priority-analysis-scheduler.js";
import { priorityHoldReason } from "../src/lib/priority-automation.js";

function harness() {
  const timers = new Map(); let id = 0; const events = []; let busy = false; let steps = 0;
  const engine = { beginPriorityAnalysis: () => true, cancelPriorityAnalysis() {},
    stepPriorityAnalysis: () => ++steps === 1 ? null : { kind: "priority", player: 0, analysis_complete: true, actions: [] } };
  const scheduler = createPriorityAnalysisScheduler({ game: () => engine, busy: () => busy,
    enqueue: (fn) => Promise.resolve().then(fn), publish: (value) => events.push(value), fail: assert.fail,
    schedule: (fn) => { timers.set(++id, fn); return id; }, cancel: (key) => timers.delete(key) });
  const tick = async () => { const [key, fn] = timers.entries().next().value; timers.delete(key); fn(); await new Promise(setImmediate); };
  return { scheduler, events, timers, tick, busy: (value) => { busy = value; }, steps: () => steps };
}

test("analysis yields between slices and gives queued commands precedence", async () => {
  const h = harness(); h.scheduler.start(); h.busy(true); await h.tick(); assert.equal(h.steps(), 0);
  h.busy(false); await h.tick(); assert.equal(h.steps(), 1); assert.equal(h.events.length, 0);
  await h.tick(); assert.equal(h.events.length, 1); assert.equal(h.timers.size, 0);
});
test("a pass cancels the old search before another slice can publish", async () => {
  const h = harness(); h.scheduler.start(); await h.tick(); h.scheduler.invalidate();
  assert.equal(h.timers.size, 0); assert.equal(h.events.length, 0);
  h.scheduler.start(); await h.tick(); assert.equal(h.events[0].revision, 1);
});
test("stale results and different players cannot enrich the current menu", () => {
  const state = { __priority_revision: 3, decision: { kind: "priority", player: 0, analysis_complete: false } };
  assert.equal(mergePriorityAnalysis(state, { revision: 2, decision: { player: 0 } }), state);
  assert.equal(mergePriorityAnalysis(state, { revision: 3, decision: { player: 1 } }), state);
  const decision = { kind: "priority", player: 0, analysis_complete: true };
  assert.equal(mergePriorityAnalysis(state, { revision: 3, decision }).decision, decision);
});
test("pending menus cannot prove hold-if-actions may pass; unconditional yield remains possible", () => {
  const args = { autoPassEnabled: true, decision: { kind: "priority", player: 0, analysis_complete: false, actions: [{ kind: "pass_priority" }] }, currentState: { perspective: 0 } };
  assert.equal(priorityHoldReason({ ...args, holdRule: "if_actions" }), "checking playable actions");
  assert.equal(priorityHoldReason({ ...args, holdRule: "never" }), null);
  assert.equal(priorityHoldReason({ ...args, holdRule: "if_actions", decision: { ...args.decision, analysis_complete: true } }), null);
});

test("a cancelled job can resume for an unchanged visible snapshot after a rejected command", async () => {
  const h = harness(); h.scheduler.start(); await h.tick(); h.scheduler.invalidate();
  h.scheduler.start(0); await h.tick(); assert.equal(h.events[0].revision, 0);
});

test("inspector slices share scheduling, deduplicate requests, and cancel without blocking commands", async () => {
  const timers = new Map(); let id = 0; let busy = false; let steps = 0;
  const engine = { beginPriorityAnalysis: () => false, cancelPriorityAnalysis() {},
    beginInspectorAnalysis() {}, stepInspectorAnalysis() { steps++; return null; } };
  const scheduler = createPriorityAnalysisScheduler({ game: () => engine, busy: () => busy,
    enqueue: fn => Promise.resolve().then(fn), publish: assert.fail, fail: assert.fail,
    schedule: fn => { timers.set(++id, fn); return id; }, cancel: key => timers.delete(key) });
  const tick = async () => { const [key, fn] = timers.entries().next().value; timers.delete(key); fn(); await new Promise(setImmediate); };
  const preview = scheduler.inspector(1n, 2);
  assert.equal(scheduler.inspector(1n, 2), preview);
  busy = true; await tick(); assert.equal(steps, 0);
  busy = false; await tick(); assert.equal(steps, 1);
  scheduler.invalidate(); assert.deepEqual(await preview, []);
  assert.equal(timers.size, 0);
});

test("completed inspector results are reused until game invalidation", async () => {
  let callback; let steps = 0;
  const result = [{ mana_payment_available: false }];
  const engine = { beginPriorityAnalysis: () => false, cancelPriorityAnalysis() {},
    beginInspectorAnalysis() {}, stepInspectorAnalysis() { steps++; return result; } };
  const scheduler = createPriorityAnalysisScheduler({ game: () => engine, busy: () => false,
    enqueue: fn => Promise.resolve().then(fn), publish: assert.fail, fail: assert.fail,
    schedule: fn => { callback = fn; return 1; }, cancel() {} });
  const first = scheduler.inspector(1n, 2); callback();
  assert.deepEqual(await first, result);
  assert.deepEqual(await scheduler.inspector(1n, 2), result); assert.equal(steps, 1);
  scheduler.invalidate(); const next = scheduler.inspector(1n, 2); callback();
  assert.deepEqual(await next, result); assert.equal(steps, 2);
});
