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
    assert.equal(await mascot.getAttribute('data'), '/ironsmith/theme/ironsmith-loading.svg#working');
    const response = await mascotResponse;
    assert.equal(response.status(), 200);
    assert.match(response.headers()['content-type'], /image\/svg\+xml/);
    await page.waitForFunction(() => {
      const img = document.querySelector('.game-loading-wordmark');
      return !!img?.contentDocument?.querySelector('.hammer-arm');
    });
    assert.equal(await mascot.getAttribute('aria-label'), 'IronSmith');
    const schemes = await mascot.evaluate(el => ({
      host: getComputedStyle(el).colorScheme,
      svg: el.contentDocument.defaultView.getComputedStyle(el.contentDocument.documentElement).colorScheme,
    }));
    assert.equal(schemes.svg, schemes.host, 'embedded SVG must match the dark host to preserve transparency');
    await page.waitForFunction(() => {
      const hammer = document.querySelector('.game-loading-wordmark')?.contentDocument?.getAnimations()
        .find(animation => animation.animationName === 'strike');
      return hammer?.currentTime > 5760;
    }, null, { timeout: 30000 });
    const forging = await mascot.evaluate(el => {
      const animations = el.contentDocument.getAnimations();
      const hammer = animations.find(a => a.animationName === 'strike');
      const mask = animations.find(a => a.animationName === 'mask-depth-turn');
      return { iterations: String(hammer.effect.getTiming().iterations), time: hammer.currentTime, maskTime: mask.currentTime };
    });
    assert.equal(forging.iterations, 'Infinity');
    assert.ok(forging.time > 5760, 'continues past three strikes');
    assert.equal(forging.maskTime, 960, 'mask stays down while downloading');
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

test('smith cadence follows progress and finishes after six cycles; reduced motion completes immediately', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const origin = `http://127.0.0.1:${vite.httpServer.address().port}`;
    await page.goto(`${origin}/tests/loading-smith.html`);
    await page.waitForFunction(() => document.querySelector('object')?.contentDocument?.getAnimations().length > 0);
    const hammerTime = () => page.evaluate(() => document.querySelector('object').contentDocument.getAnimations().find(a => a.animationName === 'strike').currentTime);
    await page.waitForTimeout(1000);
    const initial = await hammerTime();
    await page.waitForTimeout(1000);
    const stalled = await hammerTime();
    for (let i = 1; i <= 8; i++) {
      await page.evaluate(progress => window.setSmithInput({ progress, ready: false }), i / 10);
      await page.waitForTimeout(125);
    }
    const downloading = await hammerTime();
    assert.ok(downloading - stalled > (stalled - initial) * 1.3, 'faster progress increases hammer cadence');
    assert.equal(await page.evaluate(() => !!window.smithComplete), false);
    await page.evaluate(() => window.setSmithInput({ progress: 1, ready: true }));
    await page.waitForTimeout(100);
    assert.equal(await page.evaluate(() => !!window.smithComplete), false, 'does not finish before six cycles');
    await page.waitForFunction(() => window.smithComplete, null, { timeout: 10000 });
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.reload();
    await page.waitForFunction(() => !!window.setSmithInput);
    await page.evaluate(() => window.setSmithInput({ progress: 1, ready: true }));
    await page.waitForFunction(() => window.smithComplete);
    assert.equal(await page.locator('object').evaluate(el => el.contentDocument.getAnimations().length), 0);
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    await page.route('**/ironsmith-loading.svg', route => route.abort());
    await page.reload();
    await page.waitForFunction(() => !!window.setSmithInput);
    await page.evaluate(() => window.setSmithInput({ progress: 1, ready: true }));
    await page.waitForFunction(() => window.smithComplete, null, { timeout: 5000 });
  } finally {
    await browser.close();
    await vite.close();
  }
});
