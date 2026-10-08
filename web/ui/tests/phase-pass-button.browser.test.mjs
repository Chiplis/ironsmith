import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { fileURLToPath } from 'node:url';

test('blue Pass button sits beside the main decision and toggles passing', async () => {
  const server = await createServer({ root: fileURLToPath(new URL('..', import.meta.url)),
    server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await server.listen();
  const browser = await chromium.launch();
  try {
    for (const width of [1024, 1365]) {
      const page = await browser.newPage({ viewport: { width, height: 768 }, reducedMotion: 'reduce' });
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.route('https://**/*', route => route.abort());
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/diagnostics-layout.html?kind=priority`, { waitUntil: 'domcontentloaded' });
      const pass = page.getByRole('button', { name: 'Pass', exact: true });
      await pass.waitFor();
      await page.waitForTimeout(650);
      const main = page.locator('.battlefield-human-decision-panel .decision-main-button:visible').first();
      const mainBox = await main.boundingBox();
      const box = await pass.boundingBox();
      assert.ok(box.x >= mainBox.x + mainBox.width - 2 && box.x - mainBox.x - mainBox.width <= 12);
      assert.ok(Math.abs(box.y + box.height / 2 - mainBox.y - mainBox.height / 2) < 8);
      assert.equal(await pass.evaluate(el => getComputedStyle(el).backgroundColor), 'rgb(21, 89, 166)');
      await pass.click();
      assert.equal(await pass.getAttribute('aria-pressed'), 'true');
      await pass.click();
      assert.equal(await pass.getAttribute('aria-pressed'), 'false');
      for (const query of ['kind=priority&scenario=opponent-turn', 'kind=mana_payment']) {
        await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/diagnostics-layout.html?${query}`, { waitUntil: 'domcontentloaded' });
        await page.locator('.battlefield-human-quick-controls').waitFor();
        assert.equal(await pass.count(), 0, `Pass must be hidden for ${query}`);
      }
      if (width === 1365) await page.screenshot({ path: '/tmp/ironsmith-phase-pass.png' });
      assert.deepEqual(errors, []);
      await page.close();
    }
  } finally { await browser.close(); await server.close(); }
});
