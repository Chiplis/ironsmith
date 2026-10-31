import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Build defers the custom library and compiles only rules changes', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false, watch: null }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors = [];
    let poolRequests = 0;
    page.on('pageerror', error => { errors.push(error.message); console.error(error.message); });
    await page.route('https://**', route => route.abort());
    // Compiler/search responsiveness must not depend on the optional renderer.
    await page.route('**/card-glint/*/magic.css', route => route.abort());
    await page.route('**/custommagic/pool.json', route => {
      poolRequests++;
      return route.fulfill({ json: { cards: [{ id: 'custom-test', draft: { faces: [{ name: 'Custom Performance Test' }] } }] } });
    });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/build-workspace.html`, { waitUntil: 'domcontentloaded' });
    await page.getByRole('button', { name: 'Build', exact: true }).click();
    const dialog = page.getByRole('dialog');
    await page.waitForFunction(() => window.buildPreviewCalls?.length === 1);
    assert.equal(poolRequests, 0, 'opening Build must not download/parse the unused custom pool');

    assert.equal(await dialog.getByRole('combobox', { name: 'Finish', exact: true }).count(), 0);
    assert.equal(await dialog.getByRole('combobox', { name: 'Print effects', exact: true }).count(), 0);
    await dialog.getByRole('checkbox', { name: 'Preview finish effects', exact: true }).check();
    await dialog.getByRole('combobox', { name: 'Finish', exact: true }).selectOption({ label: 'None' });
    await dialog.getByRole('combobox', { name: 'Print effects', exact: true }).selectOption('off');
    await dialog.getByRole('combobox', { name: 'Border', exact: true }).selectOption('black');
    await dialog.getByRole('combobox', { name: 'Rarity', exact: true }).selectOption('rare');
    await dialog.getByRole('checkbox', { name: 'Preview finish effects', exact: true }).uncheck();
    assert.equal(await dialog.getByRole('combobox', { name: 'Finish', exact: true }).count(), 0);
    const compilation = dialog.getByRole('region', { name: 'Compilation', exact: true });
    assert.equal(await compilation.getByRole('tab', { name: 'Compiled Text', exact: true }).getAttribute('aria-selected'), 'true');
    await compilation.getByRole('tab', { name: 'Compiled Abilities', exact: true }).click();
    await compilation.getByRole('button', { name: 'Copy card definition', exact: true }).waitFor();
    await compilation.getByRole('tab', { name: 'Compiled Text', exact: true }).click();
    assert.equal(await dialog.locator('.card-forge-side').getByRole('textbox', { name: 'Rules text' }).count(), 1);
    assert.equal(await dialog.locator('.card-forge-printing-controls > :first-child').getByRole('textbox', { name: 'Artist', exact: true }).count(), 1);
    await page.waitForTimeout(400);
    assert.equal(await page.evaluate(() => window.buildPreviewCalls.length), 1, 'appearance changes must not recompile rules');

    await dialog.getByRole('textbox', { name: 'Card name', exact: true }).fill('Updated rules');
    await page.waitForFunction(() => window.buildPreviewCalls?.length === 2);
    assert.equal(await page.evaluate(() => window.buildPreviewCalls.at(-1).faces[0].name), 'Updated rules');

    await dialog.getByRole('checkbox', { name: 'Include custom cards', exact: true }).check();
    await dialog.getByPlaceholder('Find a card to add or edit').fill('Custom Performance');
    await dialog.getByRole('button', { name: /Custom Performance Test/ }).waitFor();
    assert.equal(poolRequests, 1, 'opting into custom search loads the library');
    await dialog.getByRole('checkbox', { name: 'Include custom cards', exact: true }).uncheck();
    await dialog.getByRole('button', { name: /Custom Performance Test/ }).waitFor({ state: 'hidden' });
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});

test('typing stays responsive with real preview assets and no automatic GPU startup', { timeout: 90000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false, watch: null }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('https://**', route => route.abort());
    // Keep the actual GPU renderer, fonts and local frame assets enabled.
    // Native key delivery stalls when a GPU task monopolizes the compositor,
    // even when PerformanceObserver reports no long main-thread JS tasks.
    await page.addInitScript(() => {
      window.buildInputTimes = [];
      window.buildWebGlContexts = 0;
      const getContext = HTMLCanvasElement.prototype.getContext;
      HTMLCanvasElement.prototype.getContext = function(type, options) {
        if (type === 'webgl2') window.buildWebGlContexts++;
        return getContext.call(this, type, options);
      };
      document.addEventListener('input', event => {
        if (event.target.matches('.build-card-search input')) window.buildInputTimes.push(performance.now());
      });
    });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/build-workspace.html`, { waitUntil: 'domcontentloaded' });
    await page.getByRole('button', { name: 'Build', exact: true }).click();
    const search = page.getByPlaceholder('Find a card to add or edit');
    await search.pressSequentially('Lightning Bolt', { delay: 200 });
    await page.getByRole('button', { name: 'Lightning Bolt', exact: true }).waitFor();
    await page.waitForFunction(() => document.querySelector('[data-card-glint-live]')?.dataset.cardGlintLive === 'static');
    const times = await page.evaluate(() => window.buildInputTimes);
    const longestGap = Math.max(...times.slice(1).map((time, i) => time - times[i]));
    assert.equal(times.length, 'Lightning Bolt'.length);
    assert.ok(longestGap < 1500, `typing stalled for ${Math.round(longestGap)}ms during preview startup`);
    assert.equal(await page.evaluate(() => window.buildWebGlContexts), 0, 'opening and searching Build never starts a GPU effect renderer');
    assert.equal(await page.getByRole('checkbox', { name: 'Preview finish effects', exact: true }).getAttribute('aria-checked'), 'false');
    // Exercise the explicit live-preview fallback independently of the host GPU.
    await page.evaluate(() => {
      window.buildCaveatRequests = 0;
      const getContext = HTMLCanvasElement.prototype.getContext;
      HTMLCanvasElement.prototype.getContext = function(type, options) {
        if (type === 'webgl2' && options?.failIfMajorPerformanceCaveat) {
          window.buildCaveatRequests++;
          return null;
        }
        return getContext.call(this, type, options);
      };
    });
    await page.getByRole('checkbox', { name: 'Preview finish effects', exact: true }).check();
    await page.waitForFunction(() => window.buildCaveatRequests > 0
      && document.querySelector('[data-card-glint-live]')?.dataset.cardGlintLive === 'static');
    await page.getByRole('button', { name: 'Close', exact: true }).click();
    await page.getByRole('button', { name: 'Build', exact: true }).click();
    assert.equal(await page.getByRole('checkbox', { name: 'Preview finish effects', exact: true }).getAttribute('aria-checked'), 'false', 'each new Build session starts without GPU effects');

    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
