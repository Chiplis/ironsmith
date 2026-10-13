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
