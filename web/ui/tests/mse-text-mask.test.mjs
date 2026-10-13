import test from 'node:test';
import assert from 'node:assert/strict';
import { createTextMask } from '../src/lib/mse/text-mask.mjs';
import { wrapTextCharacters } from '../src/lib/mse/text-wrap.mjs';
const data = new Uint8ClampedArray([
  0,0,0,255, 0,0,0,255, 0,0,0,255, 0,0,0,255,
  127,255,255,255, 128,0,0,0, 255,0,0,255, 255,0,0,255,
  255,0,0,255, 255,0,0,255, 255,0,0,255, 255,0,0,255,
]);
test('mask rows use red threshold, native last-pixel right bounds and field scaling', () => {
  const bounds = createTextMask({width:4,height:3,data},40,30);
  assert.deepEqual(bounds(0), {left:40,right:0});
  assert.deepEqual(bounds(10), {left:10,right:30});
  assert.deepEqual(bounds(20), {left:0,right:30});
  assert.deepEqual(bounds(-1), {left:0,right:40});
  assert.deepEqual(bounds(30), {left:0,right:40});
});
test('masked wrapping skips blocked rows and uses each new line bounds', () => {
  const rowBounds = createTextMask({width:4,height:3,data},40,30);
  const chars = [...'A B'].map(c=>({width:10,height:10,break:c===' '?'space':'maybe'}));
  const result = wrapTextCharacters(chars,{width:40,height:30,rowBounds});
  assert.equal(result.fits,true);
  assert.deepEqual(result.lines.map(l=>[l.top,l.positions[0],l.maskRight]),[[10,10,30],[20,0,30]]);
});
