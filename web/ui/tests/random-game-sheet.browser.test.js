import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

async function harness({ missing = false } = {}) {
  const vite = await createServer({ root: new URL('../', import.meta.url).pathname, server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1000, height: 900 }, reducedMotion: 'reduce' });
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/catalog/modern/*.json', route => {
    const file = new URL(route.request().url()).pathname.split('/').at(-1);
    if (missing) return route.fulfill({ status: 404, json: {} });
    const mainboard = [{ name: 'Forest', count: 24 }, { name: 'Grizzly Bears', count: 20 }, { name: 'Giant Growth', count: 16 }];
    return route.fulfill({ json: file === 'index.json' ? { decks: [{ detail: 'deck.json' }] }
      : file === 'search-index.json' ? {} : { mainboard, sideboard: [{ name: 'Omniscience', count: 1 }] } });
  });
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/random-game-sheet.html`);
  await page.locator('[data-random-game-trigger]').click();
  return { page, errors, close: async () => { await browser.close(); await vite.close(); } };
}

test('random game dialog creates two complete catalog deck positions', { timeout: 120000 }, async () => {
  const { page, errors, close } = await harness();
  try {
    await page.getByLabel('Seed', { exact: true }).fill('browser-seed');
    await page.locator('.random-game-submit').click();
    await page.waitForFunction(() => window.__generated.length > 0);
    const [{ payload, message }] = await page.evaluate(() => window.__generated);
    assert.match(message, /seed browser-seed/);
    assert.equal(payload.players.length, 2);
    for (const player of payload.players) {
      const all = Object.values(player.zones).flat();
      assert.equal(all.length, 60);
      assert.equal(all.filter(name => name === 'Forest').length, 24);
      assert.equal(all.filter(name => name === 'Grizzly Bears').length, 20);
      assert.equal(all.filter(name => name === 'Giant Growth').length, 16);
      assert.ok(!all.includes('Omniscience'));
      assert.equal(player.zones.hand.length, 7);
      assert.ok(player.zones.battlefield.every(name => name !== 'Giant Growth'));
    }
    await page.locator('.random-game-sheet').waitFor({ state: 'detached' });
    assert.deepEqual(errors, []);
  } finally { await close(); }
});

test('missing catalog reports a failure without generating a different card pool', { timeout: 120000 }, async () => {
  const { page, errors, close } = await harness({ missing: true });
  try {
    await page.locator('.random-game-submit').click();
    await page.waitForFunction(() => window.__statuses.some(status => status.isError));
    const statuses = await page.evaluate(() => window.__statuses);
    assert.match(statuses[0].message, /No deck catalog found/);
    assert.deepEqual(await page.evaluate(() => window.__generated), []);
    assert.deepEqual(errors, []);
  } finally { await close(); }
});
