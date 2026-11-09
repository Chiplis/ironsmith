import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('shared phase track, full control, public choices and double-click hand play', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.route('https://**/*', route => route.fulfill({ status: 404, body: '' }));
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(() => {
      window.handCasts = [];
      window.addEventListener('ironsmith:hand-card-keyboard-cast', event => window.handCasts.push(event.detail));
    });
    const url = `http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html`;
    await page.goto(`${url}?singleOpponent&decisionDock&hand=7&playableHand&chosenValues&otherPriority`);
    const stop = (player, phase) => page.locator(`.phase-track--sequence:not(.phase-track--combat) [data-stop-player="${player}"][data-stop-phase="${phase}"]`);
    await stop(0, 'Main').waitFor();
    assert.equal(await page.locator('.phase-track--sequence:not(.phase-track--combat) .phase-track-step-label').count(), 8, 'one shared turn sequence');
    for (const player of [0, 1]) {
      for (const phase of ['Main', 'Main2']) assert.equal(await stop(player, phase).getAttribute('data-stop'), 'always', 'main phases stop every time by default');
    }
    assert.equal(await page.locator('.phase-track-legend-player[data-phase-player="0"]').getAttribute('data-active-turn'), 'true');
    assert.equal(await page.locator('.phase-track-legend-player[data-phase-player="0"]').getAttribute('data-has-priority'), 'true', 'priority follows the decision player');
    await stop(0, 'Main').click();
    assert.equal(await stop(0, 'Main').getAttribute('data-stop'), 'off');
    assert.equal(await stop(1, 'Main').getAttribute('data-stop'), 'always');
    for (const mode of ['can_act', 'always', 'off']) {
      await stop(1, 'Upkeep').click();
      assert.equal(await stop(1, 'Upkeep').getAttribute('data-stop'), mode);
    }
    const control = page.getByRole('button', { name: 'Full control', exact: true }).last();
    await control.click();
    assert.equal(await control.getAttribute('aria-pressed'), 'true');
    await control.click();
    assert.equal(await control.getAttribute('aria-pressed'), 'false');
    assert.ok(await page.locator('.battlefield-chosen-values').filter({ hasText: 'Choice: odd' }).count());
    assert.ok(await page.locator('.battlefield-chosen-values').filter({ hasText: 'Type: Human' }).count());
    await page.locator('.battlefield-chosen-values').filter({ hasText: 'Choice: odd' }).first().locator('..').hover({ force: true });
    await page.locator('.floating-card-preview .inspector-chosen-values').filter({ hasText: 'Choice: odd' }).waitFor();
    await page.mouse.move(700, 10);
    const card = page.locator('.hand-layout-item .game-card').first();
    await card.dblclick({ force: true });
    await page.waitForFunction(() => window.handCasts.length === 1);
    assert.equal(await page.evaluate(() => window.handCasts[0].objectId), 8000);
    assert.equal(await page.evaluate(() => window.handCasts[0].actions[0].kind), 'cast_spell');
    const brandAlignment = () => page.locator('.table-phase-divider').evaluate(divider => {
      const icon = divider.querySelector('.phase-track--sequence:not(.phase-track--combat) .phase-track-stop-icon');
      const label = icon.closest('.phase-track-cell').querySelector('.phase-track-step-label');
      const center = node => { const rect = node.getBoundingClientRect(); return rect.top + rect.height / 2; };
      return { icon: center(icon), logo: center(divider.querySelector('.topbar-brand-menu-trigger')),
        label: center(label), links: center(divider.querySelector('.topbar-brand-links')) };
    });
    for (const width of [1440, 1024]) {
      await page.setViewportSize({ width, height: 900 });
      for (const count of [2, 4]) {
        for (const perspective of [0, 1]) {
          await page.goto(`${url}?playerCount=${count}&perspective=${perspective}&utilities`);
          await page.locator('.phase-track-group--players .phase-track').first().waitFor();
          const order = await page.locator('.phase-track--sequence:not(.phase-track--combat) [data-phase-name="Upkeep"] [data-stop-player]')
            .evaluateAll(nodes => nodes.map(node => Number(node.dataset.stopPlayer)));
          assert.deepEqual(order, Array.from({ length: count }, (_, index) => (perspective + index) % count), 'your dot comes first, then turn order');
          const alignment = await brandAlignment();
          assert.ok(Math.abs(alignment.icon - alignment.logo) < 1, `${width}px / ${count} players: logo aligns with phase icons: ${JSON.stringify(alignment)}`);
          assert.ok(Math.abs(alignment.label - alignment.links) < 1, 'site icons align with phase names');
        }
      }
    }
    await page.screenshot({ path: '/Users/chiplis/ironsmith/reports/priority-system/05-player-turns.png' });
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
