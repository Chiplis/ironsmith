import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
test('priority-only renders do not refit card text; text and typography changes still do', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/phase-render-fit.html`);
    await page.locator('output').waitFor();
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(500);
    await page.evaluate(() => {
      window.fitReads = 0;
      const measure = Range.prototype.getBoundingClientRect;
      Range.prototype.getBoundingClientRect = function(...args) { window.fitReads++; return measure.apply(this,args); };
    });
    for (let i = 0; i < 5; i++) await page.getByRole('button',{name:'Advance phase'}).click();
    await page.waitForTimeout(100);
    assert.equal(await page.locator('output').textContent(),'5');
    assert.equal(await page.evaluate(() => window.fitReads),0,'unchanged cards must not force text layout');
    await page.getByRole('button',{name:'Change rules'}).click();
    assert.ok(await page.evaluate(() => window.fitReads) > 0,'changed rules are refitted');
    await page.waitForTimeout(100);
    await page.evaluate(() => { window.fitReads = 0; });
    await page.getByRole('button',{name:'Change typography'}).click();
    assert.ok(await page.evaluate(() => window.fitReads) > 0,'new sampled typography is refitted');
  } finally { await browser.close(); await vite.close(); }
});
