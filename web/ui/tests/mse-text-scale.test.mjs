import test from 'node:test';
import assert from 'node:assert/strict';
import { selectTextScale } from '../src/lib/mse/text-scale.mjs';
const evaluate = limit => scale => ({ fits: 100 * scale <= limit, lines: [{ top: 0, height: 100 * scale }], measuredAt: scale });
test('scale search chooses a fitting scale, remeasures, and honors minimum size', () => {
  const fit = selectTextScale(evaluate(50), { minScale: 0.25, scaleStep: 0.02, height: 50 });
  assert.equal(fit.scale, 0.5);
  assert.equal(fit.measuredAt, fit.scale);
  assert.equal(fit.fits, true);
  const forced = selectTextScale(evaluate(50), { minScale: 0.75, scaleStep: 0.02, height: 50 });
  assert.equal(forced.scale, 0.75);
  assert.equal(forced.fits, false);
});
test('disabled shrinking and previous-scale fast path avoid unnecessary search', () => {
  const calls = [];
  const measure = scale => { calls.push(scale); return evaluate(100)(scale); };
  assert.equal(selectTextScale(measure, { minScale: 1, scaleStep: 0.02, height: 100 }).scale, 1);
  assert.deepEqual(calls, [1]);
  const result = selectTextScale(evaluate(50), { minScale: 0.25, scaleStep: 0.02, previousScale: 0.5, height: 50 });
  assert.equal(result.scale, 0.5);
});
