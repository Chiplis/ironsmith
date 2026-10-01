import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { chromium } from 'playwright';
import { createServer } from 'vite';

// Feed the reconstructed checkpoint from reproduce-verified-preview-stall.mjs
// through the actual browser worker, including deferred priority analysis.
const checkpointPath = process.argv[2];
if (!checkpointPath) throw new Error('Pass the reconstructed checkpoint JSON path');
const checkpoint = JSON.parse(await readFile(checkpointPath, 'utf8'));
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
  server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
await server.listen();
const browser = await chromium.launch();
try {
  const page = await browser.newPage();
  page.on('pageerror', error => console.error(error.message));
  await page.route('**/preview-stall-repro', route => route.fulfill({ contentType: 'text/html', body: '<title>Preview stall reproduction</title>' }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/preview-stall-repro`);
  const rows = await page.evaluate(async checkpoint => {
    const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
    const decoder = createSnapshotDecoder(), pending = new Map(), analyses = new Map(), waiters = new Map();
    const rows = [], worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
    let id = 0;
    const ready = new Promise((resolve, reject) => {
      worker.onerror = error => reject(new Error(error.message));
      worker.onmessage = ({ data }) => {
        if (data.type === 'error') return reject(new Error(data.error.message));
        if (data.type === 'ready') return resolve();
        if (data.type === 'priorityAnalysis') { analyses.set(data.revision, data.decision); waiters.get(data.revision)?.(data.decision); return; }
        if (data.type !== 'result') return;
        const request = pending.get(data.id); if (!request) return;
        pending.delete(data.id);
        if (!data.ok) request.reject(new Error(data.error.message));
        else {
          const value = data.snapshot ? decoder.decode(data.snapshot) : data.result;
          request.resolve({ value, perf: value?.__perf });
        }
      };
    });
    const call = async (method, ...args) => {
      const start = performance.now();
      const { value, perf } = await new Promise((resolve, reject) => {
        pending.set(++id, { resolve, reject }); worker.postMessage({ type: 'call', id, method, args });
      });
      const row = { method, command: args[0]?.type, ms: +(performance.now() - start).toFixed(2), perf };
      rows.push(row);
      if (value?.decision?.analysis_complete === false) {
        value.decision = analyses.get(value.__priority_revision) || await new Promise(resolve => waiters.set(value.__priority_revision, resolve));
        waiters.delete(value.__priority_revision);
      }
      return value;
    };
    worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
    try {
      await ready;
      await call('importSyncCheckpoint', checkpoint, 1);
      let state = await call('uiState');
      const cast = state.decision.actions.find(a => a.action_ref?.kind === 'cast_spell' && a.label.includes('Graveyard Trespasser'));
      if (!cast) throw new Error('Missing reconstructed cast action');
      let command = { type: 'priority_action', action_ref: cast.action_ref };
      for (let step = 0; step < 12; step++) {
        await call('previewCryptoRequirements', command);
        state = await call('dispatch', command);
        if (state.decision.kind === 'priority') break;
        if (state.decision.kind === 'mana_payment') command = { type: 'mana_payment', response: {
          action: 'confirm', plan_id: state.decision.plan_id, request_hash: state.decision.request_hash } };
        else if (state.decision.kind === 'select_options') {
          const option = state.decision.options.find(o => o.description === 'Black') || state.decision.options.find(o => o.legal !== false);
          command = { type: 'select_options', option_indices: [option.index] };
        } else throw new Error(`Unexpected decision ${state.decision.kind}`);
      }
      return rows;
    } finally { worker.terminate(); }
  }, checkpoint);
  console.log(JSON.stringify(rows, null, 2));
  if (process.env.REPRO_REPORT) await writeFile(process.env.REPRO_REPORT, JSON.stringify(rows, null, 2));
} finally { await browser.close(); await server.close(); }
