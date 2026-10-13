import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('a rejected initial snapshot shows its error and tab refresh guidance', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.addInitScript(() => {
      // Exercise the real hook, provider and Shell without downloading WASM.
      // Setup succeeds; the first snapshot fails at the RPC boundary.
      window.Worker = class extends EventTarget {
        postMessage(message) {
          queueMicrotask(() => {
            const data = message.type === 'init'
              ? { type: 'ready' }
              : {
                type: 'result', id: message.id,
                ok: message.method !== 'uiState', result: null,
                error: { message: 'Initial snapshot unavailable' },
              };
            this.dispatchEvent(new MessageEvent('message', { data }));
          });
        }
        terminate() {}
      };
    });
    // Lobby startup uses an empty table, isolating the first state refresh.
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/?lobby=startup-error-test`);
    await page.getByText('Game initialization failed', { exact: true }).waitFor();
    assert.equal(await page.getByText('Initial snapshot unavailable', { exact: true }).count(), 1);
    assert.equal(await page.getByText('Refresh the tab to try again.', { exact: true }).count(), 1);
    assert.equal(await page.getByText('Preparing Game', { exact: true }).count(), 0);
    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
    await vite.close();
  }
});
