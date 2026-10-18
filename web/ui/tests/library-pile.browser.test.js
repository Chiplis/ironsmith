import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { chromium } from 'playwright';

test('known library cards mark the Deck chip with an eye and open on hover in library order', async () => {
  const vite = await createServer({server:{host:'127.0.0.1',port:0},logLevel:'silent'});
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({viewport:{width:1440,height:900}});
    const url = `http://127.0.0.1:${vite.httpServer.address().port}/tests/zone-piles-table.html`;
    const deck = page.locator('[data-zone-pile="library"][data-zone-owner="0"]');
    await page.goto(url);
    await deck.waitFor();
    assert.equal(await deck.getAttribute('data-looking'), null, 'an unknown library shows no eye');
    assert.equal(await deck.locator('.zone-pile-count').textContent(), '40');
    await page.goto(`${url}?libraryVisible=2`);
    await deck.waitFor();
    assert.equal(await deck.getAttribute('data-looking'), 'true');
    assert.equal(await deck.locator('.zone-pile-look').count(), 1);
    assert.equal(await deck.locator('.zone-pile-count').textContent(), '40', 'the count stays the deck size');
    const deckBounds = await deck.boundingBox();
    await page.mouse.move(deckBounds.x + deckBounds.width / 2, deckBounds.y + deckBounds.height / 2);
    const strip = page.locator('[data-local-zone-strip="true"]');
    await strip.waitFor();
    // Deepest first, so the top card ends the strip beside the deck.
    assert.deepEqual(await strip.locator('[data-zone-card="library"]').evaluateAll(cards => cards.map(card => card.dataset.objectId)), ['7102', '7101']);
    assert.equal(await strip.locator('.zone-pile-position').first().textContent(), '2');
    assert.equal(await strip.locator('.zone-pile-position').last().textContent(), 'Top');
  } finally { await browser.close(); await vite.close(); }
});
