import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Mulligan uses 0 and action shortcut badges retain the small End turn size', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1600, height: 900 } });
    await page.route('https://**/*', route => route.fulfill({ status: 404, body: '' }));
    const url = `http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock`;
    await page.goto(`${url}&playableHand`);
    const endHint = page.locator('.decision-phase-pass .priority-shortcut-hint');
    await endHint.waitFor();
    const endSize = await endHint.evaluate(node => getComputedStyle(node).fontSize);
    assert.equal(endSize, '10px');
    await page.goto(`${url}&mulligan`);
    const mulligan = page.getByRole('button', { name: 'Mulligan', exact: true });
    await mulligan.waitFor();
    const hint = page.locator('.action-strip-command-region').filter({ has: mulligan }).locator('kbd');
    assert.equal(await hint.textContent(), '0');
    for (const badge of await page.locator('.action-strip-main-title-row kbd').all()) {
      assert.equal(await badge.evaluate(node => getComputedStyle(node).fontSize), endSize);
    }
    await page.evaluate(async () => {
      const { installPriorityShortcuts } = await import('/src/lib/priority-shortcuts.js');
      installPriorityShortcuts(document, {
        state: () => ({ decision: { kind: 'priority', actions: [{ kind: 'take_mulligan' }] } }),
        maintain: () => {},
      });
      const { setHandOpen } = await import('/src/lib/hand-open.js');
      setHandOpen(true);
    });
    await hint.waitFor();
    await page.keyboard.press('0');
    await page.waitForFunction(() => document.querySelector('output').textContent.includes('take_mulligan'));
    await page.evaluate(() => {
      document.querySelector('output').textContent = '';
      document.querySelector('.action-strip-mulligan-button').disabled = true;
    });
    await page.keyboard.press('0');
    assert.equal(await page.locator('output').textContent(), '');
    await page.evaluate(async () => {
      document.querySelector('.action-strip-mulligan-button').disabled = false;
      window.mulliganClicks = 0;
      document.querySelector('.action-strip-mulligan-button').addEventListener('click', () => window.mulliganClicks++);
      const { setShortcutPreset } = await import('/src/lib/keyboard-preferences.js');
      setShortcutPreset('classic');
    });
    await page.waitForFunction(() => [...document.querySelectorAll('kbd')].some(node => node.textContent === 'Alt+0'));
    await page.keyboard.press('0');
    assert.equal(await page.evaluate(() => window.mulliganClicks), 0);
    await page.keyboard.press('Alt+0');
    assert.equal(await page.evaluate(() => window.mulliganClicks), 1);
  } finally {
    await browser.close();
    await vite.close();
  }
});
