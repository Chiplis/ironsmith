import test from 'node:test';
import assert from 'node:assert/strict';
import { improvePayment } from '../src/lib/payment-analysis.js';

test('payment improvement yields between slices and returns only a completed suggestion', async () => {
  let slices = 0, yields = 0;
  const command = { type: 'mana_payment', response: { action: 'replan' } };
  const result = await improvePayment({
    game: { beginPaymentAnalysis: async () => true, stepPaymentAnalysis: async () => ++slices === 3 ? command : null },
    token: 'one', isCurrent: () => true, yieldTask: async () => { yields++; },
  });
  assert.equal(result, command);
  assert.equal(yields, 3);
});

test('editing during an in-flight slice discards its improvement and stops further work', async () => {
  let current = true, slices = 0;
  const result = await improvePayment({
    game: { beginPaymentAnalysis: async () => true, stepPaymentAnalysis: async () => {
      slices++; current = false; return { type: 'mana_payment' };
    } },
    token: 'one', isCurrent: () => current, yieldTask: async () => {},
  });
  assert.equal(result, null);
  assert.equal(slices, 1);
});

test('editing before the next slice prevents that worker call', async () => {
  let current = true;
  const result = await improvePayment({
    game: { beginPaymentAnalysis: async () => true, stepPaymentAnalysis: () => assert.fail('cancelled work ran') },
    token: 'one', isCurrent: () => current, yieldTask: async () => { current = false; },
  });
  assert.equal(result, null);
});


test('browser ranking uses isolation and discards a suggestion after a manual edit', async () => {
  let current = true;
  const result = await improvePayment({
    game: {
      analyzePayment: async token => { assert.equal(token, 'isolated'); current = false; return { type: 'mana_payment' }; },
      beginPaymentAnalysis: () => assert.fail('ranking occupied the authoritative worker'),
    },
    token: 'isolated', isCurrent: () => current,
  });
  assert.equal(result, null);
});
