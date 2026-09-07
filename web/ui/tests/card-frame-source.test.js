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
