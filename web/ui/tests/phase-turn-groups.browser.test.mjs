import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir } from 'node:fs/promises';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('one phase track with a stop dot per player, from either seat', { timeout: 60000 }, async () => {
  const server = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false }, logLevel: 'error' });
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.route('https://**', route => route.abort());
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const phases = ['Untap', 'Upkeep', 'Draw', 'Main', 'Combat', 'Main2', 'End', 'Cleanup'];
    const base = `http://127.0.0.1:${server.httpServer.address().port}/tests/zone-piles-table.html`;
    for (const seat of [0, 1]) {
      await page.goto(`${base}?singleOpponent&utilities&perspective=${seat}`);
      await page.locator('.phase-track-group--players').waitFor();
      const mainTrack = page.locator('.phase-track--sequence:not(.phase-track--combat)');
      assert.equal(await mainTrack.locator('.phase-track-stop-icon').count(), phases.length, 'one icon per phase');
      const order = await mainTrack.locator('[data-stop-player]').evaluateAll(nodes => nodes.map(node => [node.dataset.stopPlayer, node.dataset.stopPhase]));
      assert.deepEqual(order, phases.flatMap(phase => [seat, 1 - seat].map(player => [String(player), phase])), 'dots follow turn order from the local seat');
      // The dots sit between the icon and the label.
      const layout = await mainTrack.locator('[data-phase-name="Upkeep"]').evaluate(cell => {
        const y = selector => cell.querySelector(selector).getBoundingClientRect().top;
        return [y('.phase-track-stop-icon'), y('.phase-track-player-stops'), y('.phase-track-step-label')];
      });
      assert.ok(layout[0] < layout[1] && layout[1] < layout[2], `icon, dots, label: ${layout}`);
      const dot = player => mainTrack.locator(`[data-stop-phase="Upkeep"][data-stop-player="${player}"]`);
      const accent = await dot(seat).evaluate(node => getComputedStyle(node, '::before').borderTopColor);
      for (const [mode, fill] of [['can_act', 'rgb(255, 255, 255)'], ['always', accent], ['off', 'rgba(0, 0, 0, 0)']]) {
        await dot(seat).click();
        assert.equal(await dot(seat).getAttribute('data-stop'), mode);
        assert.equal(await dot(seat).evaluate(node => getComputedStyle(node, '::before').backgroundColor), fill);
        assert.equal(await dot(seat).evaluate(node => getComputedStyle(node, '::before').borderTopColor), accent, 'border keeps the player color');
        assert.equal(await dot(1 - seat).getAttribute('data-stop'), 'off');
      }
      assert.notEqual(await dot(1 - seat).evaluate(node => getComputedStyle(node, '::before').borderTopColor), accent, 'each player has their own color');
      assert.equal(await page.locator('[data-combat-skip]').count(), 2);
      const attackSkip = page.locator('[data-combat-skip="attackers"]');
      assert.equal(await attackSkip.getAttribute('aria-pressed'), 'true');
      await attackSkip.click();
      assert.equal(await attackSkip.getAttribute('aria-pressed'), 'false');
      await attackSkip.click();
      const combat = mainTrack.locator('[data-phase-name="Combat"] .phase-track-stop-button');
      await combat.hover();
      assert.equal(await combat.getAttribute('aria-expanded'), 'true');
      const damage = page.locator(`.phase-track--combat [data-stop-phase="CombatDamage"][data-stop-player="${seat}"]`);
      await damage.waitFor({ state: 'visible' });
      const before = await damage.getAttribute('data-stop');
      await damage.click();
      assert.notEqual(await damage.getAttribute('data-stop'), before);
      await page.mouse.move(0, 0);
      await damage.waitFor({ state: 'hidden' });
      assert.equal(await page.locator('.phase-track-legend-player').count(), 2);
      assert.equal(await page.locator(`.phase-track-legend-player[data-phase-player="0"]`).getAttribute('data-active-turn'), 'true');
      if (seat === 0) {
        await mkdir('test-results/phase-turn-groups', { recursive: true });
        await page.locator('.phase-track-group--players').screenshot({ path: 'test-results/phase-turn-groups/stops.png' });
      }
    }
    for (const count of [3, 4]) {
      await page.goto(`${base}?playerCount=${count}&utilities`);
      await page.locator('.phase-track-group--players').waitFor();
      assert.equal(await page.locator('.phase-track--sequence:not(.phase-track--combat) .phase-track-stop-icon').count(), phases.length);
      assert.equal(await page.locator('.phase-track--sequence:not(.phase-track--combat) [data-phase-name="Draw"] [data-stop-player]').count(), count);
      const box = await page.locator('.phase-track-group--players').boundingBox();
      assert.ok(box.x + box.width <= 1440);
      await page.locator('.phase-track-group--players').screenshot({ path: `test-results/phase-turn-groups/wide-${count}.png` });
    }
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await server.close(); }
});
