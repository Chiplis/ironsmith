import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('scene viewports fit independently in 2/3/4-player layouts and both appearances', { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch({ args: ['--disable-webgl'] });
  try {
    for (const compactCards of [true, false]) {
      const page = await browser.newPage({ viewport: { width: 1710, height: 954 } });
      const errors = [];
      page.on('pageerror', error => errors.push(String(error)));
      await page.route('https://**/*', route => route.fulfill({ status: 404, body: '' }));
      await page.addInitScript(value => localStorage.setItem('ironsmith.battlefieldAppearance', JSON.stringify({ compactCards: value, compactLands: true })), compactCards);
      const measure = async () => {
        await page.waitForTimeout(150);
        await page.waitForFunction(() => [...document.querySelectorAll('.battlefield-row')].every(n => n.dataset.sceneMoving === 'false'));
        return page.locator('.battlefield-row').evaluateAll(rows => rows.map(row => {
          const viewport = row.getBoundingClientRect();
          const cards = [...row.querySelectorAll('.battlefield-row-card')].map(card => card.getBoundingClientRect());
          return { cardWidth: Number.parseFloat(row.style.getPropertyValue('--bf-card-width')),
            fits: cards.every(r => r.top >= viewport.top - 1 && r.bottom <= viewport.bottom + 1 && r.left >= viewport.left - 1 && r.right <= viewport.right + 1) };
        }));
      };
      for (const players of [2, 3, 4]) {
        const url = `http://127.0.0.1:${vite.httpServer.address().port}/tests/diagnostics-layout.html?kind=priority&scenario=phase-controls&players=${players}&objects=1`;
        await page.setViewportSize({ width: 1710, height: 954 });
        await page.goto(url, { waitUntil: 'domcontentloaded' });
        const small = await measure();
        assert.equal(small.length, players);
        assert.ok(small.every(field => field.fits), JSON.stringify({ players, compactCards, small }));
        await page.setViewportSize({ width: 2560, height: 1440 });
        const large = await measure();
        assert.ok(large.every((field, i) => field.fits && field.cardWidth > small[i].cardWidth * 1.15), JSON.stringify({ players, compactCards, small, large }));
        await page.goto(`${url}&opponentObjects=30`, { waitUntil: 'domcontentloaded' });
        const crowded = await measure();
        assert.ok(crowded[0].cardWidth < large[0].cardWidth);
        assert.ok(Math.abs(crowded.at(-1).cardWidth - large.at(-1).cardWidth) < 1, 'opponent arrivals cannot resize the local field');
      }
      assert.deepEqual(errors, []);
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});
