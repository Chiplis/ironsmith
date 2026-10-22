import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('compact plays show only their automatic destination; moves retain compact outlines', { timeout: 90000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    for (const compactCards of [true, false]) {
      const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
      const errors = [];
      page.on('pageerror', e => errors.push(String(e)));
      await page.route('https://**/*', route => route.fulfill({ status: 404, body: '' }));
      await page.addInitScript(compactCards => localStorage.setItem('ironsmith.battlefieldAppearance', JSON.stringify({ compactCards, compactLands: true })), compactCards);
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/battlefield-stable-slots.html`);
      await page.waitForFunction(() => window.placementFixture);
      const slots = page.locator('[data-battlefield-drop-slot]');
      for (const [type, row] of [['Creature', 1], ['Land', 2], ['Planeswalker', 1], ['Battle', 1]]) {
        await page.evaluate(type => window.placementFixture.start({ id: 999, name: 'Preview', type_line: type }, type === 'Land' ? 'play_land' : 'cast_spell', true), type);
        await page.waitForTimeout(100);
        if (compactCards) {
          assert.equal(await slots.count(), 1);
          assert.equal(await slots.first().getAttribute('data-row'), String(row));
          const before = await slots.first().boundingBox();
          await page.evaluate(() => { const r = document.querySelector('.battlefield-row').getBoundingClientRect(); window.placementFixture.move(r.right - 20, r.bottom - 20); });
          await page.waitForTimeout(100);
          assert.deepEqual(await slots.first().boundingBox(), before, 'pointer does not select another destination');
          if (type === 'Land') assert.ok(before.width / before.height > 1, 'land uses its compact tile');
        } else assert.ok(await slots.count() > 1, 'full layout retains selectable grid');
        await page.evaluate(() => window.placementFixture.end());
      }
      await page.evaluate(() => window.placementFixture.start({ id: 1, stable_id: 1, type_line: 'Artifact Creature', lane: 'creatures' }, 'move_battlefield'));
      await page.waitForTimeout(100);
      assert.ok(await slots.count() > 1, 'moving retains the grid');
      if (compactCards) {
        const box = await slots.first().boundingBox();
        assert.ok(box.width > box.height, 'move outline has the compact aspect ratio');
      }
      assert.deepEqual(errors, []);
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});
