import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('resting hands clear field objects while hover and hotkey reveal float freely', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    for (const width of [1024, 1365, 2048]) {
      const page = await browser.newPage({ viewport: { width, height: 900 } });
      await page.route('https://**/*', route => route.abort());
      await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/hand-hover-frame-size.html`, { timeout: 120000 });
      await page.evaluate(async () => {
        const { installHandToggleShortcut } = await import('/src/lib/hand-open.js');
        installHandToggleShortcut(document);
      });
      await page.locator('.hand-card').first().waitFor();
      await page.waitForTimeout(600);
      await page.evaluate(() => {
        const top = Math.min(...[...document.querySelectorAll('.hand-card')].map(el => el.getBoundingClientRect().top));
        const field = document.createElement('div');
        field.id = 'clearance-obstacle';
        field.className = 'battlefield-row';
        field.style.cssText = `position:fixed;left:35%;width:30%;top:${top - 40}px;height:110px;min-height:0;padding:0;border:0;`;
        const card = document.createElement('div');
        card.className = 'game-card';
        card.style.cssText = 'all:unset;display:block;width:100%;height:100%';
        field.append(card);
        document.body.append(field);
      });
      const assertClear = async () => {
        await page.waitForTimeout(650);
        const result = await page.evaluate(() => {
          const obstacle = document.querySelector('#clearance-obstacle').getBoundingClientRect();
          const cards = [...document.querySelectorAll('.hand-card')].map(el => el.getBoundingClientRect())
            .filter(rect => rect.left < obstacle.right && rect.right > obstacle.left);
          return { offset: parseFloat(document.querySelector('.hand-reveal-shell').style.translate.split(' ')[1]),
            gap: Math.min(...cards.map(rect => rect.top - obstacle.bottom)) };
        });
        assert.ok(result.offset > 0, `hand moves down at ${width}px`);
        assert.ok(result.gap >= 11, `field stays clear at ${width}px: ${JSON.stringify(result)}`);
      };
      await assertClear();
      await page.evaluate(() => {
        const field = document.querySelector('#clearance-obstacle');
        field.battlefieldScene = { bounds: () => [{ left: 0, right: field.clientWidth, top: 0, bottom: 110 }] };
        window.dispatchEvent(new Event('ironsmith:battlefield-layout-fitted'));
      });
      await assertClear();
      await page.locator('.hand-card[data-object-id="4"]').dispatchEvent('mouseover');
      await page.waitForTimeout(650);
      assert.equal(await page.locator('.hand-reveal-shell').evaluate(el => el.style.translate), '', 'hover restores floating placement');
      await page.mouse.move(1, 1);
      await page.locator('.hand-zone-scroll').dispatchEvent('pointerout', { pointerType: 'mouse' });
      await assertClear();
      await page.keyboard.press('.');
      await page.waitForTimeout(650);
      assert.equal(await page.locator('.hand-reveal-shell').evaluate(el => el.style.translate), '', 'hotkey reveal restores floating placement');
      await page.keyboard.press('.');
      await assertClear();
      await page.evaluate(() => document.querySelector('#clearance-obstacle').remove());
      await page.waitForTimeout(650);
      assert.equal(await page.locator('.hand-reveal-shell').evaluate(el => el.style.translate), '', 'hand returns to its original placement');
      await page.close();
    }
  } finally { await browser.close(); await vite.close(); }
});
