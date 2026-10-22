import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('scene sleeps when idle, honors reduced motion and restores the host on disposal', async () => {
  const vite = await createServer({ configFile: false, optimizeDeps: { noDiscovery: true },
    server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(String(error)));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/battlefield-scene-controller.html`, { waitUntil: 'domcontentloaded' });
    await page.waitForFunction(() => document.getElementById('field').dataset.sceneMoving === 'false');
    const viewport = await page.locator('#field').evaluate(field => ({ width: field.clientWidth, height: field.clientHeight }));
    assert.deepEqual(await page.locator('#field').evaluate(field => ({ width: field.scrollWidth, height: field.scrollHeight })), viewport,
      'a fitting scene has no phantom scroll range');
    assert.equal(await page.locator('#field').evaluate(field => getComputedStyle(field).scrollbarWidth), 'none');
    const idle = await page.evaluate(() => window.sceneHarness.fitted);
    await page.waitForTimeout(120);
    assert.equal(await page.evaluate(() => window.sceneHarness.fitted), idle);
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.evaluate(() => window.sceneHarness.update(80));
    await page.waitForFunction(() => document.getElementById('field').dataset.sceneMoving === 'false');
    assert.equal(await page.locator('#field button').count(), 80);
    assert.ok(await page.evaluate(() => document.getElementById('field').scrollHeight > 300));
    assert.deepEqual(await page.locator('#field').evaluate(field => ({ width: field.clientWidth, height: field.clientHeight })), viewport,
      'overflow cannot change the viewport used for fitting');
    await page.evaluate(() => { document.getElementById('field').scrollTop = 300; window.sceneHarness.update(1); });
    await page.waitForFunction(() => document.getElementById('field').dataset.sceneMoving === 'false');
    assert.equal(await page.locator('#field button').count(), 1);
    await page.waitForTimeout(1100);
    const settled = await page.locator('#field').evaluate(field => ({ scale: field.dataset.sceneScale, frames: window.sceneHarness.fitted }));
    await page.waitForTimeout(600);
    assert.deepEqual(await page.locator('#field').evaluate(field => ({ scale: field.dataset.sceneScale, frames: window.sceneHarness.fitted })), settled,
      'removing overflow settles without a resize/growth pulse');
    const disposed = await page.evaluate(() => {
      window.sceneHarness.scene.dispose();
      const field = document.getElementById('field');
      return { children: field.children.length, display: field.style.display, padding: field.style.padding, fitted: window.sceneHarness.fitted };
    });
    assert.equal(disposed.children, 0);
    assert.equal(disposed.display, 'grid');
    assert.equal(disposed.padding, '7px');
    await page.waitForTimeout(500);
    assert.equal(await page.evaluate(() => window.sceneHarness.fitted), disposed.fitted, 'no frames or growth timers survive disposal');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
