import test from 'node:test';
import assert from 'node:assert/strict';
import {chromium} from 'playwright';
import {createServer} from 'vite';

test('card-glint renders and activates while printing metadata is pending', {timeout:60000}, async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'error'});
  await vite.listen();
  const browser = await chromium.launch();
  let releaseMetadata;
  const gate = new Promise(resolve => { releaseMetadata = resolve; });
  try {
    const page = await browser.newPage();
    const id = '11111111-1111-1111-1111-111111111111';
    let requested = false;
    await page.route(`https://api.scryfall.com/cards/${id}`, async route => {
      requested = true;
      await gate;
      await route.fulfill({json:{id,frame:'1997',rarity:'rare',set:'tmp'}});
    });
    await page.route('https://cards.scryfall.io/**', route => route.fulfill({
      contentType:'image/svg+xml',body:'<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="blue"/></svg>',
    }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/card-glint.html?warm`);
    await page.waitForFunction(() => window.setGlintArt);
    await page.evaluate(id => {
      window.setGlintArt(`https://cards.scryfall.io/normal/front/1/1/${id}.jpg`);
      window.showGlintFrame();
    }, id);
    await page.locator('[data-card-glint-frame="ready"]').waitFor({timeout:45000});
    assert.ok(requested, 'metadata request is still pending');
    await page.getByRole('button',{name:'Glint Test Adept: {T}: Draw a card.'}).click();
    assert.equal(await page.evaluate(() => window.glintActivated), 'draw');
    const before = await page.locator('canvas').evaluate(canvas => canvas.toDataURL());
    releaseMetadata();
    await page.waitForFunction(before => {
      const canvas = document.querySelector('[data-card-glint-frame="ready"]');
      return canvas && canvas.toDataURL() !== before;
    }, before);
  } finally {
    releaseMetadata();
    await browser.close();
    await vite.close();
  }
});
