import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('player label opens a top-layer menu above the Stack', { timeout: 45000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = [];
    page.on('pageerror', error => errors.push(String(error)));
    await page.route('https://**/*', route => route.fulfill({ status: 404, body: '' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/diagnostics-layout.html?kind=priority&players=2&objects=6&stackLaneTest=1`, { waitUntil: 'domcontentloaded' });
    await page.locator('[data-card-stack-pile]').waitFor();
    const trigger = page.locator('.player-header-perspective-trigger');
    await trigger.locator('.player-header-perspective-label').click();
    const menu = page.getByRole('listbox');
    await menu.waitFor();
    assert.equal(await menu.evaluate(el => el.matches(':popover-open')), true);
    assert.equal(await menu.evaluate(el => {
      const r = el.getBoundingClientRect();
      return el.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2));
    }), true, 'menu receives hits above the overlapping Stack');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    assert.equal(await page.evaluate(() => window.selectedPerspective), 1);
    await trigger.click();
    await page.keyboard.press('Escape');
    assert.equal(await trigger.getAttribute('aria-expanded'), 'false');
    assert.equal(await trigger.evaluate(el => el === document.activeElement), true);
    await trigger.click();
    await page.mouse.click(900, 100);
    assert.equal(await trigger.getAttribute('aria-expanded'), 'false');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
