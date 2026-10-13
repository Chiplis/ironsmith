import test from 'node:test';
import assert from 'node:assert/strict';
import {createBrowserArtworkLoader} from '../src/lib/mse/browser-artwork.mjs';

function fixture(overrides={}){
  const calls=[],closed=[];
  const options={
    fetcher:async(url,{signal})=>{calls.push({url,signal});return {ok:true,blob:async()=>url};},
    createBitmap:async url=>({width:2,height:2,close(){closed.push(url);}}),
    createCanvas:()=>({getContext:()=>({drawImage(){},getImageData:()=>({data:new Uint8ClampedArray(16)})})}),
    ...overrides,
  };
  return {loader:createBrowserArtworkLoader(options),calls,closed};
}
test('artwork coalesces requests, closes decoded bitmaps, and evicts least recently used pixels',async()=>{
  const {loader,calls,closed}=fixture({maxCacheBytes:32});
  const [a,b]=await Promise.all([loader.load('a'),loader.load('a')]);
  assert.equal(a,b);assert.equal(calls.length,1);assert.deepEqual(closed,['a']);
  await loader.load('b');assert.equal(await loader.load('a'),a);
  await loader.load('c');await loader.load('a');await loader.load('b');
  assert.deepEqual(calls.map(c=>c.url),['a','b','c','b']);
  loader.dispose();assert.ok(calls.every(c=>c.signal.aborted));
  await assert.rejects(loader.load('a'),{code:'render-aborted'});
});
test('artwork retries failed fetches and does not retain images larger than its cache',async()=>{
  let attempts=0;
  const {loader}=fixture({maxCacheBytes:8,fetcher:async()=>({ok:++attempts>1,status:503,blob:async()=>''})});
  await assert.rejects(loader.load('a'),{code:'artwork-fetch'});
  await loader.load('a');await loader.load('a');assert.equal(attempts,3);loader.dispose();
});
test('session cancellation during image decoding closes late bitmaps and rejects their result',async()=>{
  let release,entered;const gate=new Promise(r=>release=r),ready=new Promise(r=>entered=r);
  const signal=new AbortController();let closed=0;
  const {loader}=fixture({signal:signal.signal,createBitmap:async()=>{entered();await gate;return {width:2,height:2,close(){closed++;}};}});
  const pending=loader.load('a'),rejected=assert.rejects(pending,{code:'render-aborted'});
  await ready;signal.abort();release();await rejected;assert.equal(closed,1);
});
test('pixel limits reject oversized artwork while releasing its bitmap',async()=>{
  const {loader,closed}=fixture({maxPixels:3});
  await assert.rejects(loader.load('a'),{code:'image-size'});assert.deepEqual(closed,['a']);loader.dispose();
});
