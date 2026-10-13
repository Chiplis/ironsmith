import test from 'node:test';
import assert from 'node:assert/strict';
import {imageCombineMode} from '../src/lib/mse/image-combine-mode.mjs';
import {MseImage} from '../src/lib/mse/script-values.mjs';
import {MseRuntime} from '../src/lib/mse/script-runtime.mjs';
import {imageBuiltins} from '../src/lib/mse/image-builtins.mjs';
import {MseImageRenderer} from '../src/lib/mse/image-renderer.mjs';

test('combine metadata follows native filter and blend inheritance',()=>{
  const base=new MseImage('asset',{path:'a'}),marked=new MseImage('set-combine',{image:base,mode:'multiply'});
  assert.equal(imageCombineMode(new MseImage('alpha',{image:marked})),'multiply');
  assert.equal(imageCombineMode(new MseImage('set-combine',{image:marked,mode:'default'})),'default');
  assert.equal(imageCombineMode(new MseImage('combine',{image1:marked,image2:base,mode:'add'})),'multiply');
  assert.equal(imageCombineMode(new MseImage('masked-blend',{light:base,dark:marked,mask:marked})),'default');
  assert.equal(imageCombineMode(new MseImage('stroke',{image:marked})),'default');
});
test('set_combine preserves image pixels and rejects unknown blend names',async()=>{
  const runtime=new MseRuntime({builtins:imageBuiltins('example.mse-style')});
  const graph=runtime.run('set_combine("frame.png",combine:"multiply")');
  assert.equal(imageCombineMode(graph),'multiply');
  const data=new Uint8ClampedArray([10,20,30,128]);
  const rendered=await new MseImageRenderer({loadAsset:async()=>({width:1,height:1,data,hasAlpha:true})}).render(graph);
  assert.deepEqual([...rendered.data],[10,20,30,128]);assert.notEqual(rendered.data,data);
  assert.throws(()=>runtime.run('set_combine("frame.png",combine:"unknown")'));
});
