import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Build searches supported and custom cards and retains the card editor', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/build-card-search.html`);
    await page.getByRole('button', { name: 'Build', exact: true }).click();
    await page.getByRole('combobox', { name: 'Card Name' }).fill('Light');
    await page.getByRole('option', { name: 'Lightning Bolt' }).click();
    await page.getByRole('combobox', { name: /^Player/ }).selectOption('1');
    await page.getByRole('combobox', { name: /^Zone/ }).selectOption('graveyard');
    await page.getByRole('checkbox', { name: 'Skip triggers' }).check();
    await page.getByRole('button', { name: 'Add to Game' }).click();
    await page.getByRole('dialog').waitFor({ state: 'hidden' });
    assert.deepEqual(await page.evaluate(() => window.additions[0]), { kind: 'supported', args: [1, 'Lightning Bolt', 'graveyard', true] });

    await page.getByRole('button', { name: 'Build', exact: true }).click();
    await page.getByRole('combobox', { name: 'Card Name' }).fill('Cleveland');
    await page.getByRole('checkbox', { name: /Include custom cards/ }).check();
    await page.getByRole('option', { name: /Cleveland/ }).click();
    await page.getByRole('button', { name: 'Add to Game' }).click();
    await page.getByRole('dialog').waitFor({ state: 'hidden' });
    assert.equal(await page.evaluate(() => window.additions[1].kind), 'custom');
    assert.equal(await page.evaluate(() => window.additions[1].args.draft.faces[0].name), 'Cleveland');

    await page.getByRole('button', { name: 'Build', exact: true }).click();
    await page.getByRole('tab', { name: 'Create card', exact: true }).click();
    await page.getByText('Card Layout', { exact: true }).waitFor();
    await page.getByRole('tab', { name: 'Search cards', exact: true }).click();
    await page.getByRole('combobox', { name: 'Card Name' }).waitFor();
    assert.equal(await page.getByRole('dialog').count(), 1);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.getByRole('combobox', { name: 'Card Name' }).fill('Forest');
    await page.getByRole('option', { name: 'Forest', exact: true }).waitFor();
    const dialog = await page.getByRole('dialog').boundingBox();
    assert.ok(dialog.x >= -1 && dialog.x + dialog.width <= 391);
    await page.screenshot({ path: '/tmp/ironsmith-build-search.png' });
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
