import test from 'node:test';
import assert from 'node:assert/strict';
import { effectPixelRatio, normalizeGraphics } from '../src/lib/graphics-quality.js';
test('missing and invalid saved graphics settings default to High', () => {
  for (const value of [null, undefined, '', 'ultra']) assert.equal(normalizeGraphics(value), 'high');
  for (const value of ['off', 'low', 'high']) assert.equal(normalizeGraphics(value), value);
});
test('decorative buffers respect pixel budgets on high density and large displays', () => {
  for (const [width,height,dpr] of [[1440,900,2],[3840,2160,3],[1920,1080,1.5]]) {
    for (const quality of ['low','high']) {
      const ratio = effectPixelRatio(width,height,dpr,quality);
      assert.ok(width*height*ratio*ratio <= (quality === 'high' ? 4e6 : 1e6) + 1);
      assert.ok(ratio <= dpr);
    }
  }
  assert.equal(effectPixelRatio(1280,800,1,'high'),1);
});
