import test from "node:test";
import assert from "node:assert/strict";
import { requestInspectorPayment, inspectorPaymentDisplay } from "../src/lib/inspector-payment-cache.js";

const key = "7:2";
const action = { object_id: 7, ability_index: 2 };

test("cancelled previews never become ready and enabled on the old snapshot", async () => {
  let complete;
  const game = { inspectorActions: () => new Promise(resolve => { complete = resolve; }) };
  const entry = requestInspectorPayment(game, {}, key);
  await Promise.resolve();
  assert.equal(entry.ready, false);
  complete([]);
  await entry.promise;
  assert.equal(entry.ready, false);
});

test("missing actions and failed requests stay pending; a new snapshot retries", async () => {
  for (const result of [[], [{ ...action, ability_index: 1 }], new Error("preview failed")]) {
    let calls = 0;
    const game = { inspectorActions: async () => {
      calls++;
      if (calls > 1) return [{ ...action, mana_payment_available: false }];
      if (result instanceof Error) throw result;
      return result;
    } };
    const entry = requestInspectorPayment(game, {}, key);
    await entry.promise;
    assert.equal(entry.ready, false);
    const next = requestInspectorPayment(game, {}, key);
    await next.promise;
    assert.equal(next.ready, true);
    assert.equal(next.available, false);
  }
});

test("completed checks preserve payable, unpayable, and explicit unknown results", async () => {
  for (const available of [true, false, null, undefined]) {
    const game = { inspectorActions: async () => [{ ...action, mana_payment_available: available }] };
    const state = {};
    const entry = requestInspectorPayment(game, state, key);
    assert.equal(entry.ready, false);
    await entry.promise;
    assert.equal(entry.ready, true);
    assert.equal(entry.available, available);
    assert.equal(requestInspectorPayment(game, state, key), entry);
  }
});


test("the last completed state is visible before the new request starts and until it finishes", async () => {
  for (const previous of [true, false]) {
    let finish;
    let calls = 0;
    const game = { inspectorActions: () => ++calls === 1
      ? Promise.resolve([{ ...action, mana_payment_available: previous }])
      : new Promise(resolve => { finish = resolve; }) };
    const firstState = {};
    assert.equal(inspectorPaymentDisplay(game, firstState, key).payment_pending, true);
    await requestInspectorPayment(game, firstState, key).promise;
    const nextState = {};
    const expected = { payment_pending: false, mana_payment_available: previous };
    assert.deepEqual(inspectorPaymentDisplay(game, nextState, key), expected);
    const next = requestInspectorPayment(game, nextState, key);
    await Promise.resolve();
    assert.deepEqual(inspectorPaymentDisplay(game, nextState, key), expected);
    finish([{ ...action, mana_payment_available: !previous }]);
    await next.promise;
    assert.deepEqual(inspectorPaymentDisplay(game, nextState, key), {
      payment_pending: false, mana_payment_available: !previous,
    });
    assert.equal(inspectorPaymentDisplay(game, nextState, "8:2").payment_pending, true);
    assert.equal(inspectorPaymentDisplay({}, nextState, key).payment_pending, true);
  }
});

test("cancellation retains the completed state and older responses cannot overwrite newer results", async () => {
  const resolvers = [];
  const game = { inspectorActions: () => new Promise(resolve => resolvers.push(resolve)) };
  const old = requestInspectorPayment(game, {}, key);
  const newer = requestInspectorPayment(game, {}, key);
  await Promise.resolve();
  resolvers[1]([{ ...action, mana_payment_available: false }]);
  await newer.promise;
  resolvers[0]([{ ...action, mana_payment_available: true }]);
  await old.promise;
  const state = {};
  assert.equal(inspectorPaymentDisplay(game, state, key).mana_payment_available, false);
  const cancelled = requestInspectorPayment(game, state, key);
  await Promise.resolve();
  resolvers[2]([]);
  await cancelled.promise;
  assert.deepEqual(inspectorPaymentDisplay(game, state, key), {
    payment_pending: false, mana_payment_available: false,
  });
});
