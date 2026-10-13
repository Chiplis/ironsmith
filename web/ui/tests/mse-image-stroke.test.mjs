import test from 'node:test';
import assert from 'node:assert/strict';
import { strokeRaster } from '../src/lib/mse/image-stroke.mjs';
const source = { width: 1, height: 1, hasAlpha: true, data: new Uint8ClampedArray([20,30,40,255]) };
test('stroke dilation has native softened corners and leaves its input intact', () => {
  const stroke = strokeRaster(source, [1,2,3,255], 1);
  assert.equal(stroke.width,3);
  assert.deepEqual([...stroke.data.filter((_,i)=>i%4===3)], [153,255,153,255,255,255,153,255,153]);
  assert.deepEqual([...source.data], [20,30,40,255]);
});
test('stroke opacity, opaque inputs, radius limits, and allocation budgets', () => {
  assert.equal(strokeRaster({...source,hasAlpha:false,data:new Uint8ClampedArray(4)},[0,0,0,127]).data[3],127);
  assert.equal(strokeRaster(source,[0,0,0,255],-3).width,1);
  assert.throws(()=>strokeRaster(source,[0,0,0,255],1000,0,40000),{code:'image-budget'});
  assert.throws(()=>strokeRaster(source,[0,0,0,255],Infinity),{code:'image-stroke'});
});
