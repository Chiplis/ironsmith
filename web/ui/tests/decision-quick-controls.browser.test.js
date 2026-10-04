import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('priority toggles sit beside the main decision action and remain usable', async () => {
  const server = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await server.listen();
  const browser = await chromium.launch();
  try {
    for (const width of [1024, 1365, 2048]) {
      const page = await browser.newPage({ viewport: { width, height: 768 } });
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.route('https://**/*', route => route.abort());
      for (const kind of ['priority', 'targets', 'attackers']) {
        await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/diagnostics-layout.html?kind=${kind}`);
        const panel = page.locator('.battlefield-human-decision-panel');
        const controls = panel.locator('.decision-quick-controls');
        await controls.waitFor();
        assert.equal(await page.locator('.decision-quick-controls').count(), 1);
        const main = panel.locator('.decision-main-button:visible').first();
        await main.waitFor();
        const mainBox = await main.boundingBox();
        const controlsBox = await controls.boundingBox();
        assert.ok(Math.abs((controlsBox.y + controlsBox.height / 2) - (mainBox.y + mainBox.height / 2)) < 8, `${kind} controls align with main action at ${width}`);
        assert.ok(controlsBox.x + controlsBox.width <= mainBox.x + 2 && mainBox.x - controlsBox.x - controlsBox.width <= 12, `${kind} controls sit immediately beside main action at ${width}`);
        assert.ok(controlsBox.x >= 0 && mainBox.x + mainBox.width <= width);
        const hold = controls.getByRole('button', { name: 'Hold priority', exact: true });
        await hold.click();
        assert.equal(await controls.getByRole('button', { name: 'Holding priority', exact: true }).getAttribute('aria-pressed'), 'true');
        await controls.getByRole('button', { name: 'Holding priority', exact: true }).click();
        const auto = controls.getByRole('button', { name: 'Auto-pass', exact: true });
        await auto.click();
        assert.equal(await auto.getAttribute('aria-pressed'), 'true');
        await auto.click();
        assert.equal(await auto.getAttribute('aria-pressed'), 'false');
      }
      assert.deepEqual(errors, []);
      await page.close();
    }
  } finally { await browser.close(); await server.close(); }
});
