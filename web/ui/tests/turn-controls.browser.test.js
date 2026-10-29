import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('turn controls follow selected phases and hide Auto-pass during payment', { timeout: 60000 }, async () => {
  const server = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.route('https://**/*', route => route.fulfill({status:404, body:''}));
    const base = `http://127.0.0.1:${server.httpServer.address().port}`;
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`${base}/tests/diagnostics-layout.html?kind=priority&scenario=phase-controls`);
    await page.getByRole('button', {name:'Go to Main II', exact:true}).waitFor();
    await page.getByRole('button', {name:/^Combat: Stop when I can act/}).first().click();
    await page.getByRole('button', {name:'Go to Combat', exact:true}).click();
    await page.getByRole('button', {name:'Skipping Main I', exact:true}).waitFor();
    assert.equal(await page.evaluate(() => window.__phaseAdvance), true);
    await page.getByRole('button', {name:'End turn', exact:true}).waitFor();
    assert.equal(await page.getByRole('button', {name:'Pass', exact:true}).count(), 0);
    assert.equal(await page.getByRole('button', {name:'Auto-pass', exact:true}).count(), 1);
    await page.goto(`${base}/tests/diagnostics-layout.html?kind=mana_payment`);
    await page.locator('[data-mana-payment="true"]').waitFor();
    assert.equal(await page.getByRole('button', {name:'Auto-pass', exact:true}).count(), 0);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await server.close(); }
});
