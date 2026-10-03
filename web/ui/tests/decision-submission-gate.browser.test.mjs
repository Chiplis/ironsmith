import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Resolve and Resolve all disable for the submission gate and re-enable while verification continues', async () => {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const vite = await createServer({ root, configFile: path.join(root, 'vite.config.js'),
    optimizeDeps: { entries: ['tests/decision-submission-gate.html'] }, logLevel: 'silent',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/decision-submission-gate.html`);
    const resolve = page.getByRole('button', { name: 'Resolve', exact: true });
    const all = page.getByRole('button', { name: 'Resolve all', exact: true });
    try { await resolve.waitFor({ timeout: 10000 }); }
    catch (error) { console.error(JSON.stringify({ errors, body: await page.locator('body').innerText() })); throw error; }
    assert.equal(await resolve.isEnabled(), true);
    assert.equal(await all.isEnabled(), true);
    await page.evaluate(() => window.__setSubmissionBusy(true));
    await page.waitForFunction(() => document.querySelector('.action-strip-resolve-all-button')?.disabled);
    assert.equal(await resolve.isEnabled(), false);
    assert.equal(await all.isEnabled(), false);
    await resolve.evaluate(button => button.click());
    await all.evaluate(button => button.click());
    assert.deepEqual(await page.evaluate(() => window.__decisionCalls), []);
    await page.evaluate(() => window.__setSubmissionBusy(false));
    await page.waitForFunction(() => !document.querySelector('.action-strip-resolve-all-button')?.disabled);
    assert.equal(await resolve.isEnabled(), true);
    await resolve.click();
    await all.click();
    assert.deepEqual(await page.evaluate(() => window.__decisionCalls.map(value => typeof value === 'string' ? value : value.action_ref.kind)),
      ['pass_priority', 'resolve all']);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
