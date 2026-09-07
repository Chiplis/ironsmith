import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('reopened frames reuse completed assets and never flash the raw image', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('https://cards.scryfall.io/**', async route => {
      if (route.request().url().includes('/art_crop/')) await new Promise(resolve => setTimeout(resolve, 700));
      await route.fulfill({ contentType: 'image/svg+xml', headers: { 'access-control-allow-origin': '*' }, body: '<svg xmlns="http://www.w3.org/2000/svg" width="488" height="684"><rect width="488" height="684" fill="#917659"/></svg>' });
    });
    await page.route('https://api.scryfall.com/**', route => route.fulfill({ status: 404, body: '' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-preview-source.html`);
    const card = page.getByAltText('Field card 1');
    await card.hover();
    await page.locator('.card-frame-art-preview').waitFor();
    const stage = page.locator('.interactive-card-frame-stage');
    await page.waitForFunction(() => document.querySelector('.interactive-card-frame-stage')?.dataset.renderReady === 'true');
    assert.equal(await stage.getAttribute('data-frame-reused'), 'false');
    assert.equal(await page.evaluate(() => Boolean(window.readCachedFrame())), true);
    await page.evaluate(() => { window.firstFrame = window.readCachedFrame(); });
    const detailRequests = await page.evaluate(() => window.detailRequests);
    for (let reopen = 0; reopen < 2; reopen++) {
      await page.mouse.move(1100, 700);
      await page.getByRole('button', { name: 'Clear hover' }).evaluate(button => button.click());
      await stage.waitFor({ state: 'detached' });
      await page.evaluate(() => {
        window.rawImageSeen = false;
        window.reopenObserver = new MutationObserver(records => {
          if (records.some(record => [...record.addedNodes].some(node => node.nodeType === 1 && (node.matches('.card-frame-art-preview') || node.querySelector('.card-frame-art-preview'))))) window.rawImageSeen = true;
        });
        window.reopenObserver.observe(document.body, { childList: true, subtree: true });
      });
      await card.hover();
      await page.waitForFunction(() => document.querySelector('[data-card-hover-preview][data-visible="true"] .interactive-card-frame-stage')?.dataset.renderReady === 'true');
      assert.equal(await stage.getAttribute('data-frame-reused'), 'true');
      assert.equal(await stage.evaluate(node => getComputedStyle(node).opacity), '1');
      assert.equal(await stage.evaluate(node => getComputedStyle(node).transitionDuration), '0s');
      assert.equal(await page.locator('.card-frame-art-preview').count(), 0);
      assert.equal(await page.evaluate(() => window.rawImageSeen), false);
      assert.equal(await page.evaluate(() => window.firstFrame === window.readCachedFrame()), true);
      assert.equal(await page.evaluate(() => window.detailRequests), detailRequests);
      await page.evaluate(() => window.reopenObserver.disconnect());
    }
    // A new image must run its own preparation, not reuse the old printing.
    await page.getByRole('button', { name: 'Change field image' }).evaluate(button => button.click());
    await page.waitForFunction(() => window.readCachedFrame() && window.readCachedFrame() !== window.firstFrame);
    await page.waitForFunction(() => document.querySelector('.interactive-card-frame-stage')?.dataset.renderReady === 'true');
    assert.equal(await stage.getAttribute('data-frame-reused'), 'false');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
