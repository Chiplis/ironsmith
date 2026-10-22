import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('local field reserves the Stack lane and smoothly restores its space', { timeout: 90000 }, async () => {
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
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/diagnostics-layout.html?kind=priority&scenario=phase-controls&players=2&objects=8&stackLaneTest=1`, { waitUntil: "domcontentloaded" });
      const field = page.locator('[data-my-zone] .battlefield-row');
      const settle = async () => {
        await page.waitForTimeout(150);
        await page.waitForFunction(() => document.querySelector('[data-my-zone] .battlefield-row')?.dataset.sceneMoving === 'false');
      };
      const measure = () => field.evaluate(row => {
        const rect = row.getBoundingClientRect();
        return { scale: Number(row.dataset.sceneScale), left: rect.left,
          cards: [...row.querySelectorAll('.battlefield-row-card')].map(card => {
            const r = card.getBoundingClientRect(); return { left: r.left, right: r.right };
          }) };
      });
      await settle();
      const before = await measure();
      await page.evaluate(() => window.setStackLaneVisible(true));
      await page.waitForFunction(() => Number(document.querySelector('[data-card-stack-anchor]')?.dataset.stackLaneWidth) > 0);
      await page.waitForFunction(() => document.querySelector('[data-my-zone] .battlefield-row')?.dataset.sceneMoving === 'true');
      await settle();
      const active = await measure();
      const edge = await page.locator('[data-card-stack-anchor]').evaluate(dock => dock.getBoundingClientRect().left + Number(dock.dataset.stackLaneWidth));
      assert.ok(active.cards.every(card => card.left >= edge), JSON.stringify({ compactCards, edge, active }));
      assert.ok(active.cards[0].left > before.cards[0].left, 'cards reflow away from the Stack');
      // At the minimum card size, scrolling still cannot expose cards in the lane.
      assert.notEqual(await field.evaluate(row => getComputedStyle(row).clipPath), 'none');
      // Switching renderers must preserve every displayed pose, including
      // intermediate animation frames, in both directions.
      for (const mode of ['trigger', 'replacement', null, 'replacement', null]) {
        const samples = await page.evaluate(async mode => {
          const field = document.querySelector('[data-my-zone] .battlefield-row');
          const samples = [];
          window.setStackOrder(mode);
          const start = performance.now();
          while (performance.now() - start < 650) {
            await new Promise(requestAnimationFrame);
            samples.push([...field.querySelectorAll('.battlefield-row-card')].map(card => {
              const r = card.getBoundingClientRect(); return { left: r.left, width: r.width };
            }));
          }
          return samples;
        }, mode);
        for (const cards of samples) for (let i = 0; i < cards.length; i++) {
          assert.ok(Math.abs(cards[i].left - active.cards[i].left) < 1, `no position jump switching to ${mode}`);
          assert.ok(Math.abs(cards[i].width - (active.cards[i].right - active.cards[i].left)) < 1, `no resize switching to ${mode}`);
        }
      }
      const reservedWidth = await field.evaluate(row => row.closest('.my-zone-board-shell').style.getPropertyValue('--stack-reserved-width'));
      // A large count forces the old pile-footprint calculation to grow;
      // sample throughout arrival and exit, not only at rest.
      for (const count of [20, 19, 3, 1]) {
        const samples = await page.evaluate(async count => {
          const row = document.querySelector('[data-my-zone] .battlefield-row');
          window.setStackCount(count);
          const samples = [], start = performance.now();
          while (performance.now() - start < 900) {
            await new Promise(requestAnimationFrame);
            samples.push({ width: row.closest('.my-zone-board-shell').style.getPropertyValue('--stack-reserved-width'),
              left: row.querySelector('.battlefield-row-card').getBoundingClientRect().left,
              scale: Number(row.dataset.sceneScale) });
          }
          return samples;
        }, count);
        for (const sample of samples) {
          assert.equal(sample.width, reservedWidth, `entry count ${count} preserves the lane`);
          assert.ok(Math.abs(sample.left - active.cards[0].left) < 1);
          assert.equal(sample.scale, active.scale);
        }
      }
      // Refill during the final exit: the same reservation must survive.
      await page.evaluate(() => window.setStackCount(0));
      await page.waitForTimeout(100);
      assert.equal(await field.evaluate(row => row.closest('.my-zone-board-shell').style.getPropertyValue('--stack-reserved-width')), reservedWidth);
      await page.evaluate(() => window.setStackCount(1));
      await page.waitForTimeout(900);
      assert.equal(await field.evaluate(row => row.closest('.my-zone-board-shell').style.getPropertyValue('--stack-reserved-width')), reservedWidth);
      await page.evaluate(() => window.setStackCount(0));
      assert.ok(await page.locator('[data-card-stack-anchor]').evaluate(dock => Number(dock.dataset.stackLaneWidth)) > 0, 'lane remains during the exit');
      await page.waitForFunction(() => Number(document.querySelector('[data-card-stack-anchor]')?.dataset.stackLaneWidth) === 0);
      await settle();
      const after = await measure();
      assert.ok(Math.abs(after.scale - before.scale) < .001);
      assert.ok(Math.abs(after.cards[0].left - before.cards[0].left) < 1, 'field restores its original space');
      assert.equal(await field.evaluate(row => getComputedStyle(row).clipPath), 'none');
      assert.deepEqual(errors, []);
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});
