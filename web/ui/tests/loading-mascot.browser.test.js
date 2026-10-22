import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('loading mascot and normal icons resolve under the deployed subdirectory', async () => {
  const vite = await createServer({ base: '/ironsmith/', server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    // Keep startup on the loading screen while checking its real asset request.
    await page.route('https://fonts.googleapis.com/**', route => route.fulfill({contentType:'text/css',body:''}));
 await page.route('**/*.wasm*', () => {});
    const origin = `http://127.0.0.1:${vite.httpServer.address().port}`;
    const mascotResponse = page.waitForResponse(response => response.url() === `${origin}/ironsmith/theme/ironsmith-loading.svg`);
    await page.goto(`${origin}/ironsmith/`, { waitUntil: 'domcontentloaded' });
    const mascot = page.locator('.game-loading-wordmark');
    await mascot.waitFor();
    assert.equal(await mascot.getAttribute('src'), '/ironsmith/theme/ironsmith-loading.svg#working');
    const response = await mascotResponse;
    assert.equal(response.status(), 200);
    assert.match(response.headers()['content-type'], /image\/svg\+xml/);
    await page.waitForFunction(() => {
      const img = document.querySelector('.game-loading-wordmark');
      return img?.complete && img.naturalWidth > 0;
    });
    assert.equal(await mascot.getAttribute('alt'), 'IronSmith');
    assert.equal(await page.locator('.game-loading-brand').count(), 0);
    for (const viewport of [{width:390,height:844}, {width:844,height:390}]) {
      await page.setViewportSize(viewport);
      const box = await mascot.boundingBox();
      assert.ok(box.x >= 0 && box.x+box.width <= viewport.width);
      assert.ok(box.y >= 0 && box.y+box.height <= viewport.height);
    }
    const favicon = await page.locator('link[rel="icon"]').evaluate(el => el.href);
    assert.equal(favicon, `${origin}/ironsmith/theme/ironsmith-logo.svg`);
    const manifestUrl = `${origin}/ironsmith/manifest.webmanifest`;
    const manifest = await (await page.request.get(manifestUrl)).json();
    assert.equal(new URL(manifest.icons[0].src, manifestUrl).href, favicon);
    await page.goto(favicon);
    assert.equal(await page.evaluate(() => document.getAnimations().length), 0);
    assert.equal(await page.locator('.happy').evaluate(el => getComputedStyle(el).display), 'inline');
  } finally {
    await browser.close();
    await vite.close();
  }
});
