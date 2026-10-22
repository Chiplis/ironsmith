import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Three battlefield animates fixed-size faces, keeps hit targets attached and survives interrupted updates', { timeout: 90000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch({ args: ['--disable-webgl'] });
  try {
    for (const compactCards of [true, false]) {
      const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
      const errors = [];
      page.on('pageerror', error => errors.push(String(error)));
      await page.route('https://**/*', route => route.fulfill({ status: 404, body: '' }));
      await page.addInitScript(value => localStorage.setItem('ironsmith.battlefieldAppearance', JSON.stringify({ compactCards: value, compactLands: true })), compactCards);
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/battlefield-stable-slots.html`, { waitUntil: 'domcontentloaded' });
      await page.waitForFunction(() => document.querySelector('.battlefield-row')?.dataset.sceneMoving === 'false');
      const measure = () => page.locator('.battlefield-row-card[data-object-id="1"]').evaluate(node => {
        const rect = node.getBoundingClientRect();
        return { x: rect.x, y: rect.y, width: rect.width, height: rect.height, layoutWidth: node.offsetWidth, layoutHeight: node.offsetHeight };
      });
      const before = await measure();
      await page.getByRole('button', { name: 'Remove object', exact: true }).click();
      await page.waitForTimeout(100);
      const afterRemoval = await measure();
      assert.ok(Math.abs(before.x - afterRemoval.x) < .1 && Math.abs(before.y - afterRemoval.y) < .1, 'ordinary departure does not move survivors');
      await page.evaluate(() => {
        window.sceneCardResizes = 0;
        const card = document.querySelector('.battlefield-row-card[data-object-id="1"]');
        let first = true;
        window.sceneResizeObserver = new ResizeObserver(() => { if (!first) window.sceneCardResizes++; first = false; });
        window.sceneResizeObserver.observe(card);
        window.sceneSamples = [];
        function sample() {
          window.sceneSamples.push({ moving: document.querySelector('.battlefield-row').dataset.sceneMoving,
            width: card.getBoundingClientRect().width });
          if (window.sceneSamples.length < 300) requestAnimationFrame(sample);
        }
        requestAnimationFrame(sample);
      });
      await page.waitForTimeout(40);
      await page.getByRole('button', { name: 'Add many objects', exact: true }).click();
      await page.waitForTimeout(70);
      await page.waitForFunction(() => document.querySelector('.battlefield-row').dataset.sceneMoving === 'false');
      const crowded = await measure();
      await page.screenshot({ path: '/tmp/battlefield-scene-crowded.png' });
      assert.ok(await page.evaluate(() => window.sceneSamples.some(sample => sample.moving === 'true')), JSON.stringify({ before, crowded, samples: await page.evaluate(() => window.sceneSamples.slice(-12)) }));
      assert.ok(crowded.width < before.width, 'dense board scales down');
      assert.equal(crowded.layoutWidth, before.layoutWidth);
      assert.equal(crowded.layoutHeight, before.layoutHeight);
      assert.equal(await page.evaluate(() => window.sceneCardResizes), 0, 'scaling does not resize the DOM card');
      const hit = await page.locator('.battlefield-row-card[data-object-id="1"]').evaluate(node => {
        const r = node.getBoundingClientRect();
        return document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)?.closest('[data-object-id]')?.dataset.objectId;
      });
      assert.equal(hit, '1', 'pointer target follows the rendered object');
      await page.getByRole('button', { name: 'Remove object', exact: true }).click();
      await page.waitForTimeout(100);
      assert.ok(Math.abs((await measure()).width - crowded.width) < 1, 'departures do not immediately expand the board');
      await page.getByRole('button', { name: 'Add many objects', exact: true }).click();
      await page.waitForTimeout(450);
      assert.ok(Math.abs((await measure()).width - crowded.width) < 1, 'new arrivals cancel delayed growth');
      await page.getByRole('button', { name: 'Remove object', exact: true }).click();
      await page.waitForTimeout(1100);
      assert.ok((await measure()).width > crowded.width, 'stable sparse board grows after settling');
      await page.screenshot({ path: `/tmp/battlefield-scene-${compactCards ? 'compact' : 'full'}.png` });
      assert.deepEqual(errors, []);
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});
