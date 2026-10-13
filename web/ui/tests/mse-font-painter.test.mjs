import test from 'node:test';
import assert from 'node:assert/strict';
import {decorateTextRaster,prepareFontDrawing} from '../src/lib/mse/font-painter.mjs';
const mask={width:1,height:1,data:new Uint8ClampedArray([255,255,255,255]),hasAlpha:true};
test('text effects compose the glyph inside its stroke and avoid double painting equal colors',()=>{
  const result=decorateTextRaster(mask,[255,0,0,128],[0,0,255,255],1,0);
  assert.equal(result.margin,1);assert.equal(result.raster.width,3);
  assert.deepEqual([...result.raster.data.slice(16,20)],[128,0,127,255]);
  const equal=decorateTextRaster(mask,[10,20,30,128],[10,20,30,128],1,0);
  assert.equal(equal.raster.data[19],128);
  assert.equal(decorateTextRaster(mask,[1,2,3,0],[0,0,0,255],1,0),null);
  assert.deepEqual([...mask.data],[255,255,255,255]);
});
test('fonts without bitmap effects preserve the provider including line decorations',async()=>{
  const provider={draw(){}};
  assert.equal(await prepareFontDrawing({}, {prepare:async()=>provider}),provider);
  assert.equal(await prepareFontDrawing({underline:true,strikethrough:true},{prepare:async()=>provider}),provider);
});
