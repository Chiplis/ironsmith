import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('grouped mana outputs dispatch independently in both layouts and languages', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 850 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('https://api.scryfall.com/**', route => route.fulfill({ status: 404, body: '' }));
    await page.route('**/card-i18n/es/**', route => route.fulfill({ json: { 'payment-fixture': {
      name: 'Carta de prueba', typeLine: 'Tierra', oracleText: '{T}: Agrega {U}.\n{T}: Agrega {G}.',
    } } }));
    for (const partial of [false, true]) {
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/inspector-payment.html?dual=1${partial ? '&partial=1' : ''}`);
      await page.getByRole('button', { name: 'Toggle inspector', exact: true }).click();
      await page.waitForFunction(count => window.paymentRequests.length === count, partial ? 1 : 2);
      await page.evaluate(() => { window.resolveNextPayment(); window.resolveNextPayment(); });
      const expected = [];
      for (let layout = 0; layout < 2; layout++) {
        for (const language of ['English', 'Spanish']) {
          await page.getByRole('button', { name: language, exact: true }).click();
          const line = page.locator('.inspector-mana-line');
          await line.filter({ hasText: language === 'English' ? 'Add' : 'Agrega' }).waitFor();
          assert.equal(await page.locator('.inspector-ability-section').count(), 1);
          assert.equal(await line.locator('button').count(), 2);
          assert.match(await line.textContent(), language === 'English' ? / or / : / o /);
          const blue = line.getByRole('button', { name: /\{U\}/ });
          const green = line.getByRole('button', { name: /\{G\}/ });
          await blue.click();
          expected.push(1);
          if (partial) {
            assert.equal(await green.isDisabled(), true);
            await green.evaluate(button => button.click());
          } else {
            await green.focus();
            await page.keyboard.press('Enter');
            expected.push(0);
          }
          assert.deepEqual(await page.evaluate(() => window.activatedAbilities), expected);
        }
        if (layout === 0) {
          await page.screenshot({ path: '/tmp/grouped-mana-abilities.png' });
          await page.getByRole('button', { name: 'Toggle layout', exact: true }).click();
        }
      }
    }
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
