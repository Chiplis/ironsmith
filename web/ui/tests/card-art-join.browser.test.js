import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('embedded art borders join the title and P/T stays at the lower right', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const image = 'https://cards.scryfall.io/art_crop/front/a/b/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jpg';
    const card = { name: 'Ornithopter', image_uris: {art_crop:image, normal:image.replace('/art_crop/', '/normal/')}, flavor_text:'An ingenious little machine.' };
    await page.route('**/cards/ornithopter.json', r => r.fulfill({json:{scryfall:card}}));
    await page.route('https://api.scryfall.com/**', r => r.fulfill({json:card}));
    await page.route('https://cards.scryfall.io/**', r => r.fulfill({contentType:'image/svg+xml', headers:{'Access-Control-Allow-Origin':'*'},body:'<svg xmlns="http://www.w3.org/2000/svg" width="488" height="356"><rect width="488" height="356" fill="#534332"/><rect x="24" y="0" width="440" height="356" fill="#b4d2f0"/></svg>'}));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-art-join.html`);
    await page.locator('[data-art-title-rails="true"]').waitFor();
    await page.evaluate(() => document.fonts.ready);
    assert.equal(await page.locator('.interactive-card-frame__zone').count(), 0, 'battlefield badge is hidden');
    const landColors = await page.evaluate(async url => {
      const { sampleCardFrameColors } = await import('/src/lib/card-frame-colors.js');
      return sampleCardFrameColors(url, { textures: false });
    }, image.replace('/art_crop/', '/normal/'));
    assert.ok(landColors['--sampled-rules-paper']);
    assert.ok(Object.keys(landColors).every(key => !/texture|rails|veil/.test(key)), 'land mode uses only sampled colors even after the textured result is cached');

    for (const width of [420, 240]) {
      await page.locator('#card-host').evaluate((el,w) => {el.style.width=`${w}px`;el.style.height=`${w*1.65}px`;}, width);
      const backgrounds = await page.locator('.interactive-card-frame__title-row, .interactive-card-frame__type-row').evaluateAll(els => els.map(el => getComputedStyle(el).backgroundImage));
      assert.match(backgrounds[0], /data:image\/png/);
      assert.equal(backgrounds[0], backgrounds[1]);
      const title = await page.locator('.interactive-card-frame__title-row').boundingBox();
      const art = await page.locator('.interactive-card-frame__art').boundingBox();
      const stats = await page.locator('.interactive-card-frame__art-stats').boundingBox();
      assert.ok(Math.abs(title.y + title.height - art.y) <= 1);
      assert.ok(stats.x > art.x + art.width / 2);
      assert.ok(stats.x + stats.width <= art.x + art.width);
      assert.ok(stats.y + stats.height <= art.y + art.height);
      assert.equal(await page.locator('.interactive-card-frame__art-stats').textContent(),'0/2');
      assert.match(await page.locator('.interactive-card-frame__title-row').evaluate(el=>getComputedStyle(el,'::before').backgroundImage),/data:image\/png/);
    }
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-art-join.html?zone=Hand`);
    const zone = page.locator('.interactive-card-frame__art .interactive-card-frame__zone');
    await zone.waitFor();
    assert.equal(await zone.textContent(), 'Hand');
    assert.equal(await page.locator('.interactive-card-frame__type-row .interactive-card-frame__zone').count(), 0);
    const badgeBox = await zone.boundingBox();
    const artBox = await page.locator('.interactive-card-frame__art').boundingBox();
    assert.ok(badgeBox.x > artBox.x + artBox.width / 2);
    assert.ok(badgeBox.y < artBox.y + artBox.height / 4);
  } finally {await browser.close();await vite.close();}
});
