import test from 'node:test';
import assert from 'node:assert/strict';
import {maskRegisteredPixels} from '../src/lib/card-region-mask.js';

const scan=()=>({width:60,height:40,data:Uint8ClampedArray.from({length:60*40*4},(_,i)=>i%4===3?255:220)});
const field=(x,y,width=10,height=8)=>({kind:'rule',text:'Printed',lines:[{x:x/60,y:y/40,width:width/60,height:height/40,text:'Printed'}]});
const paint=(image,x,y)=>image.data.set([0,0,0,255],(y*image.width+x)*4);

test('unsafe font masks reconstruct the whole line, including rejected lettering',()=>{
  const image=scan();paint(image,18,16);paint(image,25,16);
  const result=maskRegisteredPixels(image,[{field:field(16,12)}],region=>({mask:new Uint8Array(region.width*region.height),quality:{safe:false}}));
  assert.deepEqual(result.fallbacks,[true]);
  for(const x of [18,25])assert.ok(result.data[(16*60+x)*4]>210,'rejected ink is removed');
  assert.equal(image.data[(16*60+18)*4],0,'original scan is unchanged');
  assert.equal(result.mask[0],0,'outside paper is preserved');
});

test('overlapping line masks are united before inpainting; later original crops cannot restore ink',()=>{
  const image=scan();paint(image,20,16);paint(image,24,16);
  const clean=region=>{
    const mask=new Uint8Array(region.width*region.height);
    // Crops start at (14,10) and (18,10). Both contain both printed pixels,
    // but each recognizer accepts only its own pixel at local (6,6).
    mask[6*region.width+6]=1;
    return {mask,quality:{safe:true},data:region.data};
  };
  const result=maskRegisteredPixels(image,[{field:field(16,12)},{field:field(20,12)}],clean);
  assert.deepEqual(result.fallbacks,[false,false]);
  for(const x of [20,24])assert.ok(result.data[(16*60+x)*4]>210);
  for(let p=0;p<result.mask.length;p++)if(!result.mask[p])assert.deepEqual(result.data.slice(p*4,p*4+4),image.data.slice(p*4,p*4+4));
});

test('missing mask results use the same complete-line fallback',()=>{
  const image=scan();paint(image,20,16);
  const result=maskRegisteredPixels(image,[{field:field(16,12)}],()=>null);
  assert.deepEqual(result.fallbacks,[true]);
  assert.ok(result.data[(16*60+20)*4]>210);
});
