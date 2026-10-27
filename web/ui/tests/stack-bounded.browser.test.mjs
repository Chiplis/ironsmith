import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { mkdir } from 'node:fs/promises';

test('stack stays below the phase tracker at every depth and can browse all entries', { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, reducedMotion: 'reduce' });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/diagnostics-layout.html?kind=priority&scenario=phase-controls&players=2&objects=8&stackLaneTest=1`);
    await page.waitForFunction(() => typeof window.setStackCount === 'function');
    await page.evaluate(() => window.setStackLaneVisible(true));
    await page.locator('[data-card-stack-pile]').waitFor();
    await mkdir('test-results', { recursive: true });
    for (const size of [{ width: 1440, height: 900 }, { width: 1100, height: 700 }]) {
      await page.setViewportSize(size);
      for (const count of [1, 6, 20]) {
        await page.evaluate(count => window.setStackCount(count), count);
        await page.waitForTimeout(1000);
        const check = async () => {
          const bounds = await page.evaluate(() => ({
            phaseBottom: Math.max(...[...document.querySelectorAll('.phase-track-group')].map(node => node.getBoundingClientRect().bottom)),
            dockTop: document.querySelector('[data-card-stack-anchor]').getBoundingClientRect().top,
            cardTop: Math.min(...[...document.querySelectorAll('.card-stack-slot:not([data-leaving])')].map(node => node.getBoundingClientRect().top)),
          }));
          assert.ok(bounds.cardTop >= bounds.dockTop, JSON.stringify(bounds));
          assert.ok(bounds.cardTop >= bounds.phaseBottom, JSON.stringify(bounds));
        };
        await check();
        await page.locator('.card-stack-cards').hover();
        await page.waitForTimeout(600);
        await check();
        if (size.width === 1440) await page.screenshot({ path: `test-results/stack-bounded-${count}.png` });
        if (count === 20) {
          while (await page.getByRole('button', { name: 'Previous stack entry', exact: true }).isEnabled()) {
            await page.getByRole('button', { name: 'Previous stack entry', exact: true }).click();
          }
          for (let index = 1; index < count; index++) await page.getByRole('button', { name: 'Next stack entry', exact: true }).click();
          assert.equal(await page.locator('.card-stack-pager-index').textContent(), '20');
          await check();
        }
      }
    }
  } finally { await browser.close(); await vite.close(); }
});
