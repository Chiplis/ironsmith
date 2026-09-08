import test from 'node:test';
import assert from 'node:assert/strict';
import process from 'node:process';
import {readFile, mkdir} from 'node:fs/promises';
import {join} from 'node:path';
import {chromium} from 'playwright';
import {createServer} from 'vite';

const fixtures=process.env.CARD_FRAME_FIXTURES;
test('borderless title masks include the first capital on English and Spanish scans', {skip:!fixtures,timeout:120000}, async()=>{
  const cases=JSON.parse(await readFile(new URL('./card-frame-regression-cases.json',import.meta.url),'utf8'));
  const slugs=cases.map(c=>c.slug);
  const prints=await Promise.all(slugs.map(slug=>readFile(join(fixtures,slug+'.json'),'utf8').then(JSON.parse)));
  prints.forEach((printing,i)=>assert.equal(printing.id,cases[i].id,'use the exact regression scan'));
  const output=process.env.CARD_FRAME_AUDIT_OUTPUT;
  if(output)await mkdir(output,{recursive:true});
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();const browser=await chromium.launch();
  try {
    for(const printing of prints)for(const locale of ['en','es']) {
      const page=await browser.newPage({viewport:{width:900,height:720},reducedMotion:'reduce'});
      await page.addInitScript(({printing,locale})=>{
        localStorage.setItem('ironsmith.locale',locale);
        window.__comparisonCards=[{...printing,id:1,sourceImageUrl:printing.image_uris.normal}];
        window.__comparisonShowOriginals=true;
      },{printing,locale});
      await page.route('https://cards.scryfall.io/**',async route=>{
        const i=prints.findIndex(p=>route.request().url().includes(p.id));
        if(i<0)return route.abort();
        const variant=route.request().url().includes('/art_crop/')?'art_crop':'normal';
        await route.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(fixtures,slugs[i]+'-'+variant+'.jpg'))});
      });
      await page.route('https://api.scryfall.com/**',route=>{
        const url=route.request().url();
        const data=prints.find(p=>url.includes(p.id)) || prints.find(p=>url.endsWith(`/fdn/312/${p.lang}`));
        return route.fulfill({json:data||{}});
      });
      await page.route('https://svgs.scryfall.io/**',route=>route.abort());
      await page.route('**/cards/*.json',route=>route.fulfill({json:{}}));
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-comparison.html`);
      await page.locator('[data-render-ready="true"]').waitFor();
      const flavor=(locale==='en'?printing:prints.find(p=>p.lang===locale)).flavor_text;
      await page.waitForFunction(text=>document.querySelector('.inspector-flavor-text')?.textContent===text,flavor);
      const result=await page.evaluate(async()=>{
        const stage=document.querySelector('.interactive-card-frame-stage');
        const bounds=JSON.parse(stage.style.getPropertyValue('--printed-title-text-bounds'));
        const image=new Image();image.src=stage.style.getPropertyValue('--source-frame-image').slice(5,-2);await image.decode();
        const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;
        const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);
        // Independently measured capital O, outside the broken mask's old x=62.
        const data=ctx.getImageData(43,42,17,20).data;
        let white=0;for(let p=0;p<data.length;p+=4)if(Math.min(data[p],data[p+1],data[p+2])>190)white++;
        return {bounds,white};
      });
      assert.ok(result.bounds.x>=40&&result.bounds.x<=46,JSON.stringify(result));
      assert.ok(result.bounds.height>=18,JSON.stringify(result));
      assert.ok(result.white<10,'original white capital must be removed: '+JSON.stringify(result));
      assert.equal(await page.locator('.interactive-card-frame__title').textContent(),locale==='es'?'Omnisciencia':'Omniscience');
      if(output)await page.screenshot({path:join(output,`omniscience-source-${printing.lang}-ui-${locale}.png`),fullPage:true});
      await page.close();
    }
  } finally {await browser.close();await vite.close();}
});
