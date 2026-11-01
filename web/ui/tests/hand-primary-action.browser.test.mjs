import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('table priority button becomes the selected hand action with a Space hint', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('https://**', route => route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?singleOpponent&decisionDock&hand=7&playableHand`);
    await page.locator('.hand-card').first().waitFor();
    await page.evaluate(async () => {
      const { installHandToggleShortcut } = await import('/src/lib/hand-open.js');
      const { installMainDecisionShortcut } = await import('/src/lib/main-decision-shortcut.js');
      installHandToggleShortcut(document); installMainDecisionShortcut(document);
      window.handRequests = [];
      window.addEventListener('ironsmith:hand-card-keyboard-cast', event => window.handRequests.push(event.detail));
    });
    await page.keyboard.press('h');
    await page.keyboard.press('ArrowRight');
    const action = page.getByRole('button', { name: 'Play', exact: true });
    await action.waitFor();
    assert.equal(await action.locator('kbd').textContent(), 'Space');
    assert.equal(await page.locator('.action-strip-main-title-row:visible').count(), 0, 'pass label and shortcut are replaced');
    await mkdir('test-results/hand-primary-action', { recursive: true });
    await page.screenshot({ path: 'test-results/hand-primary-action/table.png' });
    await action.screenshot({ path: 'test-results/hand-primary-action/button.png' });
    await page.keyboard.press('Space');
    await page.waitForFunction(() => window.handRequests.length === 1);
    assert.equal(await page.evaluate(() => window.handRequests[0].actions[0].kind), 'cast_spell');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
