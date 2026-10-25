import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('hand frame repaints live casting costs while the printed card stays unchanged', {timeout:90000}, async()=>{
  const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();
  const browser=await chromium.launch();
  try {
    const page=await browser.newPage({viewport:{width:500,height:680}}),errors=[];
    page.on('pageerror',error=>errors.push(error.message));
    await page.route('https://api.scryfall.com/**',route=>route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-mana-cost.html`);
    await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:60000});
    assert.equal(await page.locator('.game-card').getAttribute('data-hand-mana-cost'),'{2}{U}','printed frame is available while analysis is unresolved');
    await page.evaluate(()=>{window.originalHandPixels=document.querySelector('[data-card-glint-frame="ready"]').toDataURL();});
    await page.waitForFunction(()=>Boolean(window.resolveHandPrice));
    await page.evaluate(()=>window.resolveHandPrice('{U}'));
    await page.waitForFunction(()=>document.querySelector('.game-card')?.dataset.handManaCost==='{U}');
    for(const cost of ['{U}','{3}{U}','{0}',null]){
      await page.evaluate(cost=>window.setHandManaCost(cost),cost);
      await page.waitForFunction(cost=>document.querySelector('.game-card')?.dataset.handManaCost===(cost??'{2}{U}'),cost);
      await page.waitForFunction(cost=>{
        const frame=document.querySelector('[data-displayed-mana-cost]');
        return frame?.dataset.displayedManaCost===(cost??'{2}{U}')&&frame.querySelector('[data-card-glint-frame="ready"]');
      },cost);
      assert.equal(await page.evaluate(()=>window.printedHandCard.mana_cost),'{2}{U}');
      const samePixels=await page.evaluate(()=>document.querySelector('[data-card-glint-frame="ready"]').toDataURL()===window.originalHandPixels);
      assert.equal(samePixels,cost===null,'only mana-cost changes repaint the hand face');
      if(cost==='{U}') await page.screenshot({path:'../../output/ui-feedback/hand-reduced-mana-cost.png'});
    }
    assert.deepEqual(errors,[]);
  } finally {await browser.close();await vite.close();}
});
