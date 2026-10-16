import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { chromium } from 'playwright';

test('visible library covers preview, multiple cards expand, and Command hugs the player name', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1440,height:900}});
    const url = `http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html?commander`;
    await page.goto(`${url}&libraryVisible=1`);
    const deck = page.locator('[data-zone-pile="library"][data-zone-owner="0"]');
    await deck.waitFor();
    assert.equal(await deck.getAttribute('data-object-id'), '7101');
    assert.equal(await page.locator('[data-zone-pile="look"]').count(), 0);
    const dock = page.locator('.local-command-zone-dock');
    await dock.locator('[data-zone-pile="command"]').waitFor();
    const bounds = await dock.boundingBox();
    const name = await page.locator('.table-shared-player-header .player-header-name-stack').boundingBox();
    assert.ok(Math.abs(bounds.x - name.x - name.width - 6) < 1);
    assert.ok(Math.abs(bounds.y + bounds.height / 2 - name.y - name.height / 2) < 1);
    await page.screenshot({path:'/tmp/library-command-layout.png'});
    await deck.hover();
    await page.locator('[data-card-hover-preview][data-preview-object-id="7101"][data-visible="true"]').waitFor();
    assert.equal(await deck.getAttribute('aria-expanded'), null, 'one card previews without expanding');
    await page.mouse.move(0, 0);
    await page.goto(`${url}&libraryVisible=2`);
    await deck.waitFor();
    const deckBounds = await deck.boundingBox();
    await page.mouse.move(deckBounds.x + deckBounds.width / 2, deckBounds.y + deckBounds.height / 2);
    const strip = page.locator('[data-local-zone-strip="true"]');
    await strip.waitFor();
    assert.equal(await strip.locator('[data-zone-card="library"]').count(), 2);
    await page.goto(url);
    assert.equal(await deck.getAttribute('data-object-id'), null);
    assert.equal(await deck.locator('img').count(), 1);
  } finally { await browser.close(); await vite.close(); }
});
