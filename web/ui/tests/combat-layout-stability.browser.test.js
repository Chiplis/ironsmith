import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
test('combat through second main preserves battlefield geometry', async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();const browser=await chromium.launch();
 try {for(const compactCards of [true,false]) {
 for (const width of [1280,1440]) {
 const page=await browser.newPage({viewport:{width,height:900}});
 await page.addInitScript(value=>localStorage.setItem('ironsmith.battlefieldAppearance',JSON.stringify({compactCards:value,compactLands:true})),compactCards);
 await page.route('https://**/*',r=>r.abort());
 await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&combatCycle${width === 1280 ? "&singleOpponent" : ""}`);
 await page.locator('.battlefield-row').first().waitFor();await page.waitForTimeout(900);
 const sizes=()=>page.locator('.battlefield-row').evaluateAll(rows=>rows.map(r=>({w:r.style.getPropertyValue('--bf-card-width'),h:r.style.getPropertyValue('--bf-card-height'),height:r.clientHeight,padding:getComputedStyle(r).paddingTop,gap:getComputedStyle(r).rowGap})));
 const initial=await sizes();
 for(let step=1;step<=4;step++) {await page.getByRole('button',{name:'Next decision',exact:true}).click();await page.waitForTimeout(700);assert.deepEqual(await sizes(),initial,`style ${compactCards}, step ${step}`);}
 await page.close();
 }}}finally{await browser.close();await vite.close();}
});
