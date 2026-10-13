import test from 'node:test';
import assert from 'node:assert/strict';
import {drawCombinedLayer} from '../src/lib/mse/field-blend.mjs';
import {imageCombineModes,combineChannel} from '../src/lib/mse/image-combine.mjs';
test('field blending captures backdrop channels and retains foreground alpha for compositing',()=>{
  const back=new Uint8ClampedArray([100,150,200,255,12,34,56,255]);
  const front=new Uint8ClampedArray([128,64,255,127,255,255,255,0]);
  for(const mode of imageCombineModes){
    let pixels,draws=0;
    const layer={width:2,height:1,getContext:()=>({getImageData:()=>({data:front})})};
    const canvas={getContext:()=>({createImageData:()=>({data:new Uint8ClampedArray(8)}),putImageData:p=>{pixels=p.data;}})};
    const target={getImageData:()=>({data:back}),drawImage:(c,x,y)=>{assert.equal(c,canvas);assert.equal(x,0);assert.equal(y,0);draws++;}};
    drawCombinedLayer(target,layer,mode,()=>canvas);
    const fn=combineChannel(mode);
    assert.deepEqual([...pixels],[fn(100,128),fn(150,64),fn(200,255),127,fn(12,255),fn(34,255),fn(56,255),0],mode);
    assert.equal(draws,1);
  }
  assert.deepEqual([...back],[100,150,200,255,12,34,56,255]);
  assert.deepEqual([...front],[128,64,255,127,255,255,255,0]);
});
