import test from 'node:test';
import assert from 'node:assert/strict';
import {locateSetSymbol} from '../src/lib/card-set-symbol.js';

test('registers a set silhouette independently of its rarity color',()=>{
  const width=488,height=680,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([220,220,210,255],p*4);
  const template={width:20,height:20,data:new Uint8ClampedArray(20*20*4)};
  for(let y=0;y<20;y++)for(let x=0;x<20;x++)if(Math.abs(x-9.5)+Math.abs(y-9.5)<9){template.data[(y*20+x)*4+3]=255;data.set([160,95,35,255],((390+y)*width+420+x)*4);}
  const found=locateSetSymbol({data,width,height},{x:30,y:382,width:426,height:36},template);
  assert.ok(found);
  assert.ok(Math.abs(found.x-420)<=2&&Math.abs(found.y-390)<=2);
  assert.equal(locateSetSymbol({data:new Uint8ClampedArray(width*height*4).fill(220),width,height},{x:30,y:382,width:426,height:36},template),null);
});
