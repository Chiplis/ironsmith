import test from 'node:test';
import assert from 'node:assert/strict';
import {isPanelInk,expandGlyphMask,clearEdgeRules,glyphSimilarity,inpaintGlyphMask} from '../src/lib/card-frame-font-mask.js';

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

test('bevel lines along the region edge are cleared without touching lettering',()=>{
  const width=40,height=20,ink=new Uint8Array(width*height);
  // A full-width rule on the bottom edge, a glyph stem, and its descender touching the rule.
  for(let x=0;x<width;x++)ink[(height-1)*width+x]=1;
  for(let y=4;y<height;y++)ink[y*width+10]=1;
  // A dense text row in the middle must survive even when it is mostly ink.
  for(let x=0;x<width*.8;x++)ink[9*width+x]=1;
  const before=ink.slice();
  clearEdgeRules(ink,width,height);
  for(let x=0;x<width;x++)if(x!==10)assert.equal(ink[(height-1)*width+x],0,'edge rule cleared');
  for(let y=4;y<height-1;y++)assert.equal(ink[y*width+10],1,'stem kept');
  for(let x=0;x<width*.8;x++)assert.equal(ink[9*width+x],before[9*width+x],'inner rows untouched');
  // A short line on the edge is lettering (an underscore, a serif), not a bevel.
  const small=new Uint8Array(width*height);for(let x=5;x<15;x++)small[x]=1;
  assert.deepEqual(clearEdgeRules(small.slice(),width,height),small);
});


test('expanded glyph cleanup removes halos while retaining original edge rules',()=>{
  const width=40,height=20,ink=new Uint8Array(width*height);
  for(let x=0;x<width;x++)ink[(height-2)*width+x]=1;
  const original=ink.slice();
  clearEdgeRules(ink,width,height);
  const protectedPixels=original.map((v,p)=>v&&!ink[p]?1:0);
  const accepted=new Uint8Array(width*height);
  accepted[15*width+10]=1;
  const mask=expandGlyphMask(accepted,width,height,protectedPixels);
  assert.equal(mask[15*width+13],1,'includes three-pixel glyph halo');
  for(let x=0;x<width;x++)assert.equal(mask[(height-2)*width+x],0,'preserves bevel');
  assert.equal(mask[10*width+10],0,'distant paper remains unchanged');
});


test('white lettering is recognized on midtone gold without selecting gold material',()=>{
  assert.equal(isPanelInk(245,242,225,144),true);
  assert.equal(isPanelInk(205,203,195,144),true,'antialiased white ink');
  assert.equal(isPanelInk(165,145,90,144),false,'gold paper');
  assert.equal(isPanelInk(220,180,95,144),false,'gold highlight');
  assert.equal(isPanelInk(30,30,30,224),true,'black ink on light paper');
  assert.equal(isPanelInk(230,230,230,224),false,'light paper grain');
  assert.equal(isPanelInk(240,240,240,64),true,'white ink on dark paper');
});
