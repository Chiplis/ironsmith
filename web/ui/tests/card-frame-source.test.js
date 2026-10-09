import test from 'node:test';
import assert from 'node:assert/strict';
import {maskSourceFrame} from '../src/lib/card-frame-source.js';
import {reconstructPanel} from '../src/lib/card-frame-colors.js';

test('a failed text mask never replaces a whole panel with a solid fill', () => {
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4).fill(120);
  const original=data.slice();
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50}};
  assert.equal(maskSourceFrame({data,width,height},boxes,null,null,()=>null),null);
  assert.deepEqual(data,original);
});

test('source frame changes only masked text pixels and preserves artwork and borders',()=>{
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4);
  for(let y=0;y<height;y++)for(let x=0;x<width;x++)data.set([210+x%9,210+y%7,210,255],(y*width+x)*4);
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50},art:{x:10,y:45,width:140,height:75}};
  for(const box of [boxes.title,boxes.type,boxes.rules])for(let y=box.y+10;y<box.y+17;y++)for(let x=30;x<90;x+=12)for(let dx=0;dx<3;dx++)data.set([10,10,10,255],(y*width+x+dx)*4);
  const result=maskSourceFrame({data,width,height},boxes,null,null,reconstructPanel);
  assert.ok(result.mask.some(Boolean));
  for(let y=209;y<height;y++)for(let x=0;x<width;x++)assert.equal(result.mask[y*width+x],0,'artist and collector footer remains untouched');
  for(let p=0;p<width*height;p++)if(!result.mask[p])assert.deepEqual(result.data.subarray(p*4,p*4+4),data.subarray(p*4,p*4+4));
  for(let y=45;y<120;y++)for(let x=10;x<150;x++)assert.equal(result.mask[y*width+x],0);
  assert.ok(result.data[((20*width)+30)*4]>180,'printed ink is filled with surrounding paper');
});

test('SVG masks erase individual discs while preserving paper between them',()=>{
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([190,215,230,255],p*4);
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50}};
  const icon={width:20,height:20,data:new Uint8ClampedArray(20*20*4)};
  for(let y=0;y<20;y++)for(let x=0;x<20;x++)if(Math.hypot(x-9.5,y-9.5)<9.5)icon.data[(y*20+x)*4+3]=255;
  const symbols=[{x:98,y:17,width:16,height:16},{x:123,y:17,width:16,height:16}];
  for(const b of symbols)for(let y=b.y;y<b.y+16;y++)for(let x=b.x;x<b.x+16;x++)if(Math.hypot(x-b.x-7.5,y-b.y-7.5)<7.5)data.set([10,10,10,255],(y*width+x)*4);
  const noGlyphs=scan=>({...scan,mask:new Uint8Array(scan.width*scan.height)});
  const result=maskSourceFrame({data,width,height},boxes,null,null,noGlyphs,{fontGuided:true,manaMatch:{symbols},icons:[icon,icon]});
  assert.ok(result.data[(25*width+106)*4]>170);
  assert.equal(result.mask[25*width+119],0,'gap between symbols is unchanged');
  assert.deepEqual(result.data.subarray(210*width*4),data.subarray(210*width*4),'footer is preserved');
});

test('modern basic-land watermark regions are preserved instead of treated as rules glyphs',()=>{
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4).fill(190);
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50}};
  const touched=[];
  const clean=(scan,options)=>{touched.push(options.section);return {...scan,mask:new Uint8Array(scan.width*scan.height).fill(1)};};
  const result=maskSourceFrame({data,width,height},boxes,null,null,clean,{preserveRules:true});
  assert.deepEqual(touched,['title','type']);
  for(let y=158;y<208;y++)for(let x=10;x<150;x++)assert.equal(result.mask[y*width+x],0);
});

test('residual text failure discards the entire replacement frame',()=>{
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4).fill(210);
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50}};
  const original=data.slice();
  const result=maskSourceFrame({width,height,data},boxes,null,null,(scan,{section})=>({...scan,mask:new Uint8Array(scan.width*scan.height),quality:{safe:section!=='type'}}));
  assert.equal(result,null);
  assert.deepEqual(data,original);
});

test('a registered symbol lets label masking examine suffixes beyond measured bounds',()=>{
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4).fill(210);
  data.set([10,10,10,255],(135*width+90)*4);
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50}};
  const clean=scan=>{const mask=new Uint8Array(scan.width*scan.height),out=scan.data.slice();for(let p=0;p<mask.length;p++)if(out[p*4]<50){mask[p]=1;out.set([210,210,210,255],p*4);}return {...scan,data:out,mask};};
  const result=maskSourceFrame({width,height,data},boxes,null,null,clean,{setSymbol:{x:120,y:128,width:10,height:20},textBounds:{type:{x:15,y:133,width:30,height:10}}});
  assert.equal(result.data[(135*width+90)*4],210);
});

test('mana below the title does not truncate its cleanup region',()=>{
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([210,210,210,255],p*4);
  data.set([10,10,10,255],(25*width+70)*4);
  const boxes={title:{x:40,y:10,width:110,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50}};
  const clean=scan=>{const mask=new Uint8Array(scan.width*scan.height),out=scan.data.slice();for(let p=0;p<mask.length;p++)if(out[p*4]<50){mask[p]=1;out.set([210,210,210,255],p*4);}return {...scan,data:out,mask};};
  for(const x of [20,90]){
    const result=maskSourceFrame({width,height,data},boxes,null,null,clean,{
      fontGuided:true,textBounds:{title:{x:50,y:20,width:70,height:15}},
      manaMatch:{symbols:[{x,y:60,width:16,height:16}]},
      icons:[{width:1,height:1,data:new Uint8ClampedArray(4)}],
    });
    assert.equal(result.data[(25*width+70)*4],210,'title ink is removed despite an off-title cost');
  }
});

test('integrated rules retain enough paper margin to remove initial letters',()=>{
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([230,230,230,255],p*4);
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50}};
  for(let y=170;y<178;y++)for(let x=16;x<19;x++)data.set([10,10,10,255],(y*width+x)*4);
  const result=maskSourceFrame({width,height,data},boxes,null,null,reconstructPanel,{rules:'integrated'});
  for(let y=170;y<178;y++)for(let x=16;x<19;x++)assert.ok(result.data[(y*width+x)*4]>180,'leftmost text is removed');
  assert.equal(result.mask[170*width+9],0,'outside frame stays untouched');
});

test('the rules mask receives the protected stats region before quality validation',()=>{
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4).fill(220);
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50}};
  const panel={x:115,y:175,width:30,height:35};let protectedCount=0;
  maskSourceFrame({data,width,height},boxes,null,panel,(patch,options)=>{
    if(options.section==='rules')protectedCount=options.excludedPixels.reduce((a,b)=>a+b,0);
    return {...patch,mask:new Uint8Array(patch.width*patch.height)};
  });
  assert.ok(protectedCount>0);
});

test('rules reflect cleaned upper paper over lower flavor ink and separators, preserving stats',()=>{
  for(const boxHeight of [50,51]) {
    const width=160,height=220,data=new Uint8ClampedArray(width*height*4);
    for(let y=0;y<height;y++)for(let x=0;x<width;x++)data.set([200+y%13,210+x%7,220,255],(y*width+x)*4);
    const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:boxHeight}};
    const panel={x:115,y:190,width:30,height:20};
    for(let x=20;x<110;x++)data.set([90,90,90,255],(190*width+x)*4);
    let inspectedHeight;
    const clean=(scan,{section})=>{
      if(section==='rules')inspectedHeight=scan.height;
      return {...scan,mask:new Uint8Array(scan.width*scan.height)};
    };
    const result=maskSourceFrame({data,width,height},boxes,null,panel,clean);
    assert.equal(inspectedHeight,Math.ceil((boxHeight-5)/2));
    const top=158,bottom=158+boxHeight-6;
    for(let y=top+inspectedHeight;y<=bottom;y++) {
      const sourceY=top+Math.max(8,bottom-y);
      assert.deepEqual(result.data.subarray((y*width+40)*4,(y*width+41)*4),data.subarray((sourceY*width+40)*4,(sourceY*width+41)*4));
    }
    assert.equal(result.mask[190*width+120],0,'stats panel is preserved');
    assert.equal(result.mask[190*width+12],0,'frame edge is preserved');
  }
});

test('rules cleanup retains accepted first-line glyph removals inside the protected top margin',()=>{
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([35,35,35,255],p*4);
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50}};
  // The rules crop begins at y=158. Both this bevel and the top of the
  // lettering fall inside the eight rows reserved by the paper cleanup.
  for(let x=16;x<144;x++)data.set([80,100,120,255],(158*width+x)*4);
  for(let y=160;y<172;y++)for(let x=30;x<80;x+=10)data.set([255,255,255,255],(y*width+x)*4);
  const clean=scan=>{
    const pixels=scan.data.slice(),mask=new Uint8Array(scan.width*scan.height);
    for(let p=0;p<mask.length;p++)if(pixels[p*4]===255){mask[p]=1;pixels.set([35,35,35,255],p*4);}
    return {...scan,data:pixels,mask,quality:{safe:true}};
  };
  const result=maskSourceFrame({data,width,height},boxes,null,null,clean,{fontGuided:true});
  for(let y=160;y<172;y++)for(let x=30;x<80;x+=10){
    assert.equal(result.mask[y*width+x],1,'accepted glyph remains masked');
    assert.ok(result.data[(y*width+x)*4]<100,'printed white ink does not return');
  }
  for(let x=16;x<144;x++){
    assert.equal(result.mask[158*width+x],0,'adjacent bevel remains protected');
    assert.deepEqual(result.data.subarray((158*width+x)*4,(158*width+x+1)*4),data.subarray((158*width+x)*4,(158*width+x+1)*4));
  }
});

test('reflection does not duplicate the upper panel bevel at the bottom',()=>{
  const width=160,height=220,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([210,220,230,255],p*4);
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:50}};
  for(let y=163;y<166;y++)for(let x=16;x<144;x++)data.set([80,100,120,255],(y*width+x)*4);
  const clean=scan=>({...scan,mask:new Uint8Array(scan.width*scan.height)});
  const result=maskSourceFrame({data,width,height},boxes,null,null,clean);
  assert.equal(result.data[(163*width+50)*4],80,'original top bevel stays in place');
  for(let y=200;y<203;y++)assert.equal(result.data[(y*width+50)*4],210,'bottom uses paper below the bevel');
});

test('a separator above the midpoint shortens the donor strip before reflection',()=>{
  const width=160,height=260,data=new Uint8ClampedArray(width*height*4);
  for(let p=0;p<width*height;p++)data.set([220,230,240,255],p*4);
  const boxes={title:{x:10,y:10,width:140,height:30},type:{x:10,y:125,width:140,height:30},rules:{x:10,y:158,width:140,height:80}};
  for(const top of [175,205])for(let y=top;y<top+8;y++)for(let x=30;x<120;x+=10)for(let dx=0;dx<3;dx++)data.set([20,20,20,255],(y*width+x+dx)*4);
  for(let x=24;x<136;x++)data.set([190,200,210,255],(193*width+x)*4);
  let donorHeight;
  const clean=(scan,{section})=>{if(section==='rules')donorHeight=scan.height;return {...scan,mask:new Uint8Array(scan.width*scan.height)};};
  const result=maskSourceFrame({data,width,height},boxes,null,null,clean,{hasFlavor:true,flavorTop:205});
  assert.equal(donorHeight,31,'donors end four pixels above the separator');
  for(let y=193;y<233;y++)assert.ok(result.data[(y*width+60)*4]>210,'the lower box contains paper without repeated separator ink');
});
