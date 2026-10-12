import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('resolved battlefield cards show live rules before Scryfall or local art requests finish', { timeout: 30000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  let browser;
  try {
    browser = await chromium.launch();
    const page = await browser.newPage();
    await page.addInitScript(() => localStorage.setItem('ironsmith.battlefieldAppearance', JSON.stringify({ compactCards: false, compactLands: false })));
    const blocked = [];
    await page.route(/api\.scryfall\.com|cards\.scryfall\.io|\/cards\/network-independence-creature/, route => {
      blocked.push(route.request().url());
      // Deliberately leave art and metadata requests pending throughout assertions.
    });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-network-independence.html`);
    await page.getByRole('button', { name: 'Resolve into battlefield' }).click();
    await page.waitForFunction(() => {
      const frame = document.querySelector('.battlefield-prepared-frame .interactive-card-frame-stage');
      return frame?.getAttribute('aria-hidden') === 'false' && frame.textContent.includes('Vigilance');
    }, null, { timeout: 5000 });
    assert.ok(blocked.length > 0, 'The fixture must exercise an outstanding art request');
    assert.equal(await page.locator('.battlefield-prepared-frame .interactive-card-frame-stage').getAttribute('data-default-frame'), 'colorless');
  } finally { await browser?.close(); await vite.close(); }
});
