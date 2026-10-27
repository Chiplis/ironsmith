import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('object, option and target rows keep colored zone glyphs aligned and controller names distinct', async () => {
  const server = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await server.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('https://**/*',route=>route.fulfill({status:404,body:''}));
    for (const width of [1080,390]) {
      await page.setViewportSize({width,height:850});
      for (const kind of ['select_objects','select_options','targets']) {
        await page.mouse.move(0,0);
        await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/tests/decision-zone-preview.html?kind=${kind}`);
        await page.locator('.decision-object-zone').nth(7).waitFor();
        const row = page.locator('button.decision-option-row').filter({hasText:'Rhystic Study'});
        if (width === 1080 || kind === 'select_objects') await row.hover();
        if (width === 1080) {
          const preview = page.locator('.floating-card-preview[data-visible="true"][data-preview-object-id="3"]');
          await preview.waitFor();
          const frame = await preview.boundingBox();
          const menu = await page.locator('section.menu').boundingBox();
          assert.ok(frame && frame.width > 100 && frame.height > 100, `${kind} opens a card frame`);
          assert.ok(frame.x + frame.width <= menu.x || frame.x >= menu.x + menu.width
            || frame.y + frame.height <= menu.y || frame.y >= menu.y + menu.height, `${kind} keeps the decision menu unobstructed`);
          if (kind === 'select_objects') await page.screenshot({path:'../../output/decision-zones/hover-preview.png'});
        }
        const icon = row.locator('.decision-object-zone');
        assert.equal(await icon.getAttribute('aria-label'),'Battlefield · Owner: You · Controller: Bob');
        assert.equal(await icon.evaluate(n=>getComputedStyle(n).color),'rgb(183, 156, 255)');
        assert.equal(await row.locator('.decision-object-name').first().evaluate(n=>getComputedStyle(n).webkitTextFillColor),'rgb(255, 59, 48)');
        const positions = await page.locator('.decision-object-zone').evaluateAll(nodes=>nodes.map(n=>n.getBoundingClientRect().right));
        assert.ok(Math.max(...positions)-Math.min(...positions)<2, `${kind} icons align at ${width}px`);
        assert.ok(Math.max(...positions)<=width, 'icons fit viewport');
        if (width === 1080) {
          for (const id of [1, 2, 5, 6, 7, 8]) {
            await page.locator(`[data-decision-option-object="${id}"]`).hover();
            const preview = page.locator(`.floating-card-preview[data-visible="true"][data-preview-object-id="${id}"]`);
            await preview.waitFor();
            const frame = await preview.boundingBox();
            const menu = await page.locator('section.menu').boundingBox();
            assert.ok(frame.x + frame.width <= menu.x || frame.x >= menu.x + menu.width
              || frame.y + frame.height <= menu.y || frame.y >= menu.y + menu.height, `${kind} card ${id} avoids the menu`);
          }
        }
        if (width===1080 && kind==='select_objects') {
          await row.click();
          await page.getByRole('button',{name:/Submit/}).click();
          assert.match(await page.locator('output').textContent(), /"object_ids":\[3\]/);
        }
      }
    }
    assert.deepEqual(errors,[]);
  } finally { await browser.close(); await server.close(); }
});
