import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('coverage toggles default on and reach preview and creation independently', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    await page.route('https://**', route => route.abort());
    await page.route('**/card-glint/*/magic.css', route => route.abort());
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/build-workspace.html`);
    await page.getByRole('button', { name: 'Build', exact: true }).click();
    const filter = page.getByRole('checkbox', { name: 'Enforce filter coverage', exact: true });
    const line = page.getByRole('checkbox', { name: 'Enforce whole-line coverage', exact: true });
    assert.equal(await filter.isChecked(), true);
    assert.equal(await line.isChecked(), true);
    await page.waitForFunction(() => window.buildPreviewCalls?.at(-1)?.enforceFilterClaims === true && window.buildPreviewCalls.at(-1).enforceLineClaims === true);
    await filter.uncheck();
    await page.waitForFunction(() => window.buildPreviewCalls?.at(-1)?.enforceFilterClaims === false && window.buildPreviewCalls.at(-1).enforceLineClaims === true);
    await line.uncheck();
    await filter.check();
    await page.waitForFunction(() => window.buildPreviewCalls?.at(-1)?.enforceFilterClaims === true && window.buildPreviewCalls.at(-1).enforceLineClaims === false);
    await page.getByRole('button', { name: 'Add', exact: true }).click();
    await page.waitForFunction(() => window.lastAdded?.custom);
    const draft = (await page.evaluate(() => window.lastAdded)).custom.draft;
    assert.equal(draft.enforceFilterClaims, true);
    assert.equal(draft.enforceLineClaims, false);
  } finally {
    await browser.close();
    await vite.close();
  }
});
