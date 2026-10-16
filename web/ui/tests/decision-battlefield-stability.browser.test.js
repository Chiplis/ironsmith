import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('decisions preserve battlefield sizing and option hover animates the matching hand card', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1440,height:900}});
    page.on('pageerror', error=>console.error(error));
    await page.route('https://**/*', route=>route.fulfill({status:404,body:''}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&stableDecisions&multiplayer`);
    const dock = page.locator('[data-human-action-dock]');
    await dock.waitFor({state:'visible'});
    await page.waitForTimeout(900);
    const sizes = () => page.locator('.battlefield-row').evaluateAll(rows=>rows.map(row=>({
      width: row.style.getPropertyValue('--bf-card-width'),
      height: row.style.getPropertyValue('--bf-card-height'),
      rowWidth: row.clientWidth, rowHeight: row.clientHeight,
    })));
    for (let appearance=0; appearance<2; appearance++) {
      const baseline = await sizes();
      assert.ok(baseline.length && baseline.every(row=>row.width && row.height));
      for (let step=1; step<=4; step++) {
        await page.getByRole('button',{name:'Next decision',exact:true}).click();
        await page.waitForTimeout(600);
        assert.deepEqual(await sizes(), baseline, `decision ${step} leaves battlefield geometry unchanged`);
        if (step===2) assert.ok(await dock.locator('.mana-payment-editor').isVisible(), 'payment costs are rendered');
        if (step===1) {
          const card = page.locator('.hand-layout-item .game-card').first();
          const resting = await card.boundingBox();
          await dock.getByRole('button',{name:'Ornithopter',exact:true}).hover();
          await page.waitForTimeout(400);
          const lifted = await card.boundingBox();
          assert.ok(lifted.height > resting.height * 1.2 && lifted.y < resting.y, `option hover lifts and enlarges hand card ${JSON.stringify({resting,lifted,html:await card.evaluate(n=>n.closest('.hand-layout-item').outerHTML.slice(0,1200))})}`);
          await page.mouse.move(4,4);
          await page.waitForTimeout(500);
          const reset = await card.boundingBox();
          assert.ok(Math.abs(reset.height-resting.height)<2, 'leaving option restores hand card');
          await card.hover({force:true,position:{x:60,y:40}});
          await page.mouse.move(reset.x + 61, reset.y + 41);
          await page.waitForTimeout(400);
          const direct = await card.boundingBox();
          assert.ok(Math.abs(direct.height-lifted.height)<2, `option and direct hover use the same enlargement ${JSON.stringify({direct,lifted})}`);
          await page.mouse.move(4,4);
        }
      }
      await page.getByRole('button',{name:'Toggle battlefield style'}).click();
      await page.waitForTimeout(600);
    }
  } finally { await browser.close(); await vite.close(); }
});
