import test from 'node:test';
import assert from 'node:assert/strict';
import {reconstructFrameMaterial} from '../src/lib/card-frame-material.js';

test('excludes printed regions while preserving regional lighting and source grain',()=>{
  const width=160,height=220,bounds={x:8,y:8,width:144,height:204};
  const boxes={title:{x:16,y:20,width:128,height:14},art:{x:22,y:42,width:116,height:80},type:{x:18,y:128,width:124,height:12},rules:{x:20,y:148,width:120,height:48}};
  const data=new Uint8ClampedArray(width*height*4);
  for(let y=0;y<height;y++)for(let x=0;x<width;x++) {
    const paper=40+x*.3+y*.2+((x*3+y*7)%7);
    const printed=Object.values(boxes).some(b=>x>=b.x&&x<b.x+b.width&&y>=b.y&&y<b.y+b.height);
    data.set(printed?[255,0,255,255]:[paper,paper+5,paper+10,255],(y*width+x)*4);
  }
  const result=reconstructFrameMaterial({data,width,height},bounds,boxes);
  assert.ok(result.donorCount>0);
  assert.equal(result.mask[36*width+60],1,'gap above resized art cannot leak original illustration');
  assert.equal(result.mask[204*width+16],1,'collector print is excluded across the footer');
  assert.deepEqual(reconstructFrameMaterial({data,width,height},bounds,boxes).data,result.data,'repeatable material');
  for(let p=0;p<width*height;p++) {
    if(!result.mask[p]) assert.deepEqual(result.data.subarray(p*4,p*4+4),data.subarray(p*4,p*4+4),'exposed grain is retained exactly');
    assert.ok(result.data[p*4]-result.data[p*4+1]<20,'magenta print cannot become grain');
  }
  const at=(x,y)=>result.data[(y*width+x)*4];
  assert.ok(at(130,180)>at(25,50)+30,'regional lightness survives reconstruction');
});

test('declines reconstruction when no exposed material is available',()=>{
  const width=100,height=140,data=new Uint8ClampedArray(width*height*4).fill(255);
  const box={x:0,y:0,width,height};
  assert.equal(reconstructFrameMaterial({data,width,height},box,{title:box,type:box,art:box,rules:box}),null);
});
