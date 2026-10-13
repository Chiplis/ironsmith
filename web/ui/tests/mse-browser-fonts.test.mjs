import test from 'node:test';
import assert from 'node:assert/strict';
import { MseBrowserFonts } from '../src/lib/mse/browser-fonts.mjs';
function fixture(load = async face => face) {
  const fontSet = new Set();
  class Face {
    constructor(family,source){this.family=family;this.source=source;}
    load(){return load(this);}
  }
  const canvas={getContext:()=>({measureText:()=>({width:10,fontBoundingBoxAscent:8,fontBoundingBoxDescent:2})})};
  return {fontSet, FontFaceClass:Face,canvas};
}
test('code fonts use Courier New and native system-size fallback at tiny scaled sizes',async()=>{
  const options=fixture(),ctx=options.canvas.getContext();options.canvas={getContext:()=>ctx};
  const fonts=new MseBrowserFonts({...options,normalFont:{family:'system-ui',pixelSize:13}});
  const face=await fonts.register('Courier New','fixture');
  const metrics=await fonts.prepare({name:'Base',italic_name:'Other',size:12,code:true,bold:true,italic:true});
  metrics.measure('code',null,1);assert.equal(ctx.font,`normal bold 16px "${face.family}"`);
  metrics.measure('code',null,0.1);assert.equal(ctx.font,`normal bold 13px "${face.family}"`);
  metrics.measure('code',null,0.01);assert.equal(ctx.font,`normal bold 13px "${face.family}"`);
  fonts.dispose();
});
test('font families remain isolated across providers sharing the document font set', async()=>{
  const options=fixture(),a=new MseBrowserFonts(options),b=new MseBrowserFonts(options);
  const af=await a.register('Matrix','first'),bf=await b.register('Matrix','second');
  assert.notEqual(af.family,bf.family);
  const metrics=await a.prepare({name:'Matrix',size:10});
  assert.equal(metrics.measure('A',null,1).width,10);
  a.dispose();
  assert.equal(options.fontSet.has(af),false);
  assert.equal(options.fontSet.has(bf),true);
  assert.throws(()=>metrics.measure('A',null,1),{code:'font-disposed'});
  assert.equal((await b.prepare({name:'Matrix',size:10})).measure('A',null,1).width,10);
  await assert.rejects(a.register('Matrix','again'),{code:'font-disposed'});
  b.dispose(); assert.equal(options.fontSet.size,0);
});
test('disposing during font loading prevents late registration and stale metrics', async()=>{
  let release;
  const options=fixture(face=>new Promise(resolve=>{release=()=>resolve(face);}));
  const fonts=new MseBrowserFonts(options);
  const loading=fonts.register('Matrix','first');
  const prepared=fonts.prepare({name:'Matrix',size:10});
  const rejectedLoad=assert.rejects(loading,{code:'font-disposed'});
  const rejectedPrepare=assert.rejects(prepared,{code:'font-disposed'});
  fonts.dispose();release();
  await Promise.all([rejectedLoad,rejectedPrepare]);
  assert.equal(options.fontSet.size,0);
  assert.equal(fonts.families.size,0);
});

test('empty font names use the host normal font while absent named fonts still fail',async()=>{
  const context={measureText:()=>({width:10,fontBoundingBoxAscent:8,fontBoundingBoxDescent:2})};
  const fonts=new MseBrowserFonts({canvas:{getContext:()=>context},normalFont:{family:'system-ui',pixelSize:12}});
  const normal=await fonts.prepare({name:'',size:1,bold:true,italic:true});
  normal.measure('A',null,2);
  assert.equal(context.font,'normal normal 24px system-ui');
  const sized=await fonts.prepare({name:'',size:15});
  sized.measure('A',null,1);
  assert.equal(context.font,'normal normal 20px system-ui');
  await assert.rejects(fonts.prepare({name:'Missing',size:12}),{code:'font-missing'});
  fonts.dispose();
  assert.throws(()=>normal.measure('A',null,1),{code:'font-disposed'});
});


test('font lookup is case-insensitive without altering family spacing',async()=>{
  const fonts=new MseBrowserFonts(fixture());
  await fonts.register('Attic Antique DemiBold','bytes');
  const provider=await fonts.prepare({name:'Attic Antique Demibold',size:12});
  assert.equal(provider.measure('A',null,1).width,10);
  await assert.rejects(fonts.prepare({name:'AtticAntiqueDemibold',size:12}),{code:'font-missing'});
  fonts.dispose();
});

test('line decorations scale signed font metrics from the baseline and use the selected face',async()=>{
  const fonts=new MseBrowserFonts(fixture()),rectangles=[];
  await fonts.register('Example','normal',{weight:'400'},{unitsPerEm:1000,underlinePosition:-100,underlineThickness:100,strikePosition:300,strikeThickness:50});
  await fonts.register('Example','bold',{weight:'700'},{unitsPerEm:1000,underlinePosition:-200,underlineThickness:100,strikePosition:400,strikeThickness:100});
  const target={measureText:()=>({width:30,fontBoundingBoxAscent:16}),fillText(){},fillRect(...args){rectangles.push(args);}};
  const normal=await fonts.prepare({name:'Example',size:15,underline:true,strikethrough:true});
  normal.draw(target,'ABC',2,3,1);
  assert.deepEqual(rectangles,[[2,21,30,2],[2,13,30,1]]);
  rectangles.length=0;
  const bold=await fonts.prepare({name:'Example',size:15,bold:true,underline:true,strikethrough:true});
  bold.draw(target,'ABC',2,3,2);
  assert.deepEqual(rectangles,[[2,27,30,4],[2,3,30,4]]);
  fonts.dispose();
});

test('enabled local fonts resolve exact names once and never substitute an absent family',async()=>{
  const sources=[];
  const options=fixture(async face=>{sources.push(face.source);if(face.source.includes('Missing'))throw new Error('Not installed');return face;});
  const fonts=new MseBrowserFonts({...options,allowSystemFonts:true});
  const [first,second]=await Promise.all([fonts.prepare({name:'Verdana',size:12}),fonts.prepare({name:'Verdana',size:12})]);
  assert.equal(first.measure('A',null,1).width,second.measure('A',null,1).width);
  assert.deepEqual(sources,['local("Verdana")']);
  await assert.rejects(fonts.prepare({name:'Missing',size:12}),{code:'font-missing'});
  await assert.rejects(fonts.prepare({name:'Missing',size:12}),{code:'font-missing'});
  assert.equal(sources.length,2);
  await fonts.register('Missing','provided bytes');
  assert.equal((await fonts.prepare({name:'Missing',size:12})).measure('A',null,1).width,10);
  fonts.dispose();assert.equal(options.fontSet.size,0);
});
