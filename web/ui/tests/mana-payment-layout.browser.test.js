import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('desktop payment covers the middle strip, keeps controls visible and opens source choices', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1600,height:1000},reducedMotion:'reduce'});
    const errors = [];
    page.on('pageerror', e => errors.push(e.message));
    const base = `http://127.0.0.1:${vite.httpServer.address().port}/tests/diagnostics-layout.html`;
    for (const players of [2,4]) {
      await page.goto(`${base}?kind=priority&players=${players}`);
      await page.locator('.navigation-utility-controls').waitFor();
      await page.evaluate(() => document.fonts.ready);
      await page.waitForTimeout(250);
      const brand = await page.locator('.navigation-utility-controls').boundingBox();
      const untap = await page.locator('[data-phase-name="Untap"] .phase-track-step-marker').boundingBox();
      assert.ok(brand.x > 1400);
      assert.ok(Math.abs(untap.x + untap.width / 2 - 44.8) < 2);
      const cleanup = await page.locator('[data-phase-name="Cleanup"]').boundingBox();
      const label = await page.locator('[data-phase-name="Cleanup"] .phase-track-step-label').boundingBox();
      assert.ok(brand.x - label.x - label.width >= 75);
      assert.ok(cleanup.x > untap.x);
      const wordmark = page.locator('.topbar-brand-menu-trigger');
      assert.equal((await wordmark.textContent()).trim(), 'IronSmith');
      assert.ok(await wordmark.evaluate(el => getComputedStyle(el).fontFamily.includes('Matrix')));
    }
    for (const scenario of ['progenitus','long-payment','short-payment','kozilek-command']) {
      await page.goto(`${base}?kind=mana_payment&scenario=${scenario}`);
      const editor = page.locator('.desktop-mana-payment .mana-payment-editor');
      await editor.waitFor();
      if (scenario === 'kozilek-command') {
        assert.equal(await editor.locator('.mana-payment-pip-row').count(), 4);
        assert.equal(await editor.getByText("Ugin's Labyrinth", {exact:true}).count(), 2);
        assert.ok(await editor.locator('.mana-payment-source-name').evaluateAll(nodes => nodes.every(node => node.scrollWidth <= node.clientWidth)));
      }
      for (const width of [1600,1100,800]) {
        await page.setViewportSize({width,height:1000});
        await page.waitForTimeout(250);
        const bounds = await editor.boundingBox();
        const strip = await page.locator('.table-phase-divider').boundingBox();
        const pay = await page.locator('.desktop-mana-payment-actions .mana-payment-pay-button').boundingBox();
        const brand = await page.locator('.navigation-utility-controls').boundingBox();
        const cancel = await page.locator('.desktop-mana-payment-actions .decision-cancel-button').boundingBox();
        assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width);
        assert.ok(Math.abs(bounds.y - strip.y) < 1);
        assert.ok(bounds.height <= strip.height + 6);
        assert.ok(Math.abs(bounds.x + bounds.width - (width - 8)) < 1);
        assert.ok(bounds.x <= brand.x && bounds.y <= brand.y && bounds.y + bounds.height >= brand.y + brand.height);
        assert.ok(pay.y > strip.y + strip.height && pay.y > 900);
        assert.ok(pay.x >= 0 && cancel.x + cancel.width <= width);
        assert.ok(Math.abs(pay.y + pay.height / 2 - cancel.y - cancel.height / 2) < 1, JSON.stringify({scenario,width,pay,cancel}));
        const scroll = editor.locator('.mana-payment-editor-scroll');
        await scroll.evaluate(el => { el.scrollLeft = el.scrollWidth; });
        const last = await editor.locator('.mana-payment-pip-row').last().boundingBox();
        const visible = await scroll.boundingBox();
        assert.ok(last.x >= visible.x && last.x + last.width <= visible.x + visible.width + 1);
        await scroll.evaluate(el => { el.scrollLeft = 0; });
      }
      await page.setViewportSize({width:1600,height:1000});
      await editor.locator('.mana-payment-pip-row .mana-payment-source-picker').first().click();
      await page.getByRole('dialog', {name:'Choose payment source'}).waitFor();
      await page.keyboard.press('Escape');
      await page.locator('.desktop-mana-payment-actions .mana-payment-pay-button').click();
      await page.waitForTimeout(100);
      assert.ok(await page.evaluate(() => window.__dispatched));
    }
    await page.goto(`${base}?kind=mana_payment&scenario=progenitus&paymentReady=false`);
    const pay = page.locator('.desktop-mana-payment-actions .mana-payment-pay-button');
    await pay.waitFor();
    assert.equal(await pay.isDisabled(), true);
    const cancel = page.locator('.desktop-mana-payment-actions .decision-cancel-button');
    assert.equal(await cancel.isEnabled(), true);
    await cancel.click();
    assert.equal(await page.evaluate(() => window.__cancelled), true);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
