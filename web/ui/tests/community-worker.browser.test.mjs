import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('Community custom definitions load into opening hands and replay through the real worker', { timeout: 120000 }, async t => {
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
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
    const analyses = new Map();
    const ready = new Promise((resolve, reject) => {
      worker.onerror = error => reject(new Error(error.message));
      worker.onmessage = ({ data }) => {
        if (data.type === 'error') { reject(new Error(data.error.message)); return; }
        if (data.type === 'ready') { resolve(data); return; }
        if (data.type === 'priorityAnalysis') analyses.set(data.revision, data);
        if (data.type === 'priorityAnalysisError') analyses.set(data.revision, data);
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
      const draft = { layout: 'single', faces: [{ name: 'Community Test Bear', manaCost: '{1}{G}', cardTypes: ['Land'], supertypes: ['Basic'], subtypes: ['Forest'], oracleText: '{T}: Stargaze.' }], customMechanics: [{ keyword: 'stargaze', reminder: 'Draw a card.', sourceCard: 'Community Test Bear', version: 1 }] };
      const config = { playerNames: ['Host', 'Guest'], startingLife: 20, format: 'normal', seed: 42,
        decks: [Array(60).fill('Community Test Bear'), Array(60).fill('Community Test Bear')], openingHandSize: 7,
        communityFormat: { name: 'Bear format', customCards: [{ id: 'bear', draft }] } };
      const { encodeCommunityShare, decodeCommunityShare } = await import('/src/lib/community-sharing.js');
      config.communityFormat = decodeCommunityShare(encodeCommunityShare('format', config.communityFormat)).data;
      const validation = await call('validateMatchConfig', config);
      if (validation.valid === false) throw new Error(JSON.stringify(validation));
      await call('startMatch', config);
      const before = await call('uiState');
      await call('replayTrustedMatch', config, [], 0);
      const after = await call('uiState');
      return { before: before.players[0].hand_cards, after: after.players[0].hand_cards };
    } finally { worker.terminate(); }
  });
  assert.equal(result.before.length, 7);
  assert.ok(result.before.every(card => card.name === 'Community Test Bear'), JSON.stringify(result.before));
  assert.deepEqual(result.after.map(card => card.name), result.before.map(card => card.name));
});
