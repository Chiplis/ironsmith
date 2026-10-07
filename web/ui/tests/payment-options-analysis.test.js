import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { createPaymentOptionsAnalysis, paymentOptionsKey, mergePaymentOptions } from '../src/lib/payment-options-analysis.js';
import { manaPaymentActionMap, manaActivationCommand } from '../src/lib/mana-payment-actions.js';

test('failed options loading preserves authoritative land-click activations', async () => {
  const source = { source_id: '10', ability_index: 0, source_name: 'Mountain', label: '{T}: Add {R}.' };
  const stateRef = { current: { perspective: 0, __priority_revision: 1,
    decision: { kind: 'mana_payment', player: 0 },
    mana_payment: { request_hash: 'r', plan_id: 'p', can_confirm: false,
      activation_options_complete: false, activation_options: [], mana_abilities: [source] },
  } };
  let updated;
  const code = readFileSync(new URL('../src/hooks/usePaymentOptions.js', import.meta.url), 'utf8')
    .replace(/^import[^\n]+\n/gm, '').replace('export function', 'function');
  const context = { useRef: () => ({ current: null }), useEffect: effect => effect(),
    paymentOptionsKey, mergePaymentOptions };
  vm.runInNewContext(code, context);
  context.usePaymentOptions({ state: stateRef.current, stateRef, setState: value => { updated = value; },
    game: { getPaymentActivationOptions: async () => { throw new Error('Worker unavailable'); } } });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(updated.mana_payment.activation_options_error, true);
  assert.equal(updated.mana_payment.can_confirm, false);
  const action = manaPaymentActionMap(updated).get(10)?.[0];
  assert.ok(action, 'Mountain remains clickable after the options failure');
  assert.deepEqual(manaActivationCommand(action), { type: 'mana_payment',
    response: { action: 'activate', source_id: '10', ability_index: 0 } });
});

function runPaymentOptionsHook(responses) {
  const stateRef = { current: { perspective: 0, __priority_revision: 1,
    decision: { kind: 'mana_payment', player: 0 },
    mana_payment: { request_hash: 'r', plan_id: 'p', can_confirm: true, activation_options_complete: false },
  } };
  const updates = [];
  let calls = 0;
  const code = readFileSync(new URL('../src/hooks/usePaymentOptions.js', import.meta.url), 'utf8')
    .replace(/^import[^\n]+\n/gm, '').replace(/^export function/m, 'function');
  const context = { useRef: () => ({ current: null }), useEffect: effect => effect(),
    paymentOptionsKey, mergePaymentOptions };
  vm.runInNewContext(code, context);
  context.usePaymentOptions({ state: stateRef.current, stateRef, setState: value => { updates.push(value); },
    game: { getPaymentActivationOptions: async () => responses[Math.min(calls++, responses.length - 1)] } });
  return { updates, calls: () => calls };
}

test('a cancelled options job is retried instead of loading forever', async () => {
  const { updates, calls } = runPaymentOptionsHook([null, { activation_options: [{ source_id: '2' }] }]);
  for (let i = 0; i < 5; i++) await tick();
  assert.equal(calls(), 2);
  assert.equal(updates.at(-1).mana_payment.activation_options_complete, true);
  assert.equal(updates.at(-1).mana_payment.activation_options_error, undefined);
});

test('an options request that stays unavailable reports an error', async () => {
  const { updates, calls } = runPaymentOptionsHook([null]);
  for (let i = 0; i < 5; i++) await tick();
  assert.equal(calls(), 2);
  assert.equal(updates.at(-1).mana_payment.activation_options_complete, true);
  assert.equal(updates.at(-1).mana_payment.activation_options_error, true);
  assert.equal(updates.at(-1).mana_payment.can_confirm, true);
});

function fixture(capture = async () => ({ request: '{}' }), options = {}) {
  const workers = [];
  const analysis = createPaymentOptionsAnalysis({ capture, ...options, createWorker: () => {
    const worker = { terminated: false, messages: [], postMessage(input) {
      this.messages.push(input);
      if (input.type !== 'cancel') this.input = input;
    }, terminate() { this.terminated = true; } };
    workers.push(worker); return worker;
  } });
  return { analysis, workers };
}
function manualTimers() {
  const timers = [];
  return { timers, schedule: fn => { timers.push(fn); return timers.length; }, clearSchedule: id => { timers[id - 1] = null; },
    fire() { for (const timer of timers.splice(0)) timer?.(); } };
}
const tick = () => new Promise(resolve => setImmediate(resolve));

test('a pending alternatives search does not disable the valid proposal', async () => {
  const { analysis, workers } = fixture();
  const proposal = { __priority_revision: 1, mana_payment: {
    request_hash: 'r', plan_id: 'p', can_confirm: true, planned_sources: [{ source_id: '1' }],
    activation_options_complete: false,
  } };
  const key = paymentOptionsKey(proposal);
  const pending = analysis.run();
  await tick();
  assert.equal(proposal.mana_payment.can_confirm, true);
  workers[0].onmessage({ data: { token: workers[0].input.token, result: { activation_options: [{ source_id: '2' }], mana_abilities: [] } } });
  const merged = mergePaymentOptions(proposal, key, await pending);
  assert.equal(merged.mana_payment.can_confirm, true);
  assert.equal(merged.mana_payment.planned_sources, proposal.mana_payment.planned_sources);
  assert.equal(merged.mana_payment.activation_options_complete, true);
  assert.equal(workers[0].terminated, false);
  analysis.dispose();
  assert.equal(workers[0].terminated, true);
});

test('Pay or Cancel settles at once but keeps the caught-up worker; late replies cannot apply', async () => {
  const { analysis, workers } = fixture();
  const pending = analysis.run(); await tick();
  const token = workers[0].input.token;
  analysis.cancel();
  assert.equal(await pending, null);
  assert.equal(workers[0].terminated, false);
  assert.deepEqual(workers[0].messages.at(-1), { type: 'cancel', token });
  workers[0].onmessage({ data: { token, result: ['obsolete'] } });
  const next = analysis.run(); await tick();
  assert.equal(workers.length, 1, 'the next search reuses the replica instead of replaying the session');
  workers[0].onmessage({ data: { token: workers[0].input.token, result: ['fresh'] } });
  assert.deepEqual(await next, ['fresh']);
});

test('a cancelled job stuck in synchronous work is replaced and the newest request resubmitted', async () => {
  const timers = manualTimers();
  let captures = 0;
  const { analysis, workers } = fixture(async () => { captures++; return { request: '{}' }; },
    { schedule: timers.schedule, clearSchedule: timers.clearSchedule });
  analysis.run(); await tick();
  const next = analysis.run(); await tick();
  assert.equal(timers.timers.length, 1);
  timers.fire(); await tick();
  assert.equal(workers[0].terminated, true);
  assert.equal(workers.length, 2);
  assert.equal(captures, 3, 'the replacement re-captures instead of resending transferred seed memory');
  workers[1].onmessage({ data: { token: workers[1].input.token, result: ['recovered'] } });
  assert.deepEqual(await next, ['recovered']);
});

test('an acknowledged cancellation disarms the stuck-worker watchdog', async () => {
  const timers = manualTimers();
  const { analysis, workers } = fixture(undefined, { schedule: timers.schedule, clearSchedule: timers.clearSchedule });
  analysis.run(); await tick();
  const first = workers[0].input.token;
  analysis.run(); await tick();
  workers[0].onmessage({ data: { token: first, cancelled: true } });
  timers.fire();
  assert.equal(workers[0].terminated, false);
});

test('capture learns how far the worker replica has caught up, even from a cancelled job', async () => {
  const seen = [];
  const { analysis, workers } = fixture(async (_, { replicaMark }) => {
    seen.push(replicaMark);
    return { request: '{}', localReplay: { epoch: 1, operations: [] } };
  });
  analysis.run(); await tick();
  const first = workers[0].input.token;
  analysis.cancel();
  workers[0].onmessage({ data: { token: first, cancelled: true, replicaMark: { epoch: 1, position: 5 } } });
  analysis.run(); await tick();
  assert.deepEqual(seen, [null, { epoch: 1, position: 5 }]);
});

test('cancellation during checkpoint capture prevents worker startup', async () => {
  let complete;
  const { analysis, workers } = fixture(() => new Promise(resolve => { complete = resolve; }));
  const pending = analysis.run();
  analysis.cancel(); complete({ request: '{}' });
  assert.equal(await pending, null);
  assert.equal(workers.length, 0);
});

test('options from a different plan or state revision are discarded', () => {
  const state = { __priority_revision: 1, mana_payment: { request_hash: 'r', plan_id: 'p' } };
  const key = paymentOptionsKey(state);
  for (const next of [{ ...state, __priority_revision: 2 },
    { ...state, mana_payment: { ...state.mana_payment, plan_id: 'new' } },
    { ...state, mana_payment: null }]) {
    assert.equal(mergePaymentOptions(next, key, { activation_options: [] }), next);
  }
});

test('worker failure rejects analysis and releases the runtime', async () => {
  const { analysis, workers } = fixture();
  const pending = analysis.run();
  const rejected = assert.rejects(pending, /failed/);
  await tick(); workers[0].onerror({ message: 'failed' });
  await rejected; assert.equal(workers[0].terminated, true);
});


test('completed runtime survives state changes, but obsolete tokens cannot finish its next request', async () => {
  const { analysis, workers } = fixture();
  const first = analysis.run(); await tick();
  const oldToken = workers[0].input.token;
  workers[0].onmessage({ data: { token: oldToken, result: ['first'] } });
  assert.deepEqual(await first, ['first']);
  analysis.cancel();
  assert.equal(workers[0].terminated, false);
  let settled = false;
  const second = analysis.run().then(value => { settled = true; return value; }); await tick();
  assert.equal(workers.length, 1);
  workers[0].onmessage({ data: { token: oldToken, result: ['stale'] } }); await tick();
  assert.equal(settled, false);
  workers[0].onmessage({ data: { token: workers[0].input.token, result: ['second'] } });
  assert.deepEqual(await second, ['second']);
  analysis.dispose();
  assert.equal(workers[0].terminated, true);
});
