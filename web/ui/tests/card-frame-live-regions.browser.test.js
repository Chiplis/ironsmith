import test from 'node:test';
import assert from 'node:assert/strict';
import process from 'node:process';
import {readFile,mkdir} from 'node:fs/promises';
import {join} from 'node:path';
import {chromium} from 'playwright';
import {createServer} from 'vite';
const fixture=process.env.CARD_FRAME_FIXTURES;
test('outlined and prepared-spell scans mask live rules in independent clickable regions',{skip:!fixture,timeout:180000},async()=>{
  const cases=JSON.parse(await readFile(new URL('./card-frame-regression-cases.json',import.meta.url))).filter(c=>c.families.includes('user-dynamic-regression'));
  const output=process.env.CARD_FRAME_AUDIT_OUTPUT;if(output)await mkdir(output,{recursive:true});
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
  try {
    for(const c of cases) {
      const printing=JSON.parse(await readFile(join(fixture,c.slug+'.json')));
      const page=await browser.newPage({viewport:{width:800,height:1100}});
      await page.addInitScript(data=>{window.__regionFixture=data;localStorage.setItem('ironsmith.locale','es');},{registration:c.registration,printing});
      await page.route('https://cards.scryfall.io/**',async r=>r.fulfill({contentType:'image/jpeg',headers:{'Access-Control-Allow-Origin':'*'},body:await readFile(join(fixture,c.slug+'-normal.jpg'))}));
      await page.route('https://api.scryfall.com/**',r=>r.fulfill({json:{}}));
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-frame-live-regions.html`);
      const count=c.registration.fields.filter(f=>f.kind==='rule').length;
      await page.waitForFunction(count=>document.querySelectorAll('[data-field-kind="rule"][data-replaced="true"]').length===count,count);
      for(const width of [240,420,740]) {
        await page.locator('[data-live-frame]').evaluate((el,w)=>{el.style.width=w+'px';el.style.height=w*680/488+'px';},width);
        await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
        const boxes=await page.locator('[data-field-kind="rule"]').evaluateAll(nodes=>nodes.map(n=>{
          const r=n.getBoundingClientRect(),text=n.querySelector('.interactive-card-frame__rules');
          return {x:r.x,y:r.y,right:r.right,bottom:r.bottom,overflow:text.scrollWidth-text.clientWidth,vertical:text.scrollHeight-text.clientHeight};
        }));
        for(const box of boxes){assert.ok(box.overflow<=1,JSON.stringify(box));assert.ok(box.vertical<=1,JSON.stringify(box));}
        if(c.layout==='prepare')assert.ok(boxes[0].right<=boxes[1].x+1,'prepared spell must keep its own column');
        for(let i=0;i<count;i++) {
          await page.locator('[data-field-kind="rule"] .registered-card-frame__action').nth(i).click();
          assert.equal(await page.evaluate(()=>window.__activated),i+1);
        }
      }
      if(output)await page.screenshot({path:join(output,c.slug+'-live.png'),fullPage:true});
      await page.close();
    }
  } finally {await browser.close();await vite.close();}
});
