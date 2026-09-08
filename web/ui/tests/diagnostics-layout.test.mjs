import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { fileURLToPath } from 'node:url';

// Exercise the real board and CSS, including inert ancestors and hit testing.
test('expanded decisions leave diagnostics visible and usable', { timeout: 90000 }, async () => {
  const root = fileURLToPath(new URL('../', import.meta.url));
  const server = await createServer({ root, server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  let browser;
  try {
    await server.listen();
    browser = await chromium.launch();
    const base = `http://127.0.0.1:${server.httpServer.address().port}`;
    for (const width of [1440, 1024]) {
      for (const kind of ['targets', 'select_options', 'mana_payment', 'attackers']) {
        const page = await browser.newPage({ viewport: { width, height: 900 } });
        const errors = [];
        page.on('pageerror', error => errors.push(error.message));
        await page.goto(`${base}/tests/diagnostics-layout.html?kind=${kind}`);
        const diagnostics = page.getByRole('button', { name: 'Diagnostics', exact: true });
        await diagnostics.waitFor();
        const decision = await page.locator('.table-decision-overlay-slot').boundingBox();
        const utilities = await page.locator('.table-persistent-utility-strip').boundingBox();
        assert.ok(decision.y + decision.height <= utilities.y + 1, `${width}/${kind}: overlap`);
        const actions = page.locator('.table-inline-utility-actions .table-zone-action-button');
        const diagnosticBounds = await diagnostics.boundingBox();
        for (const action of await actions.all()) {
          const bounds = await action.boundingBox();
          assert.ok(Math.abs(bounds.y - diagnosticBounds.y) <= 1, `${width}/${kind}: buttons must share a row`);
          assert.ok(bounds.y >= decision.y + decision.height - 1, `${width}/${kind}: action overlaps decision`);
          await action.scrollIntoViewIfNeeded();
          await action.click({ trial: true });
        }
        assert.equal(await actions.count(), 7);
        await page.getByRole('button', { name: 'Hide table tools', exact: true }).click();
        assert.ok(await page.locator('#table-utility-actions').isHidden());
        await diagnostics.click();
        assert.ok(await page.getByRole('dialog').isVisible());
        await page.keyboard.press('Escape');
        assert.deepEqual(errors, []);
        await page.close();
      }
    }
  } finally {
    await browser?.close();
    await server.close();
  }
});
