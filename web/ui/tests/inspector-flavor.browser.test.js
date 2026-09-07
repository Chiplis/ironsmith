import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('rules and flavor text shrink together to fit without scrolling', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    for (const long of [false, true]) {
      const page = await browser.newPage({ viewport: { width: 1000, height: 850 } });
      const image = 'https://cards.scryfall.io/art_crop/front/a/b/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jpg';
      const flavor = long ? `${'A tale retold across generations. '.repeat(24)}\n${'Unbroken'.repeat(18)}` : 'A tale retold across generations.';
      const card = { name: 'Payment fixture', image_uris: { art_crop: image, normal: image.replace('/art_crop/', '/normal/') }, flavor_text: flavor };
      await page.route('**/cards/payment-fixture.json', route => route.fulfill({ json: { scryfall: card } }));
      await page.route('https://api.scryfall.com/**', route => route.fulfill({ json: card }));
      await page.route('https://cards.scryfall.io/**', route => route.fulfill({ contentType: 'image/svg+xml', headers: { 'Access-Control-Allow-Origin': '*' }, body: '<svg xmlns="http://www.w3.org/2000/svg" width="488" height="684"><rect width="488" height="684" fill="#ded3b7"/></svg>' }));
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/inspector-payment.html`);
      await page.getByRole('button', { name: 'Toggle inspector' }).click();
      const stage = page.locator('.interactive-card-frame-stage');
      const flavorNode = page.getByLabel('Flavor text');
      await flavorNode.waitFor();
      await page.locator('[data-card-colors="sampled"]').waitFor();
      await page.evaluate(() => document.fonts.ready);
      const fittedSizes = [];
      for (const width of [350, 240, 350]) {
        await stage.evaluate((el, width) => { el.parentElement.style.width = `${width}px`; el.parentElement.style.height = `${Math.round(width * 1.65)}px`; }, width);
        await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
        await page.waitForFunction(() => {
          const box = document.querySelector('.interactive-card-frame__rules[data-fit-text="true"]');
          return box && parseFloat(box.style.getPropertyValue('--card-fitted-rules-font-size')) > 0 && box.scrollHeight <= box.clientHeight + 1 && box.scrollWidth <= box.clientWidth + 1;
        });
        fittedSizes.push(await flavorNode.evaluate(el => parseFloat(getComputedStyle(el).fontSize)));
        for (const color of ['rgb(20, 25, 30)', 'rgb(240, 235, 220)']) {
          await stage.evaluate((el, color) => el.style.setProperty('--sampled-rules-ink', color), color);
          const metrics = await flavorNode.evaluate(el => {
            const rules = el.closest('.interactive-card-frame__rules');
            const oracle = rules.querySelector('.interactive-card-frame__rule-line:not(.inspector-flavor-text)');
            const font = getComputedStyle(el), other = getComputedStyle(oracle);
            return { color: font.color, oracleColor: other.color, size: font.fontSize, oracleSize: other.fontSize, italic: font.fontStyle, family: font.fontFamily, overflow: rules.scrollWidth - rules.clientWidth, verticalOverflow: rules.scrollHeight - rules.clientHeight, overflowY: getComputedStyle(rules).overflowY, height: rules.clientHeight };
          });
          assert.equal(metrics.color, color);
          assert.equal(metrics.color, metrics.oracleColor);
          assert.equal(metrics.size, metrics.oracleSize);
          assert.equal(metrics.italic, 'italic');
          assert.match(metrics.family, /MPlantin/);
          assert.ok(metrics.overflow <= 1, JSON.stringify(metrics));
          assert.ok(metrics.height > 0);
          assert.ok(metrics.verticalOverflow <= 1, JSON.stringify(metrics));
          assert.equal(metrics.overflowY, 'hidden');
          assert.ok(parseFloat(metrics.size) > 0, JSON.stringify(metrics));
          if (long) assert.ok(parseFloat(metrics.size) < 11, 'dense text can shrink below the old minimum');
        }
        const bounds = await flavorNode.evaluate(el => {
          const rules = el.closest('.interactive-card-frame__rules');
          if (rules.scrollTop !== 0) throw new Error('Rules unexpectedly scrolled');
          return { flavor: el.getBoundingClientRect().toJSON(), box: rules.getBoundingClientRect().toJSON(), padding: parseFloat(getComputedStyle(rules).paddingBottom) };
        });
        assert.ok(bounds.flavor.left >= bounds.box.left);
        assert.ok(bounds.flavor.right <= bounds.box.right + 1);
        assert.ok(bounds.flavor.bottom <= bounds.box.bottom + 1);
        assert.ok(Math.abs(bounds.box.bottom - bounds.flavor.bottom - bounds.padding - 1) <= 2, JSON.stringify(bounds));
      }
      assert.ok(fittedSizes[1] < fittedSizes[0], 'text shrinks when the available box shrinks');
      assert.ok(Math.abs(fittedSizes[2] - fittedSizes[0]) < 0.1, 'text grows back to its original size when space returns');
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});
