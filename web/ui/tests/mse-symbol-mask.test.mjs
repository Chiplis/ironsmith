import test from 'node:test';
import assert from 'node:assert/strict';
import {combineSymbolMasks} from '../src/lib/mse/symbol-mask.mjs';
const rgba=values=>new Uint8ClampedArray(values.flatMap(v=>[255,255,255,v?255:0]));
test('symbol Boolean modes keep border and interior masks distinct',()=>{
  const fill=rgba([1,1,0,0]),outline=rgba([1,1,1,0]),stroke=rgba([0,1,1,0]);
  const expected={merge:[[1,1,1,1],[1,1,1,1]],overlap:[[1,1,1,1],[1,1,1,1]],border:[[1,1,1,1],[1,0,1,1]],subtract:[[0,1,1,1],[0,0,1,1]],intersection:[[1,1,1,0],[1,0,0,0]],difference:[[0,0,1,1],[0,1,1,1]]};
  for(const [mode,[b,i]] of Object.entries(expected)){
    const border=new Uint8Array([1,1,1,1]),inside=new Uint8Array([1,0,1,1]);
    combineSymbolMasks(border,inside,fill,outline,stroke,mode);
    assert.deepEqual([...border],b,mode);assert.deepEqual([...inside],i,mode);
  }
});
test('border-only shapes work without stroke width and leave interiors untouched',()=>{
  const border=new Uint8Array(2),inside=new Uint8Array([0,1]);
  combineSymbolMasks(border,inside,rgba([1,1]),null,null,'border');
  assert.deepEqual([...border],[1,1]);assert.deepEqual([...inside],[0,1]);
});
