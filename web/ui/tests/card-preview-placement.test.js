import test from 'node:test';
import assert from 'node:assert/strict';
import { placeCardPreview } from '../src/lib/card-preview-placement.js';

const bounds = { left: 8, top: 100, right: 1200, bottom: 850 };
const preferred = { left: 700, top: 220 };
const height = 500;
function assertClear(result, obstacles, region = bounds) {
  assert.notEqual(result.visibility, 'hidden');
  const r = { left: result.left, top: result.top,
    right: result.left + parseFloat(result.width), bottom: result.top + parseFloat(result.height) };
  assert.ok(r.left >= region.left && r.right <= region.right + 0.001);
  assert.ok(r.top >= region.top && r.bottom <= region.bottom + 0.001);
  for (const o of obstacles) {
    assert.ok(r.right <= o.left - 8 + 0.001 || r.left >= o.right + 8 || r.bottom <= o.top - 8 || r.top >= o.bottom + 8);
  }
  assert.ok(Math.abs(parseFloat(result.width) / parseFloat(result.height) - 63 / 88) < 0.00001);
}

test('moves the whole frame away from decision buttons and payment overlays at full size', () => {
  const obstacles = [
    { left: 780, top: 620, right: 1192, bottom: 750 },
    { left: 650, top: 150, right: 1150, bottom: 650 },
  ];
  const result = placeCardPreview({ preferred, height, bounds, obstacles });
  assertClear(result, obstacles);
  assert.equal(result.height, '500px');
  assert.ok(result.left < preferred.left);
});

test('shrinks proportionally when necessary, without leaving the allowed board region', () => {
  const obstacles = [{ left: 350, right: 1200, top: 100, bottom: 850 }];
  const result = placeCardPreview({ preferred, height, bounds, obstacles });
  assertClear(result, obstacles);
  assert.ok(parseFloat(result.height) < height);
});

test('returns to the preferred size and position when the overlay disappears', () => {
  const result = placeCardPreview({ preferred, height, bounds });
  assertClear(result, []);
  assert.equal(result.height, '500px');
  assert.equal(result.left, preferred.left);
  assert.equal(result.top, preferred.top);
});

test('keeps stack frames above their floor and beside the stack rail', () => {
  const region = { left: 8, right: 1000, top: 320, bottom: 650 };
  const obstacles = [{ left: 10, right: 300, top: 320, bottom: 650 },
    { left: 600, right: 1000, top: 500, bottom: 650 }];
  assertClear(placeCardPreview({ preferred, height, bounds: region, obstacles }), obstacles, region);
});
