import test from 'node:test';
import assert from 'node:assert/strict';
import {hasOutlinedLightText,protectBottomOrnaments,isPanelInk,expandGlyphMask,clearEdgeRules,glyphSimilarity,inpaintGlyphMask,findFlavorSeparator,statsGlyphMask,residualTextQuality} from '../src/lib/card-frame-font-mask.js';

test('pale holes in dark letters on gray paper are not outlined white text',()=>{
 const width=60,height=20,data=new Uint8ClampedArray(width*height*4);
 for(let p=0;p<width*height;p++)data.set([182,182,182,255],p*4);
 for(let i=0;i<5;i++)for(let y=3;y<17;y++)for(let x=3+i*11;x<11+i*11;x++){
  const ring=x===3+i*11||x===10+i*11||y===3||y===16;
  data.set(ring?[20,20,20,255]:[245,245,245,255],(y*width+x)*4);
 }
 assert.equal(hasOutlinedLightText({data,width,height}),false);
});

test('outlined pale lettering is detected on a saturated bright background',()=>{
 const width=60,height=20,data=new Uint8ClampedArray(width*height*4);
 for(let p=0;p<width*height;p++)data.set([60,190,220,255],p*4);
 for(let i=0;i<5;i++)for(let y=3;y<17;y++)for(let x=3+i*11;x<11+i*11;x++){
  const ring=x===3+i*11||x===10+i*11||y===3||y===16;
  data.set(ring?[20,20,20,255]:[245,245,245,255],(y*width+x)*4);
 }
 assert.equal(hasOutlinedLightText({data,width,height}),true);
});

test('residual validation rejects dark lettering left on a dark copper rail',()=>{
  const width=8,height=8,data=new Uint8ClampedArray(width*height*4),points=[];
  for(let p=0;p<width*height;p++)data.set([175,90,20,255],p*4);
  const clean={data:data.slice(),width,height};
  for(let y=2;y<6;y++)for(let x=2;x<5;x++){
    const p=y*width+x;points.push(p);data.set([15,15,15,255],p*4);
  }
  const scan={data,width,height},paperAt=()=>95;
  assert.equal(residualTextQuality(scan,scan,[{points}],paperAt,{polarity:'dark'}).safe,false);
  assert.equal(residualTextQuality(scan,clean,[{points}],paperAt,{polarity:'dark'}).safe,true);
});

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

test('joined-word splitting retains all ink across scan scales',async()=>{
  const {splitJoinedGlyph}=await import('../src/lib/card-frame-font-mask.js');
  for(const scale of [1,2,3]) {
    const w=60*scale,h=12*scale,pixels=new Uint8Array(w*h);
    for(let y=0;y<h;y++)for(let x=0;x<w;x++)if(x%(10*scale)<2*scale||y===h-2)pixels[y*w+x]=1;
    const parts=splitJoinedGlyph({w,h,pixels});
    assert.ok(parts.length>1);
    assert.equal(parts.reduce((n,p)=>n+p.pixels.reduce((a,b)=>a+b,0),0),pixels.reduce((a,b)=>a+b,0));
    assert.ok(parts.every(p=>p.w<=h*1.5));
  }
});

test('residual verification rejects a missed word even in an otherwise clean region',async()=>{
  const {residualTextQuality}=await import('../src/lib/card-frame-font-mask.js');
  const width=100,height=20,data=new Uint8ClampedArray(width*height*4).fill(220);
  const c={points:[101,102,103,104,201,202,203,204]};
  for(const p of c.points)data.set([20,20,20,255],p*4);
  const scan={width,height,data};
  assert.equal(residualTextQuality(scan,scan,[c],()=>220).safe,false);
  const clean={...scan,data:new Uint8ClampedArray(data.length).fill(220)};
  assert.equal(residualTextQuality(scan,clean,[c],()=>220).safe,true);
});

test('bottom-connected security ornament stays protected without removing nearby text',()=>{
  const width=40,height=30,ink=new Uint8Array(width*height);
  for(let y=25;y<height;y++)for(let x=16;x<24;x++)ink[y*width+x]=1;
  for(let y=14;y<21;y++)ink[y*width+10]=1;
  const protectedPixels=protectBottomOrnaments(ink,width,height);
  assert.equal(ink[25*width+20],0);
  assert.equal(protectedPixels[25*width+20],1);
  assert.equal(ink[18*width+10],1);
  const expanded=expandGlyphMask(ink,width,height,protectedPixels,8);
  assert.equal(expanded[25*width+20],0);
});

test('dense paper cleanup fills ink farther than forty pixels from a donor',()=>{
  const width=110,height=110,data=new Uint8ClampedArray(width*height*4),mask=new Uint8Array(width*height);
  for(let y=0;y<height;y++)for(let x=0;x<width;x++) {
    const p=y*width+x,inside=x>=5&&x<105&&y>=5&&y<105;
    data.set(Math.abs(x-55)<2&&Math.abs(y-55)<2?[10,10,10,255]:[210,220,230,255],p*4);mask[p]=inside?1:0;
  }
  const clean=inpaintGlyphMask({data,width,height},mask);
  assert.ok(clean.data[(55*width+55)*4]>190,'the center contains paper rather than original ink');
});

test('separator search uses the rules/flavor gap, including a missing font registration',()=>{
  const width=120,height=100,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([220,225,230,255],p*4);
  for(const top of [12,28,55,72])for(let y=top;y<top+8;y++)for(let x=20;x<95;x+=10)for(let dx=0;dx<3;dx++)data.set([20,20,20,255],(y*width+x+dx)*4);
  const scan={data,width,height};
  assert.equal(findFlavorSeparator(scan,55),null,'a blank paragraph gap is not a separator');
  for(let x=8;x<112;x++)data.set([195,200,205,255],(44*width+x)*4);
  assert.equal(findFlavorSeparator(scan,55),44);
  assert.equal(findFlavorSeparator(scan,undefined),44,'text bands locate the gap when font registration fails');
  assert.equal(findFlavorSeparator(scan,28),null,'a rule below the registered flavor start is excluded');
});

test('stats contrast removes irregular digits and slash in both polarities while preserving bevels',()=>{
  for(const [paper,ink] of [[220,25],[50,235]]) {
    const width=60,height=30,data=new Uint8ClampedArray(width*height*4);
    for(let p=0;p<width*height;p++)data.set([paper,paper,paper,255],p*4);
    const paint=(x,y)=>data.set([ink,ink,ink,255],(y*width+x)*4);
    for(let x=0;x<width;x++)paint(x,1);
    paint(0,0);paint(0,1);
    for(let y=6;y<24;y++){paint(12,y);paint(42,y);paint(34-Math.floor(y/2),y);}
    const mask=statsGlyphMask({data,width,height}),clean=inpaintGlyphMask({data,width,height},mask);
    for(const x of [12,42])assert.ok(Math.abs(clean.data[(15*width+x)*4]-paper)<10,'digit is filled with paper');
    assert.ok(Math.abs(clean.data[(15*width+27)*4]-paper)<10,'slash is filled with paper');
    for(let x=0;x<width;x++)assert.equal(mask[width+x],0,'connected badge bevel stays intact');
  }
});
