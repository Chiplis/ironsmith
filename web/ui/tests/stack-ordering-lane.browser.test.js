import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('ordering rails stay bounded while the battlefield reserves their lane', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    for (const compactCards of [true, false]) for (const kind of ['trigger', 'replacement']) {
      const page = await browser.newPage({ viewport: { width: 2048, height: 1149 } });
      const errors = [];
      page.on('pageerror', error => errors.push(String(error)));
      await page.route('https://**/*', route => route.fulfill({ status: 404, body: '' }));
      await page.addInitScript(value => localStorage.setItem('ironsmith.battlefieldAppearance', JSON.stringify({ compactCards: value })), compactCards);
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/diagnostics-layout.html?kind=priority&players=2&objects=6&stackOrder=${kind}`, { waitUntil: "domcontentloaded" });
      const panel = page.locator(`.stack-panel[data-ordering-kind="${kind}"]`);
      await panel.waitFor({ timeout: 10000 }).catch(error => { throw new Error(`${error.message}: ${errors.join("; ")}`); });
      assert.equal(await panel.locator('[data-pending-trigger="true"], [data-pending-replacement="true"]').count(), 2);
      await page.waitForTimeout(1800);
      const bounds = await panel.boundingBox();
      assert.ok(bounds.width > 150 && bounds.width <= 360, JSON.stringify({ compactCards, kind, bounds }));
      await page.waitForTimeout(800);
      assert.ok(Math.abs((await panel.boundingBox()).width - bounds.width) < 1, 'width does not grow as cards reflow');
      const leftmost = await page.locator('[data-my-zone] .battlefield-row-card').evaluateAll(cards => Math.min(...cards.map(card => card.getBoundingClientRect().left)));
      assert.ok(leftmost >= bounds.x + bounds.width, 'battlefield clears the bounded ordering lane');
      assert.deepEqual(errors, []);
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});
