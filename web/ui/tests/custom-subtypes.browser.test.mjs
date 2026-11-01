import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
test('subtype builder saves overridden plurals, shows compiler rejection, persists, and removes', { timeout: 90000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false, watch: null }, logLevel: 'error' });
  await vite.listen(); const browser = await chromium.launch();
  try {
    const page = await browser.newPage(); const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/custom-subtypes-preview.html`);
    await page.getByLabel('Singular', { exact: true }).fill('Skychild');
    assert.equal(await page.getByLabel('Plural', { exact: true }).inputValue(), 'Skychilds');
    await page.getByLabel('Plural', { exact: true }).fill('Skychildren');
    await page.getByRole('button', { name: 'Add subtype' }).click();
    await page.getByText('skychild', { exact: true }).waitFor();
    assert.deepEqual(await page.evaluate(() => window.subtypeInputs[0].customSubtypes), [{ singular: 'skychild', plural: 'skychildren', family: 'Creature' }]);
    await page.reload(); await page.getByText('skychild', { exact: true }).waitFor();
    await page.getByLabel('Singular', { exact: true }).fill('Elf');
    await page.getByRole('button', { name: 'Add subtype' }).click();
    await page.getByRole('alert').filter({ hasText: 'reserved' }).waitFor();
    assert.equal(await page.getByRole('listitem').count(), 1);
    await page.getByRole('button', { name: 'Remove', exact: true }).click();
    assert.equal(await page.getByRole('listitem').count(), 0);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
