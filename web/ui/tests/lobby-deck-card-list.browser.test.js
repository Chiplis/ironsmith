import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('Lobby deck card list groups rows, flags problems and previews on hover', { timeout: 60000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const base = `http://127.0.0.1:${vite.httpServer.address().port}/tests/lobby-deck-card-list.html`;
    await page.goto(`${base}?commander`);
    const rows = page.locator('.lobby-deck-card-list__row');
    await rows.first().waitFor();
    assert.deepEqual(await page.locator('[data-section]').evaluateAll(nodes => nodes.map(node => node.dataset.section)),
      ['commander', 'main', 'sideboard']);
    const bolt = page.locator('[data-section="main"] [data-card-name="Lightning Bolt"]');
    assert.equal(await bolt.count(), 1);
    assert.equal(await bolt.locator('.lobby-deck-card-list__count').textContent(), '8');
    assert.match(await page.locator('[data-section="main"] .lobby-deck-card-list__section-count').textContent(), /^30$/);
    const misstep = page.locator('[data-section="main"] [data-card-name="Mental Misstep"]');
    assert.equal(await misstep.getAttribute('title'), 'Banned in Modern');
    assert.ok(await misstep.evaluate(node => node.classList.contains('is-problem')));
    assert.equal(await misstep.locator('.lobby-deck-card-list__problem').textContent(), 'Banned in Modern');

    const scrollBefore = await page.locator('.lobby-deck-card-list__scroll').boundingBox();
    await bolt.hover();
    const preview = page.locator('.lobby-deck-card-list__preview[data-preview-card="Lightning Bolt"]');
    await preview.waitFor();
    const box = await preview.boundingBox();
    assert.ok(box.x >= scrollBefore.x + scrollBefore.width, 'preview sits beside the list');
    assert.deepEqual(await page.locator('.lobby-deck-card-list__scroll').boundingBox(), scrollBefore);
    await page.mouse.move(1200, 780);
    await preview.waitFor({ state: 'detached' });

    await misstep.focus();
    await page.locator('.lobby-deck-card-list__preview[data-preview-card="Mental Misstep"]').waitFor();

    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(`${base}?commander`);
    const scroll = page.locator('.lobby-deck-card-list__scroll');
    await scroll.waitFor();
    assert.equal(await scroll.evaluate(node => node.scrollWidth <= node.clientWidth), true);

    await page.goto(`${base}?empty`);
    await page.getByText('No cards yet').waitFor();
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
