import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('hover reuses the displayed object image and follows per-object changes', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const requests = [], errors = [];
    page.on('request', request => requests.push(request.url()));
    page.on('pageerror', error => errors.push(error.message));
    await page.route('https://cards.scryfall.io/**', async route => {
      if (route.request().url().includes('/art_crop/')) await new Promise(resolve => setTimeout(resolve, 1500));
      await route.fulfill({ contentType: 'image/svg+xml', headers: { 'access-control-allow-origin': '*' }, body: '<svg xmlns="http://www.w3.org/2000/svg" width="488" height="684"><rect width="488" height="684" fill="#917659"/></svg>' });
    });
    await page.route('https://api.scryfall.com/**', route => route.fulfill({ status: 404, body: '' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-preview-source.html`);
    const first = page.getByAltText('Field card 1');
    await first.hover();
    const preview = page.locator('[data-card-hover-preview][data-visible="true"] .card-frame-art-preview');
    await preview.waitFor();
    assert.equal(await preview.getAttribute('src'), await first.getAttribute('src'));
    assert.equal(await page.locator('.interactive-card-frame-stage').getAttribute('data-render-ready'), 'false', 'field image appears before art-crop reconstruction finishes');
    await page.getByRole('button', { name: 'Change field image' }).click();
    await page.waitForFunction(() => document.querySelector('.card-frame-art-preview')?.getAttribute('src') === document.querySelector('[alt="Field card 1"]').getAttribute('src'));
    await page.getByAltText('Field card 2').hover();
    await page.waitForFunction(() => document.querySelector('[data-card-hover-preview][data-visible="true"]')?.dataset.previewObjectId === '2');
    assert.equal(await page.locator('[data-card-hover-preview][data-visible="true"] .original-card-fallback > img').getAttribute('src'), await page.getByAltText('Field card 2').getAttribute('src'));
    assert.equal(requests.some(url => url.includes('/cards/named') || url.includes('/cards/same-name-different-field-images')), false, 'displayed assets must not trigger a name-based image lookup');
    assert.equal(requests.filter(url => url.includes('cards.scryfall.io')).every(url => url.includes('/back/a/b/aaaaaaaa-bbbb-cccc-dddd-000000000077.jpg?printing=selected')), true);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
