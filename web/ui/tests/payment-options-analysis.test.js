import test from 'node:test';
import assert from 'node:assert/strict';
import { createPaymentOptionsAnalysis, paymentOptionsKey, mergePaymentOptions } from '../src/lib/payment-options-analysis.js';

function fixture(capture = async () => ({ request: '{}' })) {
  const workers = [];
  const analysis = createPaymentOptionsAnalysis({ capture, createWorker: () => {
    const worker = { terminated: false, postMessage(input) { this.input = input; }, terminate() { this.terminated = true; } };
    workers.push(worker); return worker;
  } });
  return { analysis, workers };
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

test('Pay or Cancel terminates an in-flight worker; late replies cannot apply', async () => {
  const { analysis, workers } = fixture();
  const pending = analysis.run(); await tick();
  analysis.cancel();
  assert.equal(workers[0].terminated, true);
  workers[0].onmessage({ data: { token: workers[0].input.token, result: ['obsolete'] } });
  assert.equal(await pending, null);
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
