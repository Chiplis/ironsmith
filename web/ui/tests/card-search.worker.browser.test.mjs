import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('Build worker search finds accented card names from unaccented queries', { timeout: 120000 }, async t => {
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error', optimizeDeps: {noDiscovery: true, entries: []},
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  await page.route('**/sandbox-worker-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Sandbox test</title>' }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/sandbox-worker-test`);
  const result = await page.evaluate(async () => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    const decoder = createSnapshotDecoder(), pending = new Map();
    const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
    let id = 0, patches = 0, full = 0;
    const ready = new Promise((resolve, reject) => {
      worker.onerror = error => reject(new Error(error.message));
      worker.onmessage = ({ data }) => {
        if (data.type === 'error') { reject(new Error(data.error.message)); return; }
        if (data.type === 'ready') { resolve(data); return; }
        if (data.type !== 'result') return;
        const request = pending.get(data.id); if (!request) return;
        pending.delete(data.id);
        if (!data.ok) { request.reject(new Error(data.error.message)); return; }
        if (data.snapshot) {
          if ('full' in data.snapshot) full++; else patches++;
          request.resolve(decoder.decode(data.snapshot));
        } else request.resolve(data.result);
      };
    });
    const call = (method, ...args) => new Promise((resolve, reject) => {
      pending.set(++id, { resolve, reject }); worker.postMessage({ type: 'call', id, method, args });
    });
    worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
    try {
      await ready;
      const results = {};
      for (const query of ['Kili', 'KÍLI', 'Ki\u0301li', 'Resourceful']) {
        results[query] = await call('autocompleteCardNames', query, 8);
      }
      return results;
    } finally { worker.terminate(); }
  });
  for (const [query, names] of Object.entries(result)) {
    assert.ok(names.includes('Kíli the Resourceful'), `${query}: ${JSON.stringify(names)}`);
  }
});
