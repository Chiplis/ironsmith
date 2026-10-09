import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';
import { fileURLToPath } from 'node:url';
const runtimePath = fileURLToPath(new URL('../../wasm_demo/pkg/ironsmith.js', import.meta.url));

// Unlike the small puzzle fixture, normal startup registers entire catalog
// decks. Stale baked artifacts take the source compiler fallback in the worker,
// where an unoptimized engine can exhaust the smaller browser worker stack.
test('default catalog startup reaches a playable table without poisoning WASM', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch({ args: ['--enable-unsafe-swiftshader'] });
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/`);
    await page.waitForFunction(() => document.querySelector('[data-workspace-shell]')
      || document.body.textContent.includes('Game initialization failed'), null, { timeout: 90000 });
    assert.equal(await page.locator('[data-workspace-shell]').count(), 1,
      (await page.locator('body').innerText()).slice(0, 500));
    await page.waitForSelector('.forge-board canvas');
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});

test('source registration for Endurance fits the browser worker stack', async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0 }, logLevel: 'silent' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/forge-board.html`);
    const result = await page.evaluate(async runtimePath => {
      const source = await (await fetch('/cards/endurance.json')).json();
      // Force the actual current source compiler, independent of artifact age.
      delete source.artifacts;
      const moduleUrl = new URL(`/@fs${runtimePath}`, location.href).href;
      const code = `import init,{WasmGame} from ${JSON.stringify(moduleUrl)};
        onmessage=async({data})=>{try {
          await init({compiler:false,verifier:false});
          const game=new WasmGame();
          const result=game.registerExternalCardSources(data);
          const usable=game.isKnownCardName('Endurance');
          postMessage({result,usable});
        } catch(error) {postMessage({error:String(error)});}};`;
      const url = URL.createObjectURL(new Blob([code], { type: 'text/javascript' }));
      const worker = new Worker(url, { type: 'module' });
      try {
        return await new Promise((resolve, reject) => {
          const timeout = setTimeout(() => reject(new Error('Source-registration worker timed out')), 60000);
          worker.onmessage = event => { clearTimeout(timeout); resolve(event.data); };
          worker.onerror = event => { clearTimeout(timeout); reject(new Error(event.message)); };
          worker.postMessage(source);
        });
      } finally { worker.terminate(); URL.revokeObjectURL(url); }
    }, runtimePath);
    assert.equal(result.error, undefined);
    assert.deepEqual(result.result.failed, []);
    assert.equal(result.usable, true);
  } finally { await browser.close(); await vite.close(); }
});
