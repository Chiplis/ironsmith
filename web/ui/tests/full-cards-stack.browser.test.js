import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('full cards shares the desktop stack and retains it when appearance changes', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1440,height:900}});
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(() => localStorage.setItem('ironsmith.battlefieldAppearance', JSON.stringify({compactCards:false})));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?singleOpponent`, {waitUntil:'domcontentloaded'});
    await page.locator('[data-battlefield-style="full"]').first().waitFor();
    const pile = page.locator('[data-card-stack-pile]');
    await pile.waitFor();
    await page.locator('.card-stack-pager').waitFor();
    assert.equal(await page.locator('[data-card-stack-anchor]').count(), 1);
    assert.equal(await page.locator('.stack-timeline-entry').count(), 0);
    assert.equal(await page.locator('[data-automatic-stack-preview="true"][data-visible="true"]').count(), 0);
    await pile.evaluate(el => { el.dataset.continuityCheck = 'retained'; });
    await page.getByRole('button', {name:'Toggle battlefield style'}).click();
    await page.locator('[data-battlefield-style="compact"]').first().waitFor();
    assert.equal(await pile.getAttribute('data-continuity-check'), 'retained');
    await page.getByRole('button', {name:'Toggle battlefield style'}).click();
    await page.locator('[data-battlefield-style="full"]').first().waitFor();
    assert.equal(await pile.getAttribute('data-continuity-check'), 'retained');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
