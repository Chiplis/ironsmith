import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('token tiles retain an arched silhouette and readable name bar on desktop and mobile', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.route('https://**/*', route => route.fulfill({ status: 404, body: '' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/mobile-arena-cards.html`);
    for (const viewport of [{width:1440,height:900}, {width:844,height:390}]) {
      await page.setViewportSize(viewport);
      const tower = page.locator('.arena-permanent').filter({has:page.locator('.arena-permanent-name[title="Command Tower"]')});
      assert.equal(await tower.locator('.arena-land-mana-strip > [data-mana-color]').count(), 5);
      const wastes = page.locator('.arena-permanent').filter({has:page.locator('.arena-permanent-name[title="Wastes"]')});
      assert.equal(await wastes.locator('.arena-land-mana-strip > [data-mana-color="C"]').count(), 1);
      const fetch = page.locator('.arena-permanent').filter({has:page.locator('.arena-permanent-name[title="Misty Rainforest"]')});
      assert.equal(await fetch.locator('.arena-land-mana-strip').count(), 0);
      const fountain = page.locator('.arena-permanent').filter({has:page.locator('.arena-permanent-name[title="Hallowed Fountain"]')});
      assert.deepEqual(await fountain.locator('.arena-land-mana-strip > [data-mana-color]').evaluateAll(els=>els.map(el=>el.dataset.manaColor)), ['W','U']);
      const strip = await tower.locator('.arena-land-mana-strip').boundingBox();
      const art = await tower.locator('.arena-permanent-art').boundingBox();
      assert.ok(art.y + art.height <= strip.y + 1);
      const token = page.locator('.arena-permanent[data-token]');
      const ordinary = page.locator('.arena-permanent[data-kind="creature"]:not([data-token])');
      await token.waitFor();
      assert.equal(await token.evaluate(el => getComputedStyle(el).borderTopLeftRadius), '48%');
      assert.equal(await ordinary.evaluate(el => getComputedStyle(el).borderTopLeftRadius), '4px');
      assert.equal(await token.locator('.arena-permanent-name').innerText(), 'Llanowar Elves');
      const tapped = page.locator('.battlefield-arena-card.tapped');
      const transforms = await tapped.evaluate(el => ['.game-card-surface', '.battlefield-group-stack'].map(selector => getComputedStyle(el.querySelector(selector)).transform));
      assert.notEqual(transforms[0], 'none');
      assert.equal(transforms[0], transforms[1], 'backing cards and front face tap together');
      assert.equal(await tapped.locator('.arena-permanent').evaluate(el => getComputedStyle(el).transform), 'none', 'face must not rotate twice');
      const bounds = await token.boundingBox(), title = await token.locator('.arena-permanent-name').boundingBox();
      assert.ok(title.y > bounds.y && title.y + title.height < bounds.y + bounds.height);
      assert.ok(title.x > bounds.x && title.x + title.width < bounds.x + bounds.width);
    }
    await page.screenshot({path:'/tmp/token-silhouettes.png'});
  } finally { await browser.close(); await vite.close(); }
});
