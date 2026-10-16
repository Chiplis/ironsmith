import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('looking, combat, and effect ordering keep the primary action in the priority slot', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    for (const compactCards of [true, false]) {
      const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
      await page.addInitScript(value => localStorage.setItem('ironsmith.battlefieldAppearance', JSON.stringify({ compactCards: value, compactLands: true })), compactCards);
      await page.route('https://**/*', route => route.abort());
      const button = page.locator('[data-human-action-dock] .decision-main-button');
      const open = async query => {
        await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?decisionDock&${query}`);
        await button.waitFor({ state: 'visible' });
        await page.waitForTimeout(700);
      };
      const sameSlot = (a, b) => {
        for (const key of ['x', 'y', 'width', 'height']) assert.ok(Math.abs(a[key] - b[key]) <= 1, `${key}: ${a[key]} vs ${b[key]}`);
      };
      await open('motion&look');
      const looking = await button.boundingBox();
      await button.click();
      await page.waitForTimeout(500);
      sameSlot(looking, await button.boundingBox());
      await open('combatCycle');
      assert.match(await button.textContent(), /no attackers/i);
      const attacking = await button.boundingBox();
      const hint = await page.locator('.combat-floating-hint').boundingBox();
      assert.ok(hint.y + hint.height < attacking.y);
      for (let step = 1; step <= 3; step++) {
        await page.getByRole('button', { name: 'Next decision', exact: true }).click();
        await page.waitForTimeout(500);
        sameSlot(attacking, await button.boundingBox());
      }
      for (const [kind, label, indices] of [
        ['replacement', 'Apply First', [1]],
        ['trigger', 'Submit Order', [1, 0]],
      ]) {
        await open(`effectOrder=${kind}`);
        assert.equal((await button.textContent()).trim(), label);
        sameSlot(attacking, await button.boundingBox());
        assert.equal(await page.locator('[data-human-action-dock] .action-strip-decision-stack').count(), 0);
        assert.equal(await page.locator('[data-human-action-dock] .battlefield-decision-disclosure-toggle').count(), 0);
        await button.click();
        assert.deepEqual(JSON.parse(await page.locator('output').textContent()), { type: 'select_options', option_indices: indices });
      }
      await page.close();
    }
  } finally {
    await browser.close();
    await vite.close();
  }
});
