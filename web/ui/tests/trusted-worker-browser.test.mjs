import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('real authoritative worker streams startup, patches snapshots, restores savepoints and isolates previews', { timeout: 120000 }, async t => {
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  const requests = [];
  page.on('request', request => requests.push(request.url()));
  // A blank same-origin document avoids initializing the application's worker too.
  await page.route('**/worker-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Worker regression</title>' }));
  const port = server.httpServer.address().port;
  await page.goto(`http://127.0.0.1:${port}/worker-test`);
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
      const capabilities = await ready;
      await call('resetEmpty', ['Alice', 'Bob'], 20);
      await call('addCardToZone', 0, 'Island', 'battlefield', true);
      await call('addCardToZone', 0, 'Island', 'hand', true);
      await call('addCardToZone', 0, 'Island', 'graveyard', true);
      const before = await call('uiState');
      const savepoint = await call('createRuntimeSavepoint');
      await call('setLife', 0, 7);
      const edited = await call('uiState');
      const stableZones = before.players[0].hand_cards === edited.players[0].hand_cards
        && before.players[0].graveyard_cards === edited.players[0].graveyard_cards;
      const restored = await call('restoreRuntimeSavepoint', savepoint);
      const previewPromise = call('previewCastTargets', [], 0);
      await call('setLife', 0, 18);
      const live = await call('snapshot');
      const preview = await previewPromise;
      return { savepoints: capabilities.runtimeSavepoints, restoredLife: restored.players[0].life,
        stableZones, beforeLife: before.players[0].life, liveLife: live.players[0].life, preview, patches, full };
    } finally { worker.terminate(); }
  });
  assert.equal(result.savepoints, true);
  assert.equal(result.stableZones, true, 'unrelated player edits retain unchanged decoded zone arrays');
  assert.equal(result.restoredLife, result.beforeLife);
  assert.equal(result.liveLife, 18);
  assert.ok(result.patches > 0); assert.ok(result.full >= 2);
  assert.equal(result.preview, null, 'a mutation cancels previews captured at the old revision');
  assert.equal(requests.some(url => /(?:compiler|verifier)_bg.*wasm/.test(url)), false, 'Trusted startup reuses the engine source compiler without downloading duplicate compiler/verifier modules');
});
