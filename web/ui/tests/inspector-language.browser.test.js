import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('language switches preserve ability dispatch and payment gating in both inspector layouts', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1000, height: 850 } });
    await page.route('**/card-i18n/es/**', async route => {
      await new Promise(resolve => setTimeout(resolve, 150));
      await route.fulfill({ json: { 'payment-fixture': {
        name: 'Carta de prueba', typeLine: 'Artefacto',
        oracleText: '{T}: Agrega {G}.\nPaga 1 vida: Roba una carta.',
      } } });
    });
    await page.route('https://api.scryfall.com/**', route => route.fulfill({ status: 404, body: '' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/inspector-payment.html?translated=1`);
    await page.getByRole('button', { name: 'Toggle inspector', exact: true }).click();
    const abilities = page.locator('button[data-available]');
    for (let layout = 0; layout < 2; layout++) {
      await page.getByRole('button', { name: 'Spanish', exact: true }).click();
      const draw = abilities.filter({ hasText: 'Roba una carta.' });
      const mana = abilities.filter({ hasText: 'Agrega' });
      await draw.waitFor();
      assert.equal(await mana.isDisabled(), true, 'unavailable mana action stays disabled');
      if (layout === 0) {
        await page.waitForFunction(() => window.paymentRequests.length === 1);
        await page.evaluate(() => window.resolveNextPayment());
        assert.equal(await draw.isDisabled(), true, 'translation preserves payment failure');
        await draw.evaluate(button => button.click());
        assert.deepEqual(await page.evaluate(() => window.activatedAbilities), []);
        await page.getByRole('button', { name: 'Toggle payment', exact: true }).click();
        await page.waitForFunction(() => window.paymentRequests.length === 2);
        assert.equal(await draw.isDisabled(), true, 'pending payment stays disabled');
        await page.evaluate(() => window.resolveNextPayment());
      }
      await page.waitForFunction(() => [...document.querySelectorAll('button[data-available]')].some(button => button.textContent.includes('Roba') && !button.disabled));
      await draw.click();
      await page.getByRole('button', { name: 'English', exact: true }).click();
      const english = abilities.filter({ hasText: 'Draw a card.' });
      await english.waitFor();
      await english.click();
      assert.deepEqual(await page.evaluate(() => window.activatedAbilities), Array((layout + 1) * 2).fill(1));
      if (layout === 0) await page.getByRole('button', { name: 'Toggle layout', exact: true }).click();
    }
  } finally { await browser.close(); await vite.close(); }
});
