import test from 'node:test';
import assert from 'node:assert/strict';
import { MseBrowserFonts } from '../src/lib/mse/browser-fonts.mjs';
function fixture(load = async face => face) {
  const fontSet = new Set();
  class Face {
    constructor(family,source,descriptors){this.family=family;this.source=source;this.descriptors=descriptors;}
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

test('explicit native font DPI changes point conversion while the default remains 96 DPI',async()=>{
  for(const [pixelsPerPoint,expected] of [[undefined,16],[1,12]]){
    const options=fixture(),ctx=options.canvas.getContext();options.canvas={getContext:()=>ctx};
    const fonts=new MseBrowserFonts({...options,pixelsPerPoint});
    const face=await fonts.register('Example','fixture');
    const metrics=await fonts.prepare({name:'Example',size:12});
    metrics.measure('text',null,1);
    assert.equal(ctx.font,`normal normal ${expected}px "${face.family}"`);
    const override=await fonts.prepare({name:'Example',size:12},{pixelsPerPoint:1});
    override.measure('text',null,1);assert.equal(ctx.font,`normal normal 12px "${face.family}"`);
    await assert.rejects(fonts.prepare({name:'Example',size:12},{pixelsPerPoint:0}),{code:'font-provider'});
    fonts.dispose();
  }
  assert.throws(()=>new MseBrowserFonts({...fixture(),pixelsPerPoint:NaN}),{code:'font-provider'});
});

test('macOS reference metrics include hhea leading and native fourfold quantization',async()=>{
  const options=fixture(),ctx=options.canvas.getContext();options.canvas={getContext:()=>ctx};
  const fonts=new MseBrowserFonts({...options,pixelsPerPoint:1});
  const face=await fonts.register('MPlantin','fixture',{},null,{unitsPerEm:1000,ascent:774,descent:-225,lineGap:150});
  const metrics=await fonts.prepare({name:'MPlantin',size:14});
  assert.deepEqual(metrics.measure('Ability',null,1),{width:10,height:16.25});
  metrics.measure('Ability',null,0.9);
  assert.equal(ctx.font,`normal normal 12.5px "${face.family}"`);
  // MSE's tie bias rounds 50.5 down on its fourfold text surface.
  metrics.measure('Ability',null,50.5/56);
  assert.equal(ctx.font,`normal normal 12.5px "${face.family}"`);
  const browser=await fonts.prepare({name:'MPlantin',size:14},{pixelsPerPoint:96/72});
  assert.equal(browser.measure('Ability',null,1).height,10);
  fonts.dispose();
});

test('macOS metrics use descent magnitude for legacy positive hhea descenders',async()=>{
  const fonts=new MseBrowserFonts({...fixture(),pixelsPerPoint:1});
  await fonts.register('ModMatrix','fixture',{weight:'700'},null,{unitsPerEm:1000,ascent:633,descent:167,lineGap:0});
  const metrics=await fonts.prepare({name:'ModMatrix',size:18,bold:true});
  assert.equal(metrics.measure('3/4',null,1).height,14.5);
  fonts.dispose();
});

test('native raster drawing scales the already quantized font size',async()=>{
  const options=fixture(),ctx=options.canvas.getContext();options.canvas={getContext:()=>ctx};
  const fonts=new MseBrowserFonts({...options,pixelsPerPoint:1});
  const face=await fonts.register('Example','fixture');
  const provider=await fonts.prepare({name:'Example',size:14});
  const target={measureText:ctx.measureText,fillText(){}};
  provider.drawRaster(target,'A',0,0,0.9);
  assert.equal(target.font,`normal normal 50px "${face.family}"`);
  assert.equal(provider.rasterScale,4);
  assert.equal(provider.pointSize(0.9),12.5);
  fonts.dispose();
});

test('native glyph baselines retain fractional font ascent at raster resolution',async()=>{
  const fonts=new MseBrowserFonts({...fixture(),pixelsPerPoint:1});
  await fonts.register('MPlantin','fixture',{},null,{unitsPerEm:1000,ascent:774,descent:-225,lineGap:150});
  const calls=[],target={measureText:()=>({width:30,fontBoundingBoxAscent:43}),fillText:(...args)=>calls.push(args)};
  const provider=await fonts.prepare({name:'MPlantin',size:14});
  provider.drawRaster(target,'Trial',2,3,1);
  assert.deepEqual(calls,[['Trial',2,46.344]]);
  const standard=await fonts.prepare({name:'MPlantin',size:14},{pixelsPerPoint:96/72});
  standard.draw(target,'Trial',2,3,1);
  assert.equal(calls[1][2],46);
  fonts.dispose();
});

test('local bold and italic faces are registered independently and shared by concurrent requests',async()=>{
  const loaded=[],options=fixture(async face=>{loaded.push(face);return face;});
  const fonts=new MseBrowserFonts({...options,allowSystemFonts:true});
  await Promise.all([fonts.prepare({name:'Verdana',size:12,bold:true}),fonts.prepare({name:'Verdana',size:14,bold:true})]);
  assert.equal(loaded.length,1);
  assert.ok(loaded[0].source.startsWith('local("Verdana Bold")'));
  assert.deepEqual(loaded[0].descriptors,{weight:'700',style:'normal'});
  await fonts.prepare({name:'Verdana',size:12,italic:true});
  await fonts.prepare({name:'Verdana',size:12,bold:true,italic:true});
  await fonts.prepare({name:'Verdana',size:12});
  assert.deepEqual(loaded.map(f=>f.descriptors),[
    {weight:'700',style:'normal'},{weight:'400',style:'italic'},
    {weight:'700',style:'italic'},{weight:'400',style:'normal'},
  ]);
  assert.equal(new Set(loaded.map(f=>f.family)).size,1);
  fonts.dispose();assert.equal(options.fontSet.size,0);
});

test('an unavailable local variant retains regular-face synthesis without repeating failed lookups',async()=>{
  const sources=[],options=fixture(async face=>{
    sources.push(face.source);if(face.descriptors.weight==='700')throw new Error('No bold face');return face;
  });
  const fonts=new MseBrowserFonts({...options,allowSystemFonts:true});
  const first=await fonts.prepare({name:'Regular Only',size:12,bold:true});
  const second=await fonts.prepare({name:'Regular Only',size:12,bold:true});
  assert.equal(first.measure('A',null,1).width,second.measure('A',null,1).width);
  assert.equal(sources.length,2);assert.equal(sources[1],'local("Regular Only")');
  assert.equal(options.fontSet.size,1);
  fonts.dispose();
});
