import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
test('compact cards retain sickness coloring and typed counter icons', {timeout:60000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch();
 try{
  const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
  await page.route('https://**/*',r=>r.fulfill({status:404,body:''}));
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/compact-card-status.html`,{waitUntil:'domcontentloaded'});
  const pt=page.locator('.arena-permanent-stat').first();await pt.waitFor();
  assert.equal(await pt.evaluate(el=>getComputedStyle(el).backgroundColor),'rgb(102, 102, 102)');
  assert.match(await pt.getAttribute('aria-label'),/summoning sickness/);
  assert.equal(await page.locator('.game-card').first().locator('.battlefield-counter-chip').count(),3);
  assert.ok(await page.locator('.battlefield-counter-icon, .battlefield-counter-symbol').count()>=2);
  assert.equal(await page.locator('.arena-permanent-counter').count(),0,'typed counters replace the anonymous total');
  await page.getByText('Change sickness',{exact:true}).click();
  assert.equal(await pt.getAttribute('data-summoning-sick'),null);
  assert.equal(await pt.evaluate(el=>getComputedStyle(el).backgroundColor),'rgb(217, 223, 223)');
  const shapes=await page.locator('#shape-fixture .game-card').evaluateAll(cards=>cards.map(card=>{
   const rect=el=>{const r=el.getBoundingClientRect();return {left:r.left,top:r.top,width:r.width,height:r.height}};
   return {face:rect(card.querySelector('.arena-permanent')),glow:rect(card.querySelector('.card-inspector-source-glow')),
    background:getComputedStyle(card).backgroundColor,surface:getComputedStyle(card.querySelector('.game-card-surface')).backgroundColor};
  }));
  const shadows=await page.evaluate(()=>window.measureCardShapes().cardShadows);
  assert.equal(shadows.length,3);
  shapes.forEach((shape,i)=>{
   assert.equal(shape.background,'rgba(0, 0, 0, 0)');
   assert.equal(shape.surface,'rgba(0, 0, 0, 0)');
   for(const key of ['left','top','width','height']) assert.ok(Math.abs(shape.face[key]-shape.glow[key])<1,`glow ${i} ${key} follows the face`);
   assert.ok(Math.abs(shadows[i].right-shadows[i].left-shape.face.width)<1,'shadow follows visible width');
   assert.ok(Math.abs(shadows[i].bottom-shadows[i].top-shape.face.height)<1,'shadow follows visible height');
  });
  await page.screenshot({path:'/tmp/compact-card-status.png'});
  assert.deepEqual(errors,[]);
 }finally{await browser.close();await vite.close();}
});
