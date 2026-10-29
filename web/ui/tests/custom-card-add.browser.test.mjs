import test from 'node:test';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'vite';

test('saved custom mechanics survive Add to Game and repeated battlefield inspection', { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false, watch: null }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/custommagic/pool.json', route => route.fulfill({ json: { cards: [] } }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/tests/custom-card-add-preview.html`);
    await page.getByText('Include custom cards (1 available)', { exact: true }).waitFor();
    await page.getByRole('checkbox', { name: /Include custom cards/ }).check();
    await page.getByRole('combobox', { name: 'Card Name' }).fill('Custom action regression');
    await page.getByRole('option', { name: /Custom action regression/ }).click();
    await page.getByRole('combobox', { name: 'Zone' }).selectOption('battlefield');
    await page.getByRole('checkbox', { name: 'Skip triggers' }).check();
    await page.getByRole('button', { name: 'Add to Game' }).click();
    await page.waitForFunction(() => /Added |Add card failed:/.test(document.querySelector('[role="status"]').textContent));
    assert.match(await page.getByRole('status').innerText(), /^Added Custom action regression/);
    for (let i = 0; i < 3; i++) {
      await page.getByRole('button', { name: 'Inspect Custom action regression' }).click();
      await page.getByRole('button', { name: 'plunder rules helper' }).first().click();
      await page.locator('[data-ui-layer="tooltip"][data-state="open"]').filter({ hasText: 'Create a Treasure token.' }).waitFor();
      await page.keyboard.press('Escape');
      await page.getByRole('button', { name: 'Close inspector' }).click();
    }
    assert.equal(await page.evaluate(() => window.customAddGate.isInFlight()), false);
    assert.deepEqual(errors, []);
  } finally { await browser.close(); await vite.close(); }
});

test('custom mechanic inspection and rejected insertion leave the real worker responsive', { timeout: 120000 }, async () => {
  const vite = await createServer({ server: { host: '127.0.0.1', port: 0, hmr: false, watch: null }, logLevel: 'error' });
  await vite.listen();
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.route('**/custom-worker-test', route => route.fulfill({ contentType: 'text/html', body: '<title>Custom worker regression</title>' }));
    await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/custom-worker-test`);
    const result = await page.evaluate(async () => {
      const { prepareCustomCard } = await import('/src/lib/custom-mechanics.js');
      const { createSnapshotDecoder } = await import('/src/lib/snapshot-channel.js');
      const decoder = createSnapshotDecoder(), pending = new Map();
      const worker = new Worker('/src/workers/wasmGameWorker.js', { type: 'module' });
      let id = 0;
      const ready = new Promise((resolve, reject) => {
        worker.onerror = event => reject(new Error(event.message));
        worker.onmessage = ({ data }) => {
          if (data.type === 'ready') return resolve();
          if (data.type === 'error') return reject(new Error(data.error.message));
          if (data.type !== 'result') return;
          const request = pending.get(data.id);
          if (!request) return;
          pending.delete(data.id);
          clearTimeout(request.timer);
          if (!data.ok) return request.reject(new Error(data.error.message));
          request.resolve(data.snapshot ? decoder.decode(data.snapshot) : data.result);
        };
      });
      const call = (method, ...args) => new Promise((resolve, reject) => {
        const requestId = ++id;
        const timer = setTimeout(() => reject(new Error(`${method} did not respond within 20 seconds`)), 20000);
        pending.set(requestId, { resolve, reject, timer });
        worker.postMessage({ type: 'call', id: requestId, method, args });
      });
      worker.postMessage({ type: 'init', assetBaseUrl: `${location.origin}/` });
      try {
        await ready;
        await call('resetEmpty', ['Alice', 'Bob'], 20);
        const draft = { faces: [{ name: 'Worker custom regression', cardTypes: ['Creature'], manaCost: '{0}', power: '2', toughness: '2',
          oracleText: 'Whenever this creature attacks, plunder.\n{T}: Plunder.',
        }], customMechanics: [{ keyword: 'plunder', reminder: 'Create a Treasure token.' }] };
        let failure;
        try { await call('createCustomCard', { draft, playerIndex: 0, zoneName: 'battlefield', skipTriggers: true }); }
        catch (error) { failure = error.message; }
        const prepared = await prepareCustomCard(draft, input => call('previewCustomCard', input));
        const object = await call('createCustomCard', { draft: prepared.input, playerIndex: 0, zoneName: 'battlefield', skipTriggers: true });
        for (let seat = 0; seat < 2; seat++) {
          for (let card = 0; card < 10; card++) await call('addCardToZone', seat, 'Mountain', 'library', true);
        }
        await call('finishPuzzleSetup');
        let state;
        for (let step = 0; step < 30; step++) {
          state = await call('uiState');
          if (/first.main/i.test(state.phase)) break;
          const action = state.decision?.actions?.find(entry => ['keep_opening_hand', 'continue_pregame', 'begin_game', 'pass_priority'].includes(entry.action_ref?.kind));
          if (!action) throw new Error(`Unexpected decision ${JSON.stringify(state.decision)}`);
          await call('dispatch', { type: 'priority_action', action_ref: action.action_ref });
        }
        const details = await call('objectDetails', object);
        const actions = await call('inspectorActions', object);
        const after = await call('uiState');
        return { failure, text: details.oracle_text, actions, phase: after.phase };
      } finally {
        for (const request of pending.values()) clearTimeout(request.timer);
        worker.terminate();
      }
    });
    assert.match(result.failure, /clause: 'plunder'/);
    assert.match(result.text, /plunder/i);
    assert.ok(Array.isArray(result.actions));
    assert.match(result.phase, /first.main/i);
  } finally { await browser.close(); await vite.close(); }
});
