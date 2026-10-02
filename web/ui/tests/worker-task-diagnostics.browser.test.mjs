import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('a blocked worker remains identifiable in a main-thread diagnostics export', { timeout: 30000 }, async t => {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const server = await createServer({ root, configFile: path.join(root, 'vite.config.js'), logLevel: 'error',
    optimizeDeps: { noDiscovery: true, include: [] },
    server: { host: '127.0.0.1', port: 0, hmr: false, watch: null } });
  await server.listen(); t.after(() => server.close());
  const browser = await chromium.launch(); t.after(() => browser.close());
  const page = await browser.newPage();
  await page.route('**/worker-diagnostics-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Worker diagnostics</title>' }));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/worker-diagnostics-test`);
  const report = await page.evaluate(async () => {
    const diagnostics = await import('/src/lib/action-diagnostics.js');
    diagnostics.resetDiagnostics();
    const source = `import { createWorkerTaskDiagnostics } from '${location.origin}/src/lib/worker-task-diagnostics.js';
      const tracker = createWorkerTaskDiagnostics({ publish: message => postMessage(message) });
      const task = tracker.create({ requestId: 1, method: 'dispatch', runtimeBranch: 2 });
      tracker.enqueue(task); tracker.start(task); tracker.phase(task, 'engine_call');
      const deadline = performance.now() + 1000;
      while (performance.now() < deadline) {} // Deliberately block only this test worker.
      tracker.phase(task, 'response_post'); tracker.finish(task); postMessage({ type: 'done' });`;
    const url = URL.createObjectURL(new Blob([source], { type: 'text/javascript' }));
    const worker = new Worker(url, { type: 'module' });
    try {
      return await new Promise((resolve, reject) => {
        worker.onerror = event => reject(new Error(event.message));
        worker.onmessage = ({ data }) => {
          if (data.type === 'done') return reject(new Error('Worker finished before stalled diagnostics were exported'));
          diagnostics.recordWorkerTaskDiagnostics(data);
          if (data.state?.active?.phase === 'engine_call') resolve(diagnostics.exportDiagnostics());
        };
      });
    } finally { worker.terminate(); URL.revokeObjectURL(url); }
  });
  assert.equal(report.workerTasks.active.method, 'dispatch');
  assert.equal(report.workerTasks.active.runtimeBranch, 2);
  assert.equal(report.workerTasks.active.phase, 'engine_call');
  assert.equal(report.workerTasks.pendingCount, 1);
});
