import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('game table groups full turns by player from either seat', { timeout: 60000 }, async () => {
  const server = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false }, logLevel: 'error' });
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.route('https://**', route => route.abort());
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const phases = ['Untap', 'Upkeep', 'Draw', 'Main', 'Combat', 'Main2', 'End', 'Cleanup'];
    for (const seat of [0, 1]) {
      await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/zone-piles-table.html?singleOpponent&utilities&perspective=${seat}`);
      await page.locator('.phase-track-group--duel').waitFor();
      const order = await page.locator('.phase-track-group--duel nav:not(.phase-track--combat) [data-stop-player]').evaluateAll(nodes => nodes.map(node => [node.dataset.stopPlayer, node.dataset.stopPhase]));
      assert.deepEqual(order, [seat, 1 - seat].flatMap(player => phases.map(phase => [String(player), phase])));
      const stops = page.locator('.phase-track-group--duel [data-stop-phase="Upkeep"]');
      await stops.first().click();
      assert.equal(await stops.first().getAttribute('data-stop'), 'can_act');
      assert.equal(await stops.last().getAttribute('data-stop'), 'off');
      const playerTrack = page.locator(`.phase-track-player[data-phase-player="${seat}"]`);
      const skips = page.locator('[data-combat-skip]');
      assert.equal(await skips.count(), 2);
      const attackSkip = playerTrack.locator('[data-combat-skip="attackers"]');
      assert.equal(await attackSkip.getAttribute('aria-pressed'), 'true');
      assert.match(await attackSkip.getAttribute('title'), /even with no combat stops set/);
      await attackSkip.click();
      assert.equal(await attackSkip.getAttribute('aria-pressed'), 'false');
      assert.equal(await playerTrack.locator('[data-combat-skip="blockers"]').getAttribute('aria-pressed'), 'true');
      await attackSkip.click();
      const combat = playerTrack.locator('[data-stop-phase="Combat"]');
      const damage = playerTrack.locator('[data-stop-phase="CombatDamage"]');
      await playerTrack.locator('.phase-track-player-heading').hover();
      assert.equal(await combat.getAttribute('aria-expanded'), 'true');
      await damage.waitFor({ state: 'visible' });
      const area = await playerTrack.boundingBox();
      await page.mouse.move(area.x + 2, area.y + area.height - 2);
      assert.equal(await damage.isVisible(), true, 'container whitespace keeps combat open');
      await damage.hover();
      assert.equal(await damage.isVisible(), true);
      const before = await damage.getAttribute('data-stop');
      await damage.click();
      assert.notEqual(await damage.getAttribute('data-stop'), before);
      await page.mouse.move(0, 0);
      await damage.waitFor({ state: 'hidden' });
      const boxes = await page.locator('.phase-track-group--duel .phase-track-player').evaluateAll(nodes => nodes.map(node => {
        const box = node.getBoundingClientRect(); return { left: box.left, right: box.right };
      }));
      assert.ok(boxes[0].right <= boxes[1].left);
      assert.ok(boxes[1].right <= 1440);
      assert.equal(await page.locator('.phase-track-player-heading > svg').count(), 2);
      if (seat === 0) {
        await mkdir('test-results/phase-turn-groups', { recursive: true });
        await page.screenshot({ path: 'test-results/phase-turn-groups/table.png' });
        await page.locator('.phase-track-group--duel').screenshot({ path: 'test-results/phase-turn-groups/stops.png' });
      }
    }
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await server.close(); }
});
