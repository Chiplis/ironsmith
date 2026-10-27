import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('clicking an attacker assigns its sole target and clicking again removes it', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/combat-decision.html`, { waitUntil: 'domcontentloaded' });
    await page.getByRole('button', { name: 'Go to Attackers', exact: true }).click();
    const creature = page.locator('.battlefield-row .game-card[data-object-id="10"]').first();
    await creature.click();
    await page.waitForFunction(() => /1/.test(document.querySelector('.combat-submit-button')?.textContent || ''));
    assert.equal(JSON.parse(await page.locator('[data-combat-arrows]').getAttribute('data-combat-arrows'))[0].toPlayerId, 1);
    await page.waitForFunction(() => {
      const button = document.querySelector('.combat-submit-button');
      return button && !button.disabled && button.getAttribute('aria-disabled') !== 'true';
    });
    await page.keyboard.press('Space');
    await page.waitForFunction(() => document.querySelector('output')?.textContent.includes('declare_attackers'));
    const result = JSON.parse(await page.locator('output').textContent());
    assert.deepEqual(result, { type: 'declare_attackers', declarations: [{ creature: 10, target: { kind: 'player', player: 1 } }] });
    // Let the card's double-click guard expire before deselecting it.
    await page.waitForTimeout(400);
    await creature.click();
    await page.waitForFunction(() => JSON.parse(document.querySelector('[data-combat-arrows]').dataset.combatArrows).length === 0);
    await page.waitForFunction(() => {
      const button = document.querySelector('.combat-submit-button');
      return button && !button.disabled && button.getAttribute('aria-disabled') !== 'true';
    });
    await page.keyboard.press('Space');
    await page.waitForFunction(() => document.querySelector('output')?.textContent.includes('"declarations":[]'));
    assert.deepEqual(JSON.parse(await page.locator('output').textContent()).declarations, []);
    for (const targets of [0, 2]) {
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/combat-decision.html?targets=${targets}`, { waitUntil: 'domcontentloaded' });
      await page.getByRole('button', { name: 'Go to Attackers', exact: true }).click();
      await creature.click();
      await page.waitForTimeout(100);
      assert.deepEqual(JSON.parse(await page.locator('[data-combat-arrows]').getAttribute('data-combat-arrows')), []);
      assert.equal(await page.locator('[data-selected-attacker]').getAttribute('data-selected-attacker'), targets === 2 ? '10' : '');
    }
  } finally { await browser.close(); await vite.close(); }
});
