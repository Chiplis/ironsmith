import test from 'node:test';
import assert from 'node:assert/strict';
import { Buffer } from 'node:buffer';
import { webkit, devices } from 'playwright';
import { createServer } from 'vite';

test('iPhone keeps both hands and analyzes subsequent turns without a second engine worker', { timeout: 120000 }, async () => {
  const vite = await createServer({
    server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent',
    plugins: [{ name: 'observe-priority-workers', transform(source, id) {
      if (!id.split('?')[0].endsWith('/src/workers/wasmGameWorker.js')) return;
      return `const NativeWorker = globalThis.Worker;
        globalThis.Worker = class extends NativeWorker {
          constructor(url, options) {
            throw new Error('Unexpected auxiliary engine: ' + String(url) + String(options));
          }
        };\n${source}`;
    } }],
  });
  await vite.listen();
  const browser = await webkit.launch();
  try {
    const page = await browser.newPage(devices['iPhone 13 landscape']);
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
    await page.addInitScript(() => {
      window.priorityResults = [];
      const NativeWorker = window.Worker;
      window.Worker = class extends NativeWorker {
        constructor(url, options) {
          super(url, options);
          this.addEventListener('message', ({ data }) => {
            if (data.type === 'priorityAnalysis') window.priorityResults.push(data);
            if (data.type === 'priorityAnalysisError') console.error(data.error);
          });
        }
      };
    });
    const puzzle = { version: 1, players: ['Alice', 'Bob'].map(name => ({ name, life: 20, zones: {
      battlefield: ['Mountain', 'Forest', 'Forest'], hand: ['Lightning Bolt', 'Grizzly Bears'],
      library: Array(10).fill('Mountain'),
    } })) };
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/?puzzle=${Buffer.from(JSON.stringify(puzzle)).toString('base64url')}`);
    const keep = page.getByRole('button', { name: 'Keep hand', exact: true });
    await keep.waitFor({ timeout: 90000 });
    await keep.tap();
    await page.waitForFunction(() => document.querySelector('.mobile-mtga-self-hud .mobile-mtga-hud-name')?.textContent === 'Bob');
    await keep.tap();
    await keep.waitFor({ state: 'hidden' });
    await page.waitForFunction(() => window.priorityResults.some(result => result.decision.analysis_complete));
    assert.equal(await page.locator('.mobile-mtga-self-hud .mobile-mtga-hud-name').textContent(), 'Alice');
    assert.ok(await page.locator('.mobile-decision-primary-button').isEnabled());
    const completed = await page.evaluate(() => window.priorityResults.filter(result => result.decision.analysis_complete).length);
    await page.locator('.mobile-decision-primary-button').tap();
    await page.waitForFunction(count => window.priorityResults.filter(result => result.decision.analysis_complete).length > count, completed);
    assert.equal(await page.getByRole('alert').count(), 0);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});
