import test from 'node:test';
import assert from 'node:assert/strict';
import process from 'node:process';
import {readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
import {createServer} from 'vite';

const cache=process.env.FRAME_HISTORY_CACHE;
const id='b90faa91-3173-4898-a1fa-0b8e7ce35c72';
test('the store-championship rules box removes first-line ascenders without erasing its rim',{skip:!cache,timeout:90000},async()=>{
  const printing=JSON.parse(await readFile(join(cache,id+'.json'),'utf8'));
  const vite=await createServer({root:fileURLToPath(new URL('../',import.meta.url)),server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();const browser=await chromium.launch();
  try{
    const page=await browser.newPage();
    await page.addInitScript(p=>{window.__comparisonCards=[{...p,id:1,sourceImageUrl:p.image_uris.normal}];},printing);
    await page.route('https://cards.scryfall.io/**',async route=>route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(cache,id+'-'+(route.request().url().includes('/art_crop/')?'art_crop':'normal')+'.jpg'))}));
    await page.route('https://api.scryfall.com/**',route=>route.fulfill({json:route.request().url().includes(id)?printing:{}}));
    await page.route('https://svgs.scryfall.io/**',route=>route.abort());
    await page.route('**/cards/*.json',route=>route.fulfill({json:{scryfall:printing}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`,{waitUntil:'domcontentloaded'});
    await page.waitForFunction(()=>document.querySelector('[data-render-ready="true"]'));
    const stage=page.locator('.interactive-card-frame-stage');
    assert.equal(await stage.getAttribute('data-frame-mode'),'masked');
    const checked=await stage.evaluate(async(node,url)=>{
      const load=async src=>{const image=new Image();image.crossOrigin='anonymous';image.src=src;await image.decode();const canvas=document.createElement('canvas');canvas.width=488;canvas.height=680;const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0,488,680);return ctx.getImageData(0,0,488,680).data;};
      const [clean,original]=await Promise.all([load(node.style.getPropertyValue('--source-frame-image').slice(5,-2)),load(url)]);
      let ink=0,residual=0,rimChanges=0;
      // Independently inspected first-line coordinates in the pinned scan.
      // In particular, y=430/431 lie above the former five-pixel crop inset.
      for(let y=430;y<450;y++)for(let x=43;x<438;x++){
        const p=(y*488+x)*4;
        if(Math.min(...original.subarray(p,p+3))>190){ink++;if(Math.min(...clean.subarray(p,p+3))>190)residual++;}
      }
      for(let y=421;y<427;y++)for(let x=100;x<400;x++)for(let c=0;c<4;c++)if(clean[(y*488+x)*4+c]!==original[(y*488+x)*4+c])rimChanges++;
      return {ink,residual,rimChanges};
    },printing.image_uris.normal);
    assert.ok(checked.ink>300,JSON.stringify(checked));
    assert.ok(checked.residual/checked.ink<.01,JSON.stringify(checked));
    assert.equal(checked.rimChanges,0,'the original upper rules rim is unchanged');
  }finally{await browser.close();await vite.close();}
});
