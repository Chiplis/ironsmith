import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('desktop and mobile decision labels render game symbols', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  let browser;
  try {
    await vite.listen();
    browser = await chromium.launch();
    const page = await browser.newPage();
    await page.route('https://**/*', route => route.fulfill({ status: 404, body: '' }));
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    for (const mobile of [false, true]) {
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/decision-symbols.html${mobile ? '?mobile' : ''}`);
      const label = page.locator(mobile ? '.mobile-decision-secondary-button' : '.action-strip-main-title-text').filter({ hasText: 'Pay' }).first();
      await label.waitFor();
      assert.equal(await label.locator('img').count(), 5);
      assert.doesNotMatch(await label.textContent(), /\{[^}]+\}/);
      assert.equal(await page.locator('#counters label img').count(), 1);
      assert.doesNotMatch(await page.locator('#counters label').textContent(), /\{E\}/);
      await page.waitForFunction(() => [...document.querySelectorAll('#menu img, #counters img')].every(img => img.complete && img.naturalWidth > 0));
    }
    assert.deepEqual(errors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});
