import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
test('two, three and four player opponent boards keep every compact card inside its panel', {timeout:180000}, async () => {
 const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
 await vite.listen();
 const browser = await chromium.launch({args:['--disable-webgl']});
 try {
  const page = await browser.newPage({viewport:{width:1440,height:900}});
  const battlefield = ['Forest','Island','Mountain','Plains','Swamp','Tropical Island','Volcanic Island','Myr Moonvessel','Ornithopter','Yawgmoth, Thran Physician','Omniscience'];
  for (const count of [2,3,4]) {
   const puzzle = {version:1,players:Array.from({length:count},(_,i)=>({name:`Player ${i+1}`,life:20,zones:{battlefield,library:['Forest'],hand:['Island']}}))};
   await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/?puzzle=${Buffer.from(JSON.stringify(puzzle)).toString('base64url')}`);
   await page.waitForSelector('.battlefield-row[data-bf-side="top"] .game-card', {timeout:90000});
   for (const viewport of [{width:1440,height:900},{width:1280,height:720}]) {
    await page.setViewportSize(viewport);
    await page.waitForTimeout(1800);
    const result = await page.locator('.battlefield-row[data-bf-side="top"]').evaluateAll(rows => rows.map(row => {
     const panel = row.closest('.battlefield-subpanel').getBoundingClientRect();
     const cards = [...row.querySelectorAll('.game-card')];
     return {count:cards.length,cutoff:cards.filter(card=> {const r=card.getBoundingClientRect();return r.bottom>panel.bottom+1 || r.top<panel.top-1 || r.left<panel.left-1 || r.right>panel.right+1;}).map(card=>card.dataset.objectId)};
    }));
    assert.equal(result.length,count-1);
    for (const row of result) { assert.ok(row.count>=10); assert.deepEqual(row.cutoff,[],`${count} players at ${viewport.width}: clipped cards`); }
    if (count > 2) {
     const sizes = await page.evaluate(() => {
      const tile = side => document.querySelector(`.battlefield-row[data-bf-side="${side}"] .arena-permanent[data-kind="creature"]`).getBoundingClientRect().width;
      return {own:tile('bottom'),opponent:tile('top')};
     });
     assert.ok(sizes.own >= sizes.opponent, JSON.stringify(sizes));
     assert.ok(sizes.own / sizes.opponent < 1.5, `multiplayer cards should stay comparable: ${JSON.stringify(sizes)}`);
     const seam = await page.evaluate(() => {
      const bounds = side => [...document.querySelectorAll(`.battlefield-row[data-bf-side="${side}"] .arena-permanent[data-kind="creature"]`)].map(el=>el.getBoundingClientRect());
      return Math.min(...bounds('bottom').map(r=>r.top)) - Math.max(...bounds('top').map(r=>r.bottom));
     });
     assert.ok(seam >= 0 && seam < 100, `front ranks should use the center space: ${seam}`);
     const clearance = await page.evaluate(() => {
      const bottom = Math.max(...[...document.querySelectorAll('.battlefield-row[data-bf-side="bottom"] .game-card')].map(el=>el.getBoundingClientRect().bottom));
      const top = Math.min(...[...document.querySelectorAll('.phase-track')].map(el=>el.getBoundingClientRect()).filter(r=>r.width>0).map(r=>r.top));
      return top-bottom;
     });
     assert.ok(clearance >= 10, `phase tracker clearance: ${clearance}`);
     await page.screenshot({path:`/tmp/multiplayer-balanced-${count}-${viewport.width}.png`});
    }
   }
  }
 } finally {await browser.close();await vite.close();}
});
