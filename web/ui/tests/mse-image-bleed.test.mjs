import test from 'node:test';
import assert from 'node:assert/strict';
import {bleedRaster} from '../src/lib/mse/image-bleed.mjs';
import {MseRuntime} from '../src/lib/mse/script-runtime.mjs';
import {coreBuiltins} from '../src/lib/mse/script-builtins.mjs';
import {imageBuiltins} from '../src/lib/mse/image-builtins.mjs';
import {MseImageRenderer} from '../src/lib/mse/image-renderer.mjs';
const source=(w,h)=>({width:w,height:h,hasAlpha:true,data:Uint8ClampedArray.from({length:w*h*4},(_,i)=>i%4===3?255:Math.floor(i/4))});
test('bleed mirrors interior pixels, including corners, without mutating source',async()=>{
  const input=source(4,4),before=input.data.slice();
  const runtime=new MseRuntime({builtins:{...coreBuiltins(),...imageBuiltins('test.mse-style')}});
  const renderer=new MseImageRenderer({loadAsset:async()=>input});
  const result=await renderer.render(runtime.run('add_bleed_edge("frame",horizontal_size:0.25,vertical_size:0.25)'));
  assert.deepEqual([result.width,result.height],[6,6]);
  const indexes=[1,0,1,2,3,2];
  for(let y=0;y<6;y++)for(let x=0;x<6;x++)assert.equal(result.data[(y*6+x)*4],indexes[y]*4+indexes[x]);
  assert.deepEqual(input.data,before);
});
test('bleed defaults follow orientation and tiny images remain unchanged',()=>{
  assert.deepEqual([bleedRaster(source(100,200)).width,bleedRaster(source(100,200)).height],[108,214]);
  assert.deepEqual([bleedRaster(source(200,100)).width,bleedRaster(source(200,100)).height],[214,108]);
  const tiny=source(3,8);assert.equal(bleedRaster(tiny),tiny);
  assert.throws(()=>bleedRaster(source(4,4),{horizontal:1,vertical:1},50),{code:'image-budget'});
});
test('bleed composites transparent pixels onto background before mirroring',()=>{
  const input=source(4,4);input.data.fill(0);
  const result=bleedRaster(input,{horizontal:0.25,vertical:0.25,background:[20,40,60,255]});
  for(let i=0;i<result.data.length;i+=4)assert.deepEqual([...result.data.slice(i,i+4)],[20,40,60,255]);
});
