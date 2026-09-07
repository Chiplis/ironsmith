import test from 'node:test';
import process from 'node:process';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright';
import {createServer} from 'vite';

// Real scan regression without checking copyrighted images into the repo.
// The fixture directory contains printing.json, normal.jpg and art_crop.jpg.
const fixture=process.env.CARD_FRAME_FUTURE_FIXTURE;
test('Future Sight masks retain the original art, curved frame and off-title mana', {skip:!fixture,timeout:60000}, async()=>{
  const printing=JSON.parse(await readFile(join(fixture,'printing.json'),'utf8'));
  assert.equal(printing.frame,'future');
  const root=dirname(dirname(fileURLToPath(import.meta.url)));
  const vite=await createServer({root,server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:900,height:800}});
    const errors=[];page.on('pageerror',error=>errors.push(error.message));
    await page.addInitScript(p=>{window.__comparisonCards=[{...p,id:1}];},printing);
    await page.route('https://cards.scryfall.io/**',async route=>{
      assert.ok(route.request().url().includes(printing.id));
      const variant=route.request().url().includes('/art_crop/')?'art_crop':'normal';
      await route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(fixture,variant+'.jpg'))});
    });
    await page.route('https://api.scryfall.com/**',route=>route.fulfill({json:route.request().url().includes(printing.id)?printing:{}}));
    await page.route('https://svgs.scryfall.io/**',route=>route.abort());
    await page.route('**/cards/*.json',route=>route.fulfill({json:{scryfall:printing}}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
    const stage=page.locator('.interactive-card-frame-stage');
    await page.waitForFunction(()=>document.querySelector('[data-render-ready="true"]'));
    assert.equal(await stage.getAttribute('data-frame-mode'),'masked');
    assert.equal(await stage.locator('.original-card-fallback').count(),0);
    assert.equal(await stage.locator('.interactive-card-frame__mana').count(),0);
    const registration=await stage.evaluate(node=>({
      mana:JSON.parse(node.style.getPropertyValue('--printed-mana-symbols')),
      layout:JSON.parse(node.style.getPropertyValue('--printed-layout')),
      rulesSize:node.style.getPropertyValue('--printed-rules-font-size'),
      source:node.style.getPropertyValue('--source-frame-image'),
    }));
    assert.ok(registration.rulesSize,'symbol-led rules retain measured font sizing');
    assert.ok(registration.mana.symbols.every(s=>s.x<registration.layout.art.x&&s.y>registration.layout.title.y));
    assert.equal(await stage.locator('.interactive-card-frame__source-mana img').count(),registration.mana.symbols.length);
    const unchanged=await page.evaluate(async({source,url})=>{
      const load=async src=>{const img=new Image();img.crossOrigin='anonymous';img.src=src;await img.decode();return img;};
      const images=await Promise.all([load(source.slice(5,-2)),load(url)]);
      const scans=images.map(image=>{const canvas=document.createElement('canvas');canvas.width=488;canvas.height=680;const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0,488,680);return ctx.getImageData(0,0,488,680).data;});
      for(const [x,y,w,h] of [[110,130,280,230],[20,200,8,160],[190,32,200,4],[110,610,180,40]])
        for(let py=y;py<y+h;py++)for(let px=x;px<x+w;px++)for(let c=0;c<4;c++)if(scans[0][(py*488+px)*4+c]!==scans[1][(py*488+px)*4+c])return {x:px,y:py,c,masked:scans[0][(py*488+px)*4+c],original:scans[1][(py*488+px)*4+c]};
      return true;
    },{source:registration.source,url:printing.image_uris.normal});
    assert.equal(unchanged,true,JSON.stringify(unchanged));
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
