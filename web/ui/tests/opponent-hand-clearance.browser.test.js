import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('opponent fans clear their battlefield above it, and recover when space is available', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    for (const width of [1024, 1365, 2048]) {
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      await page.route('https://**/*', route => route.abort());
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/opponent-hand-clearance.html`);
      await page.locator('.opponent-hand-card').first().waitFor();
      const measure = () => page.evaluate(() => {
        const field = document.querySelector('#opponent-field').getBoundingClientRect();
        const cards = [...document.querySelectorAll('.opponent-hand-card')].map(card => card.getBoundingClientRect())
          .filter(rect => rect.left < field.right && rect.right > field.left);
        return { gap: field.top - Math.max(...cards.map(rect => rect.bottom)),
          translate: document.querySelector('.opponent-hand-fan').style.translate };
      });
      await page.waitForTimeout(700);
      const first = await measure();
      assert.ok(first.gap >= 11, `DOM cards cleared at ${width}: ${JSON.stringify(first)}`);
      assert.match(first.translate, /^0px -/);
      await page.waitForTimeout(600);
      assert.deepEqual(await measure(), first, 'translation settles without feedback');
      await page.evaluate(() => {
        const field = document.querySelector('#opponent-field');
        field.style.top = '20px';
        field.battlefieldScene = { bounds: () => [{ left: 0, right: field.clientWidth, top: 0, bottom: 100 }] };
        window.dispatchEvent(new Event('ironsmith:battlefield-layout-fitted'));
      });
      await page.waitForTimeout(700);
      assert.ok((await measure()).gap >= 11, 'canvas footprints also clear');
      await page.evaluate(() => { document.querySelector('#opponent-field').style.display = 'none'; });
      await page.waitForTimeout(700);
      assert.equal((await measure()).translate, '', 'fan returns to normal despite another opponent’s field');
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});
