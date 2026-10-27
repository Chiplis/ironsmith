import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('clicking keywords shows reminders, mana symbols, and the CR fallback', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false, watch: null }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/keyword-reminders.html`);
    const tooltip = page.locator('[data-ui-layer="tooltip"][data-state="open"]');
    await page.getByRole('button', { name: 'Flying rules helper' }).click();
    await tooltip.waitFor();
    assert.match(await tooltip.innerText(), /can't be blocked/);
    assert.ok(!(await tooltip.innerText()).includes('Reminder text:'));
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Cycling rules helper' }).click();
    await tooltip.waitFor();
    assert.match(await tooltip.innerText(), /cycling/i);
    assert.match(await tooltip.innerText(), /Draw a card/);
    assert.ok(!(await tooltip.innerText()).includes('Example:'));
    assert.equal(await tooltip.locator('img').count(), 4);
    assert.equal(await tooltip.locator('.mtg-keyword-helper').count(), 0);
    assert.ok(!(await tooltip.innerText()).includes('{2}'), 'mana costs render as symbols');
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Counter rules helper' }).click();
    await tooltip.waitFor();
    assert.ok(!(await tooltip.innerText()).includes('Reminder text:'));
    assert.match(await tooltip.innerText(), /701.6/);
    for (const [name, expected] of [
      ['Mobilize', /create 2 tapped and attacking 1\/1 red Warrior creature tokens/],
      ['Ward', /Pay 3 life/],
      ['Suspend', /4 time counters/],
      ['Morph', /2\/2 creature/],
      ['Fabricate', /3 1\/1 colorless Servo artifact creature tokens/],
    ]) {
      await page.keyboard.press('Escape');
      await page.getByRole('button', {name: `${name} rules helper`,exact:true}).click();
      await tooltip.waitFor();
      assert.match(await tooltip.innerText(),expected);
      assert.ok(!(await tooltip.innerText()).includes('Example:'));
    }
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
