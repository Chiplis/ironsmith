import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

async function harness(t) {
  const server = await createServer({ root, logLevel: 'silent',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen();
  const browser = await chromium.launch();
  t.after(async () => { await browser.close(); await server.close(); });
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const cardRequests = [], errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route(/\/cards\/[^/?]+\.json(?:\?|$)/, route => {
    cardRequests.push(route.request().url());
    return route.abort('failed');
  });
  await page.route('https://api.scryfall.com/**', route => route.fulfill({ status: 404, body: '' }));
  await page.route('https://cards.scryfall.io/**', route => route.fulfill({
    contentType: 'image/svg+xml', headers: { 'access-control-allow-origin': '*' },
    body: '<svg xmlns="http://www.w3.org/2000/svg" width="488" height="680"><rect width="488" height="680" fill="#81744e"/></svg>',
  }));
  return { page, cardRequests, errors, url: `http://127.0.0.1:${server.httpServer.address().port}` };
}

test('embedded catalogue preserves complete source payloads and loads cards with card JSON requests blocked', { timeout: 120000 }, async t => {
  const { page, cardRequests, errors, url } = await harness(t);
  await page.route('**/embedded-catalog-probe', route => route.fulfill({ contentType: 'text/html', body: '<title>Embedded catalogue</title>' }));
  await page.goto(`${url}/embedded-catalog-probe`);
  const routes = ['lightning-bolt', 'fire-ice', 'ice', 'storm-crow'];
  const sources = await Promise.all(routes.map(route => readFile(path.join(root, 'public/cards', `${route}.json`), 'utf8').then(JSON.parse)));
  const result = await page.evaluate(async ({ routes }) => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    const decoder = createSnapshotDecoder(), pending = new Map();
    const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
    let id = 0;
    const ready = new Promise((resolve, reject) => {
      worker.onerror = error => reject(new Error(error.message));
      worker.onmessage = ({ data }) => {
        if (data.type === 'error') return reject(new Error(data.error.message));
        if (data.type === 'ready') return resolve();
        if (data.type !== 'result') return;
        const request = pending.get(data.id); if (!request) return;
        pending.delete(data.id);
        if (!data.ok) request.reject(new Error(data.error.message));
        else request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
      };
    });
    const call = (method, ...args) => new Promise((resolve, reject) => {
      pending.set(++id, { resolve, reject }); worker.postMessage({ type: 'call', id, method, args });
    });
    worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
    try {
      await ready;
      const index = JSON.parse(await call('getEmbeddedCardCatalogIndexJson'));
      const payloads = await Promise.all(routes.map(async route => JSON.parse(await call('getEmbeddedCardSourceJson', route))));
      const unknown = await call('getEmbeddedCardSourceJson', 'not-a-real-card');
      const names = await call('autocompleteCardNames', 'Storm Crow', 10);
      await call('resetEmpty', ['Alice', 'Bob'], 20);
      const details = [];
      for (const name of ['Lightning Bolt', 'Fire // Ice', 'Storm Crow']) {
        const object = await call('addCardToZone', 0, name, 'hand', true);
        details.push(await call('objectDetails', object));
      }
      return { routeCount: index.routeCount, payloads, unknown: unknown ?? null, names, details };
    } finally { worker.terminate(); }
  }, { routes });
  assert.ok(result.routeCount > 30000);
  assert.deepEqual(result.payloads, sources, 'full source, compiled artifacts, aliases, and Scryfall metadata survive embedding');
  assert.equal(result.unknown, null);
  assert.ok(result.names.includes('Storm Crow'));
  assert.equal(result.details[0].name, 'Lightning Bolt');
  assert.match(result.details[1].name, /Fire/);
  assert.equal(result.details[2].name, 'Storm Crow');
  assert.deepEqual(cardRequests, []);
  assert.deepEqual(errors, []);
});

test('Add Card search and insertion use the embedded catalogue without card JSON requests', { timeout: 120000 }, async t => {
  const { page, cardRequests, errors, url } = await harness(t);
  await page.goto(url, { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: /^add card$/i }).click({ timeout: 60000 });
  await page.getByPlaceholder('Card name', { exact: true }).fill('Storm Crow');
  await page.getByRole('option', { name: 'Storm Crow', exact: true }).waitFor();
  await page.getByRole('option', { name: 'Storm Crow', exact: true }).click();
  await page.getByLabel('Zone', { exact: true }).selectOption('battlefield');
  await page.locator('.add-card-submit').click();
  await page.locator('.game-card[data-card-name="Storm Crow"]').first().waitFor({ state: 'attached' });
  assert.deepEqual(cardRequests, [], 'startup, autocomplete, metadata and card insertion need no card JSON HTTP requests');
  assert.deepEqual(errors, []);
});
