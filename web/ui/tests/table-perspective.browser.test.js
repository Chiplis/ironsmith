import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
test('grouped cards tilt while fallback background and labels remain flat', {timeout:60000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'}); await vite.listen();
 const browser=await chromium.launch({args:['--disable-webgl']});
 try {
  const page=await browser.newPage({viewport:{width:1440,height:900}});
  await page.route('https://**/*',r=>r.fulfill({status:404,body:''}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/mobile-arena-cards.html`, {waitUntil:'domcontentloaded'});
  await page.locator('.arena-permanent').first().waitFor();
  await page.evaluate(()=>{
   document.querySelector('main').dataset.forgeBoard='true';
   const board=document.createElement('div');board.className='forge-board';board.style.zIndex='-1';document.querySelector('main').prepend(board);
  });
  const transforms=await page.evaluate(()=>({
   background:getComputedStyle(document.querySelector('.forge-board'),'::before').transform,
   card:getComputedStyle(document.querySelector('.game-card-surface')).transform,
   tapped:[...document.querySelectorAll('.tapped > .game-card-surface, .tapped > .battlefield-group-stack')].map(el=>getComputedStyle(el).transform),
   label:getComputedStyle(document.querySelector('figcaption')).transform,
  }));
  assert.equal(transforms.background,'none');assert.match(transforms.card,/matrix3d/);
  assert.equal(transforms.tapped[0],transforms.tapped[1]);assert.equal(transforms.label,'none');
  await page.screenshot({path:'/tmp/angled-card-layers.png'});
 }finally{await browser.close();await vite.close();}
});
