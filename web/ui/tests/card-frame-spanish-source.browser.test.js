import test from 'node:test';
import assert from 'node:assert/strict';
import process from 'node:process';
import {readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {chromium} from 'playwright';
import {createServer} from 'vite';
const fixture=process.env.CARD_FRAME_SPANISH_FIXTURE;
test('English mask stays identical in Spanish, with clean type text and translated flavor',{skip:!fixture,timeout:90000},async()=>{
  const printing=JSON.parse(await readFile(join(fixture,'printing.json')));
  const localized=JSON.parse(await readFile(join(fixture,'localized.json')));
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
  const browser=await chromium.launch();
  try {
    const masks=[];
    for(const locale of ['en','es']) {
      const page=await browser.newPage({viewport:{width:900,height:700},reducedMotion:'reduce'});
      await page.addInitScript(({printing,locale})=>{
        localStorage.setItem('ironsmith.locale',locale);
        window.__comparisonCards=[{...printing,id:1,sourceImageUrl:printing.image_uris.normal}];
        window.__comparisonShowOriginals=true;
      },{printing,locale});
      await page.route('https://cards.scryfall.io/**',async route=>route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(fixture,route.request().url().includes('/art_crop/')?'art_crop.jpg':'normal.jpg'))}));
      await page.route('https://api.scryfall.com/**',route=>route.fulfill({json:route.request().url().endsWith('/es')?localized:route.request().url().includes(printing.id)?printing:{}}));
      await page.route('https://svgs.scryfall.io/**',route=>route.abort());
      await page.route('**/cards/*.json',route=>route.fulfill({json:{}}));
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
      await page.locator('[data-render-ready="true"]').waitFor();
      const expected=locale==='es'?localized.flavor_text:printing.flavor_text;
      await page.waitForFunction(text=>document.querySelector('.inspector-flavor-text')?.textContent===text,expected);
      assert.equal(await page.locator('.interactive-card-frame__type').textContent(),locale==='es'?localized.printed_type_line:printing.type_line);
      masks.push(await page.locator('.interactive-card-frame-stage').evaluate(el=>el.style.getPropertyValue('--source-frame-image')));
      assert.ok(masks.at(-1));
      const coverage=await page.evaluate(async()=>{
        const stage=document.querySelector('.interactive-card-frame-stage');
        const bounds=JSON.parse(stage.style.getPropertyValue('--printed-type-text-bounds'));
        const load=async src=>{const img=new Image();img.src=src;await img.decode();const c=document.createElement('canvas');c.width=img.width;c.height=img.height;const ctx=c.getContext('2d');ctx.drawImage(img,0,0);return ctx.getImageData(bounds.x,bounds.y,bounds.width,bounds.height).data;};
        const original=await load(document.querySelector('img[alt="Original printing"]').src);
        const masked=await load(stage.style.getPropertyValue('--source-frame-image').slice(5,-2));
        let ink=0,left=0;for(let p=0;p<original.length;p+=4)if(original[p]+original[p+1]+original[p+2]<270){ink++;if(masked[p]+masked[p+1]+masked[p+2]<270)left++;}
        return {ink,left};
      });
      assert.ok(coverage.ink>20,JSON.stringify(coverage));
      assert.ok(coverage.left/coverage.ink<.03,JSON.stringify(coverage));
      if(locale==='es')await page.screenshot({path:join(fixture,'spanish-fixed.png'),fullPage:true});
      await page.close();
    }
    assert.equal(masks[0],masks[1],'UI locale must not affect source-mask pixels');
  } finally {await browser.close();await vite.close();}
});
