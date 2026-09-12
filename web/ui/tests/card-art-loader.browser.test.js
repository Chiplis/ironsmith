import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
const art='<svg xmlns="http://www.w3.org/2000/svg" width="180" height="252"><rect width="180" height="252" fill="#315e6a"/></svg>';
test('hand, battlefield and token loaders follow image loads, source changes and failures',async()=>{
 const server=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await server.listen();
 const browser=await chromium.launch();
 try {
  const page=await browser.newPage({viewport:{width:900,height:500}});const errors=[];page.on('pageerror',e=>errors.push(e.message));
  let release;const gate=new Promise(resolve=>{release=resolve;});
  await page.route('**/tests/art/first.svg',async route=>{await gate;await route.fulfill({contentType:'image/svg+xml',body:art});});
  await page.route('**/tests/art/second.svg',async route=>route.abort());
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/card-art-loader.html`, {waitUntil:'domcontentloaded'});
  await page.waitForFunction(()=>document.querySelectorAll('[data-card-art-loader="loading"]').length===3);
  assert.equal(await page.locator('[data-probe="hand"] [data-variant="hand"]').count(),1);
  assert.equal(await page.locator('[data-probe="mobile-token"] foreignObject [data-variant="battlefield"]').count(),1);
  assert.equal(await page.locator('.card-art-loader').first().evaluate(e=>getComputedStyle(e).pointerEvents),'none');
  for (const [mode, colors] of [['hand',['W','U','B','R','G']],['battlefield',['W','U']],['mobile-token',['R']]]) {
    assert.deepEqual(await page.locator(`[data-probe="${mode}"] [data-mana-color]`).evaluateAll(nodes=>nodes.map(node=>node.dataset.manaColor)),colors);
  }
  const contained = await page.locator('.card-art-loader').evaluateAll(loaders=>loaders.every(loader=>{
    const bounds=loader.getBoundingClientRect();
    return [...loader.querySelectorAll('[data-mana-color]')].every(symbol=>{
      const box=symbol.getBoundingClientRect();
      return box.width>0&&box.height>0&&box.left>=bounds.left&&box.top>=bounds.top&&box.right<=bounds.right&&box.bottom<=bounds.bottom;
    });
  }));
  assert.equal(contained,true);

  if (process.env.CARD_ART_LOADER_SCREENSHOT) await page.screenshot({path:process.env.CARD_ART_LOADER_SCREENSHOT});
  assert.equal(await page.locator('[data-probe="battlefield"] .card-art-loader__ring').evaluate(e=>getComputedStyle(e).animationName),'card-art-heartbeat');
  await page.emulateMedia({reducedMotion:'reduce'});
  assert.equal(await page.locator('[data-probe="battlefield"] .card-art-loader__ring').evaluate(e=>getComputedStyle(e).animationName),'none');
  await page.emulateMedia({reducedMotion:'no-preference'});
  release();
  await page.waitForFunction(()=>document.querySelectorAll('[data-card-art-loader]').length===0);
  await page.waitForFunction(()=>document.querySelectorAll('[data-card-art-state="revealing"]').length===3);
  assert.equal(await page.locator('.card-art-reveal-sheen').count(),3);
  await page.waitForFunction(()=>document.querySelectorAll('[data-card-art-state="loaded"]').length===3);
  assert.equal(await page.locator('.card-art-reveal-sheen').count(),0);
  await page.evaluate(()=>window.changeArt('/tests/art/second.svg'));
  await page.waitForFunction(()=>document.querySelectorAll('[data-card-art-loader="unavailable"]').length===3);
  assert.equal(await page.locator('[data-probe="battlefield"] .card-art-loader__ring').evaluate(e=>getComputedStyle(e).animationName),'none');
  await page.emulateMedia({reducedMotion:'reduce'});
  await page.evaluate(()=>window.changeArt('/tests/art/first.svg'));
  await page.waitForFunction(()=>document.querySelectorAll('[data-card-art-loader]').length===0);
  assert.equal(await page.locator('[data-card-art-state="revealing"]').count(),0);
  assert.equal(await page.locator('.card-art-reveal-sheen').count(),0);
  assert.deepEqual(errors,[]);
 }finally{await browser.close();await server.close();}
});
