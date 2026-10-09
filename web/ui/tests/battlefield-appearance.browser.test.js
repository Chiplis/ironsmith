import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';
test('full cards restore original row spacing and the preference survives sessions', {timeout:60000}, async()=>{
 const vite=await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});await vite.listen();
 const browser=await chromium.launch({args:['--disable-webgl']});
 try {
  let context=await browser.newContext({viewport:{width:1440,height:900}});
  let page=await context.newPage();
  const url=`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html`;
  await page.goto(url,{waitUntil:'domcontentloaded'});
  await page.locator('.battlefield-row[data-battlefield-style="compact"]').first().waitFor();
  await page.getByText('Toggle battlefield style',{exact:true}).click();
  await page.locator('.battlefield-row[data-battlefield-style="full"]').first().waitFor();
  const check=async()=>{
   const gaps=await page.locator('.battlefield-row[data-bf-side="top"]').evaluateAll(rows=>rows.map(row=>{
    const s=getComputedStyle(row);return {style:row.dataset.battlefieldStyle,x:parseFloat(s.columnGap),y:parseFloat(s.rowGap),scale:parseFloat(s.getPropertyValue('--bf-row-scale-gap'))||0};
   }));
   assert.ok(gaps.length>0);
   for(const gap of gaps){assert.equal(gap.style,'full');assert.equal(gap.x,20);assert.equal(gap.y,20+gap.scale);}
  };
  await check();
  await page.reload({waitUntil:'domcontentloaded'});
  await page.locator('.battlefield-row[data-battlefield-style="full"]').first().waitFor();await check();
  const storageState=await context.storageState();await context.close();
  context=await browser.newContext({storageState,viewport:{width:1440,height:900}});page=await context.newPage();
  await page.goto(url,{waitUntil:'domcontentloaded'});
  await page.locator('.battlefield-row[data-battlefield-style="full"]').first().waitFor();await check();
  await page.getByText('Toggle battlefield style',{exact:true}).click();
  await page.locator('.battlefield-row[data-battlefield-style="compact"]').first().waitFor();
  assert.equal(await page.evaluate(()=>JSON.parse(localStorage.getItem('ironsmith.battlefieldAppearance')).compactCards),true);
 }finally{await browser.close();await vite.close();}
});
