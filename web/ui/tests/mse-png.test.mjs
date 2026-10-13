import test from 'node:test';
import assert from 'node:assert/strict';
import { decodePng } from '../src/lib/mse/png-decoder.mjs';
import { createMseImageLoader } from '../src/lib/mse/browser-images.mjs';
const fixtures = [{"mode": "RGBA", "png": "iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAAEUlEQVR4nGP8xSXCoCInVw8ACkUB+aKD+ZsAAAAASUVORK5CYII=", "rgba": [250, 10, 20, 0, 30, 40, 50, 127]}, {"mode": "RGB", "png": "iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAIAAAB7QOjdAAAAD0lEQVR4nGNgZGL+9/cPAAYTAv4+Vsb6AAAAAElFTkSuQmCC", "rgba": [1, 2, 3, 255, 254, 253, 252, 255]}, {"mode": "LA", "png": "iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAQAAABeK7cBAAAADUlEQVR4nGNgYjjxHwACnAHK+5op/gAAAABJRU5ErkJggg==", "rgba": [2, 2, 2, 0, 200, 200, 200, 255]}, {"mode": "L", "png": "iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAAAAADRSSBWAAAAC0lEQVR4nGNgPgEAANEAzGvDJH8AAAAASUVORK5CYII=", "rgba": [3, 3, 3, 255, 200, 200, 200, 255]}, {"mode": "1", "png": "iVBORw0KGgoAAAANSUhEUgAAAAIAAAABAQAAAADcWUInAAAACklEQVR4nGNwAAAAQgBBKTf07wAAAABJRU5ErkJggg==", "rgba": [0, 0, 0, 255, 255, 255, 255, 255]}, {"mode": "palette", "png": "iVBORw0KGgoAAAANSUhEUgAAAAIAAAABAQMAAADO7O3JAAAABlBMVEX/AAAA/wDSh+9xAAAAAnRSTlMAgJsrThgAAAAKSURBVHicY3AAAABCAEEpN/TvAAAAAElFTkSuQmCC", "rgba": [255, 0, 0, 0, 0, 255, 0, 128]}];
test('PNG decoder preserves raw RGB and decodes palette, packed grayscale and alpha', async () => {
  for (const fixture of fixtures) {
    const image = await decodePng(Buffer.from(fixture.png,'base64'));
    assert.deepEqual([...image.data],fixture.rgba,fixture.mode);
    assert.equal(image.width,2);
    assert.equal(image.height,1);
  }
});
test('PNG decoder rejects corruption and allocation overruns', async () => {
  const bytes=Buffer.from(fixtures[0].png,'base64');
  await assert.rejects(()=>decodePng(bytes,{maxPixels:1}),{code:'image-budget'});
  bytes[bytes.length-1]^=1;
  await assert.rejects(()=>decodePng(bytes),{code:'image-png'});
});
test('browser image loading coalesces requests and retries failed fetches', async () => {
  let count=0;
  const load=createMseImageLoader({assetUrl:p=>p.key},{fetcher:async()=>{
    count++;
    return count===1?new Response('',{status:404}):new Response(Buffer.from(fixtures[0].png,'base64'));
  }});
  const path={key:'example.mse-style/frame.png'};
  await assert.rejects(()=>load(path),{code:'image-fetch'});
  const [a,b]=await Promise.all([load(path),load(path)]);
  assert.equal(a,b);
  assert.equal(count,2);
});

test('browser JPEG decoding returns opaque pixels and releases resources on budget failure',async()=>{
  let closed=0,drawn=0;
  const options={fetcher:async()=>({ok:true,arrayBuffer:async()=>new Uint8Array([255,216,255,224]).buffer}),
    decodeBitmap:async(blob,settings)=>{assert.equal(blob.type,'image/jpeg');assert.equal(settings.colorSpaceConversion,'none');return {width:2,height:1,close:()=>closed++};},
    createCanvas:()=>({getContext:()=>({drawImage:()=>drawn++,getImageData:()=>({data:new Uint8ClampedArray([1,2,3,255,4,5,6,255])})})})};
  const image=await createMseImageLoader({assetUrl:p=>p.key},options)({key:'frame.jpg'});
  assert.equal(image.hasAlpha,false);assert.equal(image.width,2);assert.equal(closed,1);assert.equal(drawn,1);
  await assert.rejects(createMseImageLoader({assetUrl:p=>p.key},{...options,maxPixels:1})({key:'frame.jpg'}),{code:'image-budget'});
  assert.equal(closed,2);assert.equal(drawn,1);
});

// Three-pixel rows encoded with None, Sub, Up, Average, then Paeth.
test('PNG decoder reconstructs all five row filters with upper-left samples',async()=>{
  const image=await decodePng(Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAMAAAAFCAYAAACAcVaiAAAAMklEQVR4nGNgUPXKn7LzHrOGb9F0RtOo+iVToIDJFAkwZ9X2L0p9+jQVCJ6yQMSmgEkACeMW/n2iJUAAAAAASUVORK5CYII=','base64'));
  assert.equal(image.width,3);assert.equal(image.height,5);
  assert.deepEqual([...image.data],Array.from({length:60},(_,i)=>((i%12)*37+Math.floor(i/12)*53)%256));
});
