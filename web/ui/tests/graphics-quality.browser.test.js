import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Off skips WebGL, Low stops while idle, and modes release their resources', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch({ args: ['--enable-unsafe-swiftshader'] });
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 });
    await page.addInitScript(() => {
      localStorage.setItem('ironsmith.graphicsQuality', 'off');
      window.graphicsContexts = 0; window.graphicsDraws = 0;
      const get = HTMLCanvasElement.prototype.getContext;
      HTMLCanvasElement.prototype.getContext = function(type, ...args) {
        if (type.startsWith('webgl')) window.graphicsContexts++;
        return get.call(this, type, ...args);
      };
      const draw = WebGL2RenderingContext.prototype.drawElements;
      WebGL2RenderingContext.prototype.drawElements = function(...args) {
        window.graphicsDraws++; return draw.apply(this, args);
      };
    });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/forge-board.html`);
    await page.waitForSelector('[data-graphics-quality="off"]');
    assert.equal(await page.evaluate(() => window.graphicsContexts), 0);
    await page.locator('.battlefield-row button').first().click();
    assert.equal(await page.locator('output').textContent(), '1');
    const change = quality => page.evaluate(async q => {
      const { setGraphicsQuality } = await import('/src/hooks/useGraphicsQuality.js');
      setGraphicsQuality(q);
    }, quality);
    await change('low');
    await page.waitForSelector('.forge-board[data-models="6"][data-terrain="ready"] canvas');
    await page.waitForTimeout(1000);
    const idle = await page.evaluate(() => window.graphicsDraws);
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(() => window.graphicsDraws), idle);
    assert.ok(await page.locator('canvas').evaluate(c => c.width * c.height <= 1_000_000));
    await change('high');
    await page.waitForSelector('.forge-board[data-motion="animated"] canvas');
    assert.equal(await page.locator('canvas').count(), 1);
    assert.ok(await page.locator('canvas').evaluate(c => c.width * c.height <= 4_000_000));
    await page.waitForSelector('.forge-board[data-models="6"][data-terrain="ready"]');
    await page.waitForTimeout(2000);
    const highIdle = await page.evaluate(() => window.graphicsDraws);
    await page.waitForTimeout(250);
    assert.equal(await page.evaluate(() => window.graphicsDraws), highIdle, 'High has no perpetual ambient rendering');
    await change('off');
    await page.waitForFunction(() => !document.querySelector('.forge-board canvas'));
    assert.equal(await page.locator('.forge-field-lights').count(), 0);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
