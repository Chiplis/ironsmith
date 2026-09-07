import test from 'node:test';
import assert from 'node:assert/strict';
import {glyphSimilarity,inpaintGlyphMask} from '../src/lib/card-frame-font-mask.js';

test('glyph matching distinguishes shape rather than merely dark pixels',()=>{
  const a={w:3,h:3,pixels:Uint8Array.from([1,0,0,1,0,0,1,1,1])};
  const b={w:3,h:3,pixels:Uint8Array.from([1,1,1,0,1,0,0,1,0])};
  assert.equal(glyphSimilarity(a,a),1);
  assert.ok(glyphSimilarity(a,b)<.4);
});

test('glyph inpainting changes only the supplied mask and preserves a lighting gradient',()=>{
  const width=40,height=30,data=new Uint8ClampedArray(width*height*4),mask=new Uint8Array(width*height);
  for(let y=0;y<height;y++)for(let x=0;x<width;x++) {
    const p=y*width+x;data.set([150+x,160+x,170+x,255],p*4);
    if(x>=18&&x<=20&&y>=7&&y<=22){mask[p]=1;data.set([0,0,0,255],p*4);}
  }
  const clean=inpaintGlyphMask({data,width,height},mask);
  for(let p=0;p<mask.length;p++)if(!mask[p])assert.deepEqual(clean.data.subarray(p*4,p*4+4),data.subarray(p*4,p*4+4));
  assert.ok(Math.abs(clean.data[(15*width+19)*4]-169)<3);
});
