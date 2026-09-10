import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const source = await readFile(new URL('../src/lib/card-frame-preparation.js', import.meta.url), 'utf8');
async function fixture({lang='es', localMask=false, englishMask=true, reject=false}={}) {
  const calls=[];
  let release;
  const gate=new Promise(resolve=>{release=resolve;});
  globalThis.__frameMocks={
    resolveScryfallFlavorText:async()=> 'Translated flavor',
    resolveScryfallPrintingMetadata:async()=>({lang}),
    resolveScryfallSetSymbol:async()=>null,
    resolveScryfallEnglishPrinting:async()=>{calls.push('english');await gate;if(reject)throw Error('Offline');return {lang:'en',image_uris:{normal:'english'}};},
    fullCardImageUrl:url=>url||'', preloadCardFrameSource:async()=>{},
    sampleCardFrameColors:async url=>{calls.push(url);return (url==='localized'?localMask:englishMask)?{'--source-frame-image':`url("${url}-mask")`}:{'--source-frame-status':'original'};},
    registrationForImage:()=>null,registrationForPrinting:()=>null,registrationGeometryIsUsable:()=>true,
  };
  globalThis.Image=class {async decode(){calls.push(`decode:${this.src}`);}};
  const transformed=source.replace(/^import .*;\n/gm,'')
    .replace(/async function prepareTypography\(printing\) \{[\s\S]*?\n\}/,'async function prepareTypography(printing) { return {lang:printing.lang}; }')
    .replace("(await import('./card-region-catalog.generated.js')).default",'{}');
  const module=await import(`data:text/javascript,${encodeURIComponent(`const {${Object.keys(globalThis.__frameMocks).join(',')}}=globalThis.__frameMocks;\n${transformed}\n// ${Math.random()}`)}`);
  return {module,calls,release};
}
test('English retry is unpublished until masked and decoded; translated source and flavor survive',async()=>{
  const {module,calls,release}=await fixture();
  const pending=module.prepareCardFrame('localized');
  await new Promise(resolve=>setImmediate(resolve));
  assert.ok(calls.includes('english'));
  assert.equal(module.cachedCardFrame('localized'),null);
  release();
  const result=await pending;
  assert.equal(result.imageUrl,'localized');
  assert.equal(result.originalImageUrl,'localized');
  assert.equal(result.flavorText,'Translated flavor');
  assert.equal(result.typography.lang,'en');
  assert.equal(result.style['--source-frame-image'],'url("english-mask")');
  assert.ok(calls.includes('decode:english-mask'));
  assert.equal(module.cachedCardFrame('localized'),result);
});
for(const options of [{englishMask:false},{reject:true}])test(`failed retry preserves original ${JSON.stringify(options)}`,async()=>{
  const {module,release}=await fixture(options);release();
  const result=await module.prepareCardFrame('localized');
  assert.equal(result.originalImageUrl,'localized');
  assert.equal(result.typography.lang,'es');
  assert.equal(result.style['--source-frame-status'],'original');
});
for(const options of [{lang:'en'},{localMask:true}])test(`no unnecessary retry ${JSON.stringify(options)}`,async()=>{
  const {module,calls}=await fixture(options);
  await module.prepareCardFrame('localized');
  assert.ok(!calls.includes('english'));
});
