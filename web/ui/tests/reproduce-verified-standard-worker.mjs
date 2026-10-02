import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

// Run the reconstructed position through the actual worker, alternating the
// visible optimistic runtime and the retained verification runtime.
const checkpoint = JSON.parse(await readFile(process.argv[2], 'utf8'));
const deployed = process.argv.includes('--deployed');
const original = JSON.parse(await readFile(process.env.REPRO_DIAGNOSTICS || `/Users/chiplis/Downloads/ironsmith-diagnostics-${checkpoint.perspective === 0 ? '1790907645176' : '1790907194743'}.json`, 'utf8'));
const startup = original.journal.entries.filter(e => e.seq <= 12 && !e.argsOmitted);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const server = deployed ? null : await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
  server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
if (server) await server.listen();
const base = deployed ? 'https://chiplis.com/ironsmith/' : `http://127.0.0.1:${server.httpServer.address().port}/`;
const browser = await chromium.launch();
try {
  const page = await browser.newPage();
  page.on('pageerror', error => console.error(error.message));
  await page.exposeFunction('report', row => console.log(JSON.stringify(row)));
  await page.route(base, route => route.fulfill({ contentType: 'text/html', body: '<title>Standard stall reproduction</title>' }));
  await page.goto(base);
  const decoderSource = await readFile(path.join(root, 'src/lib/snapshot-channel.js'), 'utf8');
  let workerPath = 'src/workers/wasmGameWorker.js';
  if (deployed) {
    const html = await (await fetch(base)).text();
    const entry = html.match(/src="\.\/(assets\/index-[^"]+\.js)"/)?.[1];
    if (!entry) throw new Error('Could not identify deployed application bundle');
    const bundle = await (await fetch(base + entry)).text();
    const worker = bundle.match(/wasmGameWorker-[\w-]+\.js/)?.[0];
    if (!worker) throw new Error('Could not identify deployed engine worker');
    workerPath = 'assets/' + worker;
  }
  console.log(JSON.stringify({ workerUrl: base + workerPath, perspective: checkpoint.perspective }));
  const rows = await page.evaluate(async ({ checkpoint, base, workerPath, decoderSource, startup, deployed }) => {
    const diagnostics = deployed ? null : await import('/src/lib/action-diagnostics.js');
    diagnostics?.resetDiagnostics();
    const { createSnapshotDecoder } = await import(URL.createObjectURL(new Blob([decoderSource], { type: 'text/javascript' })));
    const decoder = createSnapshotDecoder(), pending = new Map();
    const rows = [], worker = new Worker(base + workerPath, { type: 'module' });
    let id = 0;
    const ready = new Promise((resolve, reject) => {
      worker.onerror = error => reject(new Error(error.message));
      worker.onmessage = ({ data }) => {
        if (data.type === 'workerDiagnostics') { diagnostics?.recordWorkerTaskDiagnostics(data); return; }
        if (data.type === 'error') return reject(new Error(data.error.message));
        if (data.type === 'ready') return resolve();
        if (data.type !== 'result') return;
        const request = pending.get(data.id); if (!request) return;
        pending.delete(data.id);
        const decodeStartedAt = performance.now(), receivedAtWall = Date.now();
        if (!data.ok) request.reject(new Error(data.error.message));
        else request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
        diagnostics?.recordEngineResultReceipt(data.id, { sentAtWall: data.sentAtWall, receivedAtWall,
          snapshotDecodeMs: data.snapshot ? performance.now() - decodeStartedAt : 0 });
        diagnostics?.endEngineRequest(data.id);
      };
    });
    const call = async (method, args = [], runtimeBranch = null) => {
      const started = performance.now();
      await window.report({ begin: method, command: args[0]?.type, runtimeBranch });
      let timer;
      const value = await new Promise((resolve, reject) => {
        const requestId = ++id;
        pending.set(requestId, { resolve, reject });
        diagnostics?.beginEngineRequest(requestId, method, runtimeBranch);
        timer = setTimeout(() => reject(new Error(`Worker timed out: ${method}, branch ${runtimeBranch}`)), 60000);
        worker.postMessage({ type: 'call', id: requestId, method, args, runtimeBranch });
      }).finally(() => clearTimeout(timer));
      const row = { method, command: args[0]?.type, runtimeBranch, ms: +(performance.now() - started).toFixed(2),
        decision: value?.decision?.kind, player: value?.decision?.player, stack: value?.stack_preview, perf: value?.__perf };
      rows.push(row); await window.report(row); return value;
    };
    worker.postMessage({ type: 'init', assetBaseUrl: base });
    try {
      await ready;
      for (const entry of startup) await call(entry.method, entry.args || []);
      const config = startup.find(e => e.method === 'startMatch').args[0];
      await call('filterKnownCardNames', [config.publicDecklists.flat()]);
      await call('importSyncCheckpoint', [checkpoint, checkpoint.perspective]);
      const branch = await call('createRuntimeSavepoint');
      const sanctifier = checkpoint.objects.find(o => o.owner === 0 && o.zone === 'hand' && o.name === 'Hinterland Sanctifier');
      if (!sanctifier) throw new Error('Missing Sanctifier in reconstruction');
      const cast = { type: 'priority_action', action_ref: { kind: 'cast_spell', spell_id: sanctifier.id, from_zone: 'hand', casting_method: { kind: 'normal' } } };
      const visible = await call('dispatch', [cast]);
      const verified = await call('dispatch', [cast], branch);
      const payment = state => ({ type: 'mana_payment', response: { action: 'confirm', plan_id: state.decision.plan_id,
        request_hash: state.decision.request_hash, required_source_ids: [], excluded_source_ids: [], preserved_source_ids: [] } });
      await call('dispatch', [payment(visible)]);
      await call('previewCryptoRequirements', [payment(verified)], branch);
      await call('dispatch', [payment(verified)], branch);
      const assertBranchesAgree = async () => {
        const visible = await call('exportPublicAuditCheckpoint');
        const verified = await call('exportPublicAuditCheckpoint', [], branch);
        delete visible.__perf; delete verified.__perf;
        if (JSON.stringify(visible) !== JSON.stringify(verified)) throw new Error('Visible and verified public checkpoints differ');
      };
      await assertBranchesAgree();
      await Promise.all(Array.from({ length: 30 }, () => call('uiState')));
      // Repeat the boundary work used during signed action verification.
      for (let repeat = 0; repeat < 3; repeat++) {
        await call('uiState', [], branch);
        await call('exportPublicAuditCheckpoint', [], branch);
        await call('uiState');
      }
      await call('copyRuntimeSavepoint', [branch]);
      for (let step = 0; step < 4; step++) {
        const command = { type: 'priority_action', action_ref: { kind: 'pass_priority' } };
        await call('previewCryptoRequirements', [command], branch);
        const state = await call('dispatch', [command]);
        await call('dispatch', [command], branch);
        await assertBranchesAgree();
        await call('uiState');
        if (state.decision.kind !== 'priority') break;
      }
      await call('releaseRuntimeSavepoint', [branch]);
      if (diagnostics) {
        const deadline = performance.now() + 1000;
        while (diagnostics.getDiagnosticsSnapshot().workerTasks?.pendingCount && performance.now() < deadline) {
          await new Promise(resolve => setTimeout(resolve, 1));
        }
        const exported = diagnostics.exportDiagnostics();
        const tasks = exported.workerTasks;
        if (!tasks || tasks.pendingCount !== 0) throw new Error('Worker diagnostics retain a completed task');
        const branchTask = tasks.recent.find(t => t.method === 'dispatch' && t.runtimeBranch === branch);
        for (const phase of ['branch_enter', 'engine_call', 'branch_exit', 'snapshot_encode', 'response_post']) {
          if (!branchTask?.phases.some(p => p.phase === phase)) throw new Error(`Missing diagnostic phase: ${phase}`);
        }
        if (!tasks.resultReceipts.some(r => r.runtimeBranch === branch && r.deliveryDelayMs !== null)) {
          throw new Error('Missing response delivery diagnostics');
        }
        if (exported.engineRequests.count !== 0) throw new Error('Worker results left pending diagnostics requests');
        await window.report({ diagnosticsVerified: true, recentTasks: tasks.recent.length,
          resultReceipts: tasks.resultReceipts.length, branchPhases: branchTask.phases });
      }
      return rows;
    } finally { worker.terminate(); }
  }, { checkpoint, base, workerPath, decoderSource, startup, deployed });
  if (process.env.REPRO_REPORT) await writeFile(process.env.REPRO_REPORT, JSON.stringify(rows, null, 2));
} finally { await browser.close(); await server?.close(); }
