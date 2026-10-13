import test from 'node:test';
import assert from 'node:assert/strict';
import {resizeArtwork} from '../src/lib/mse/artwork-sampling.mjs';
const source={width:2,height:2,hasAlpha:true,data:new Uint8ClampedArray([100,20,40,255,200,40,80,128,250,200,100,0,80,60,40,64])};
test('artwork integer reductions reproduce wx unweighted color and alpha averages',()=>{
  assert.deepEqual([...resizeArtwork(source,1,1).data],[95,30,40,111]);
  assert.deepEqual([...resizeArtwork(source,2,2).data],[...source.data]);
});
test('artwork noninteger sizes use fixed-point nearest samples',()=>{
  const result=resizeArtwork(source,3,3);
  assert.deepEqual([...result.data].filter((_,i)=>i%4===0),[100,100,200,100,100,200,250,250,80]);
  assert.throws(()=>resizeArtwork(source,20,20,100),{code:'image-budget'});
});
