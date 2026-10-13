import test from 'node:test';
import assert from 'node:assert/strict';
import { measureTextRun } from '../src/lib/mse/text-metrics.mjs';

test('prefix measurement retains kerning, resets at newlines and marks spaces and bullets', () => {
  const calls = [];
  const metrics = { measure: text => { calls.push(text); return { width: text === 'AV' ? 17 : [...text].length * 10, height: 14 }; }, lineHeight: () => 15 };
  const font = { size: 12, scale_down_to: 8 };
  const result = measureTextRun('AV\n •😀', font, metrics, { soft: true, breakStyle: 'soft' });
  assert.deepEqual(calls, ['A','AV',' ',' •',' •😀']);
  assert.deepEqual(result.characters.map(c => c.width), [10,7,0,10,10,10]);
  assert.deepEqual(result.characters.map(c => c.break), ['maybe','maybe','soft','space','maybe','maybe']);
  assert.equal(result.characters[4].bullet, true);
  assert.equal(result.characters.every(c => c.soft), true);
  assert.equal(result.width, 30);
  assert.equal(result.minScale, 2/3);
  assert.equal(result.scaleStep, 1/48);
});
