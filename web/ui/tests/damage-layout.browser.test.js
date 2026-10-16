import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
test('compact battlefield shows loyalty once and marked damage above P/T',async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
 try {const page=await browser.newPage({viewport:{width:1100,height:700}});await page.route('https://**/*',r=>r.fulfill({status:404,body:''}));
 await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/damage-layout.html`);
 const cards=page.locator('.game-card');await cards.nth(2).waitFor();
 assert.equal(await cards.nth(0).locator('.arena-permanent-stat').innerText(),'5');
 assert.equal(await cards.nth(0).locator('.battlefield-counter-rail').count(),0);
 assert.equal(await cards.nth(1).locator('.battlefield-damage-badge').innerText(),'1');
 assert.equal(await cards.nth(2).locator('.battlefield-damage-badge').count(),0);
 const damage=await cards.nth(1).locator('.battlefield-damage-badge').boundingBox();const pt=await cards.nth(1).locator('.arena-permanent-pt').boundingBox();
 assert.ok(damage.y+damage.height<=pt.y,'damage sits above P/T');
 await page.screenshot({path:'/tmp/ironsmith-damage-layout.png'});
 }finally{await browser.close();await vite.close();}
});
