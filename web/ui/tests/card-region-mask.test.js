import test from 'node:test';
import assert from 'node:assert/strict';
import {maskRegisteredPixels} from '../src/lib/card-region-mask.js';

const scan=()=>({width:60,height:40,data:Uint8ClampedArray.from({length:60*40*4},(_,i)=>i%4===3?255:220)});
const field=(x,y,width=10,height=8)=>({kind:'rule',text:'Printed',lines:[{x:x/60,y:y/40,width:width/60,height:height/40,text:'Printed'}]});
const paint=(image,x,y)=>image.data.set([0,0,0,255],(y*image.width+x)*4);

test('loyalty cleanup removes pale digits without replacing a dark shield with nearby paper',()=>{
 const image=scan();
 for(let y=10;y<31;y++)for(let x=16;x<40;x++)paint(image,x,y);
 for(let y=16;y<24;y++)for(let x=24;x<27;x++)image.data.set([245,245,245,255],(y*60+x)*4);
 const badge={...field(21,14,12,12),kind:'loyalty-cost',text:'−3',polarity:'light',outlined:false};
 const result=maskRegisteredPixels(image,[{field:badge}],()=>{throw Error('Shield cleanup must retain its paper even when font matching fails');});
 assert.ok(result.data[(20*60+25)*4]<30,'digit ink is removed');
 assert.equal(result.data[(14*60+21)*4],0,'shield paper is retained');
 assert.equal(result.data[(9*60+20)*4],220,'surrounding rules paper is retained');
});

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

test('registered lettering honors explicit dark ink on a reflective label',()=>{
  const image=scan(),label={...field(16,12),kind:'type',polarity:'dark'};
  let options;
  const result=maskRegisteredPixels(image,[{field:label}],(region,value)=>{options=value;return {mask:new Uint8Array(region.width*region.height),quality:{safe:true}};});
  assert.equal(options.polarity,'dark');
  assert.equal(options.outlined,false);
  assert.equal(result.inks[0],'rgb(0,0,0)');
});

test('inverted text is recognized upright and its glyph mask returns to scan coordinates',()=>{
 const image=scan();paint(image,18,16);
 const inverted={...field(16,12),rotation:180};
 const result=maskRegisteredPixels(image,[{field:inverted}],region=>{
  const originalIndex=6*region.width+4,rotatedIndex=region.width*region.height-1-originalIndex;
  assert.equal(region.data[rotatedIndex*4],0,'template input is upright');
  const mask=new Uint8Array(region.width*region.height);mask[rotatedIndex]=1;
  return {mask,quality:{safe:true}};
 });
 assert.ok(result.data[(16*60+18)*4]>210,'correct source glyph is removed');
 for(let p=0;p<result.mask.length;p++)if(!result.mask[p])assert.deepEqual(result.data.slice(p*4,p*4+4),image.data.slice(p*4,p*4+4));
});

test('changed decorative headers clear their complete label interior',()=>{
 const image=scan();paint(image,18,16);paint(image,25,16);
 const header={...field(16,12),kind:'name',opaqueHeader:true};let called=false;
 const result=maskRegisteredPixels(image,[{field:header}],()=>{called=true;return {mask:new Uint8Array(1),quality:{safe:true}};});
 assert.equal(called,false,'unavailable decorative glyphs are not matched as ordinary letters');
 assert.ok(result.data[(16*60+18)*4]>210);assert.ok(result.data[(16*60+25)*4]>210);
 assert.equal(result.mask[0],0);
});

test('quarter-turn fields are recognized upright and map their glyphs back exactly',()=>{
 const image=scan();paint(image,18,16);
 const rotated={...field(16,12),rotation:90};
 const result=maskRegisteredPixels(image,[{field:rotated}],region=>{
  assert.equal(region.width,12);assert.equal(region.height,14);
  const index=(14-1-4)*12+6;
  assert.equal(region.data[index*4],0);
  const mask=new Uint8Array(region.width*region.height);mask[index]=1;
  return {mask,quality:{safe:true}};
 });
 assert.ok(result.data[(16*60+18)*4]>210);
 for(let p=0;p<result.mask.length;p++)if(!result.mask[p])assert.deepEqual(result.data.slice(p*4,p*4+4),image.data.slice(p*4,p*4+4));
});

test('decorative label rebuilding preserves an overlapping neighboring label',()=>{
 const image=scan();paint(image,18,16);paint(image,25,16);
 const decorated={...field(16,12),opaqueLettering:true,protectedBounds:[{x:24/60,y:12/40,width:5/60,height:8/40}]};
 const result=maskRegisteredPixels(image,[{field:decorated}],()=>{throw Error('Decorative lettering has no matching font');});
 assert.ok(result.data[(16*60+18)*4]>210);
 assert.equal(result.data[(16*60+25)*4],0);
});

test('shared pale rails retain their paper when black lettering is rebuilt',()=>{
 const image=scan();paint(image,18,16);
 const rail={...field(16,12),sharedRule:true,polarity:'dark',rebuildPrintedLines:true};
 const result=maskRegisteredPixels(image,[{field:rail}],()=>null);
 assert.ok(result.data[(16*60+18)*4]>210,'letter removed');
 assert.equal(result.mask[12*60+16],0,'rail paper remains a donor');
 assert.deepEqual(result.data.slice((12*60+16)*4,(12*60+16)*4+4),image.data.slice((12*60+16)*4,(12*60+16)*4+4));
});

test('failed templates clear low-contrast ink while retaining brown panel paper',()=>{
 const image=scan();
 for(let p=0;p<image.width*image.height;p++)image.data.set([115,85,55,255],p*4);
 image.data.set([75,45,15,255],(16*60+20)*4);
 const result=maskRegisteredPixels(image,[{field:{...field(16,12),polarity:'dark'}}],()=>null);
 assert.ok(result.mask[16*60+20], 'faint printed ink is included');
 assert.equal(result.mask[12*60+16],0, 'untouched paper stays available as a donor');
 assert.ok(result.data[(16*60+20)*4]>105, 'the faint lettering is removed');
});
