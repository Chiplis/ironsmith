import test from 'node:test';
import assert from 'node:assert/strict';
import {visibilityRaster} from '../src/lib/mse/image-visibility.mjs';
import {MseRuntime} from '../src/lib/mse/script-runtime.mjs';
import {imageBuiltins} from '../src/lib/mse/image-builtins.mjs';
import {MseImageRenderer} from '../src/lib/mse/image-renderer.mjs';

test('visibility masks match clipped square neighborhoods for all threshold boundaries',()=>{
  const width=7,height=5,data=new Uint8ClampedArray(width*height*4);
  for(let i=0;i<width*height;i++)data.set([12,34,56,[0,1,127,254,255][i%5]],i*4);
  for(const hasAlpha of [false,true])for(const threshold of [-1,0,1,126,127,253,254,255,256])for(const radius of [-3,0,1,2,10]){
    const result=visibilityRaster({width,height,data,hasAlpha},threshold,radius);
    assert.equal(result.hasAlpha,false);
    for(let y=0;y<height;y++)for(let x=0;x<width;x++){
      let white=false;
      if(hasAlpha)for(let yy=0;yy<height;yy++)for(let xx=0;xx<width;xx++)
        if(Math.abs(xx-x)<=Math.max(0,radius)&&Math.abs(yy-y)<=Math.max(0,radius)&&data[(yy*width+xx)*4+3]<=threshold)white=true;
      assert.deepEqual([...result.data.slice((y*width+x)*4,(y*width+x+1)*4)],white?[255,255,255,255]:[0,0,0,255]);
    }
  }
});
test('visibility_mask script defaults mark partial transparency without modifying the input',async()=>{
  const data=new Uint8ClampedArray([1,2,3,254]);
  const vm=new MseRuntime({builtins:imageBuiltins('test.mse-style')});
  const graph=vm.run('visibility_mask("frame.png")');
  const renderer=new MseImageRenderer({loadAsset:async()=>({width:1,height:1,data,hasAlpha:true})});
  assert.deepEqual([...(await renderer.render(graph)).data],[255,255,255,255]);
  assert.deepEqual([...data],[1,2,3,254]);
});
