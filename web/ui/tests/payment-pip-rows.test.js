import test from 'node:test';
import assert from 'node:assert/strict';
import { paymentPipRows, paymentOptionsForPip, selectPaymentPipSource } from '../src/lib/payment-pip-rows.js';
import { paymentPreferences } from '../src/lib/payment-draft.js';
const source = (id, pool, max = 1) => ({ source_id: id, source_name: id, ability_index: 0, payment_kind: 'mana_ability', expected_mana: pool, repeatable: max > 1, max_activations: max });
const draft = paymentPreferences();
test('cost icons stay generic even when paid with colored mana; color requirements get priority', () => {
  const payment = { pips: [['1'], ['B']], planned_sources: [source('Swamp', { black: 1 }), source('Island', { blue: 1 })] };
  const rows = paymentPipRows(payment, draft);
  assert.deepEqual(rows.map(row => row.pip), [['1'], ['B']]);
  assert.deepEqual(rows.map(row => row.source.source_id), ['Island', 'Swamp']);
});
test('Wall of Roots and other once-only sources disappear from other pip pickers', () => {
  const wall = source('Wall of Roots', { green: 1 });
  const island = source('Island', { blue: 1 });
  const payment = { pips: [['2']], planned_sources: [wall, island], activation_options: [wall, island] };
  const rows = paymentPipRows(payment, draft);
  assert.deepEqual(paymentOptionsForPip(payment, draft, rows[1], rows).map(s => s.source_id), ['Island']);
  assert.equal(paymentOptionsForPip(payment, draft, rows[0], rows).some(s => s.source_id === wall.source_id), true);
});
test('finite reusable capacity is shared across color-output branches', () => {
  const limited = source('Battery', { blue: 1 }, 2);
  const red = { ...limited, expected_mana: { red: 1 }, color_restriction: ['red'] };
  const other = source('Land', { white: 1 });
  const payment = { pips: [['3']], planned_sources: [limited, limited, other], activation_options: [limited, red, other] };
  const rows = paymentPipRows(payment, draft);
  assert.deepEqual(paymentOptionsForPip(payment, draft, rows[2], rows).map(s => s.source_id), ['Land']);
  assert.equal(paymentOptionsForPip(payment, draft, rows[0], rows).some(s => s.source_id === 'Battery'), true);
});
test('multi-mana output covers multiple pips with one activation and may fill a spare pip', () => {
  const ring = source('Sol Ring', { colorless: 2 });
  const payment = { pips: [['3']], planned_sources: [ring], activation_options: [ring] };
  const rows = paymentPipRows(payment, draft);
  assert.deepEqual(rows.map(row => row.kind), ['source', 'source', 'unassigned']);
  assert.equal(rows[0].source, rows[1].source);
  assert.equal(paymentOptionsForPip(payment, draft, rows[2], rows).length, 0);
  const two = { ...payment, planned_sources: [ring], pips: [['2']], pool_before: { blue: 1 } };
  const partial = paymentPipRows(two, draft);
  assert.equal(paymentOptionsForPip(two, draft, partial[0], partial).length, 1);
  const selected = selectPaymentPipSource(draft, ring, partial[0], partial);
  assert.equal(selected.required_activations.length, 1);
});
test('a new use of a repeatable source pins its previous automatic use as well', () => {
  const battery = source('Battery', { blue: 1 }, 2);
  const land = source('Land', { white: 1 });
  const payment = { pips: [['2']], planned_sources: [battery, land], activation_options: [battery, land] };
  const rows = paymentPipRows(payment, draft);
  const selected = selectPaymentPipSource(draft, battery, rows[1], rows);
  assert.equal(selected.required_activations.length, 2);
  assert.deepEqual(selected.required_activations.map(s => s.source_id), ['Battery', 'Battery']);
  assert.deepEqual(selected.excluded_source_ids, ['Land']);
});
test('floating mana, life and missing pips remain visible when there are no activations', () => {
  const payment = { pips: [['3']], pool_before: { blue: 1 }, allocations: [{ pip_id: 1, payment_kind: 'life', life: 2 }] };
  assert.deepEqual(paymentPipRows(payment, draft).map(row => row.kind), ['pool', 'life', 'unassigned']);
});


test('choosing life directly replaces a source without a remove-source action', () => {
  const swamp = source('Swamp', {black:1});
  const payment = {pips:[['B','P']],planned_sources:[swamp],activation_options:[swamp]};
  const pinned = paymentPreferences({required_activations:[{source_id:'Swamp',ability_index:0,color_restriction:null}]});
  const rows = paymentPipRows(payment,pinned);
  const next = selectPaymentPipSource(pinned,{payment_kind:'life'},rows[0],rows);
  assert.deepEqual(next.required_life_pips,[0]);
  assert.deepEqual(next.required_activations,[]);
  assert.deepEqual(next.excluded_source_ids,['Swamp']);
});
